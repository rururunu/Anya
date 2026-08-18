use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::{header, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};

use crate::app_state::AppState;

use super::protocol::{ClientMessage, ServerMessage};
use super::state::{gateway_status, remote_state, RemoteGatewayState};
use super::upload::{MAX_CHUNK_BYTES, MAX_UPLOAD_BYTES};

const PATH: &str = "/remote/v1";

/// 心跳周期：Companion 会以应用层 pong 回应，保证隧道/NAT 上有双向流量。
const PING_INTERVAL: Duration = Duration::from_secs(15);
/// 连续 3 个心跳周期无任何入站帧即视为死连接，主动断开让手机端尽快重连。
const INBOUND_DEADLINE: Duration = Duration::from_secs(45);
/// 单帧发送超时：网络切换时挂死的连接按死连接处理，而不是永久阻塞。
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// 出站消息句柄：把消息排队给专职写任务，可克隆给各 RPC 任务共享。
#[derive(Clone)]
struct Outbound {
    tx: mpsc::Sender<Message>,
}

impl Outbound {
    async fn send(&self, msg: &ServerMessage) -> Result<(), String> {
        let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        self.send_text(text).await
    }

    async fn send_text(&self, text: String) -> Result<(), String> {
        self.send_raw(Message::Text(text.into())).await
    }

    async fn send_raw(&self, msg: Message) -> Result<(), String> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| "connection closed".to_string())
    }

    /// Non-blocking send for keep-alives. A full queue must not stall the read/ping loop.
    fn try_send(&self, msg: &ServerMessage) -> Result<(), String> {
        let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        self.tx
            .try_send(Message::Text(text.into()))
            .map_err(|err| match err {
                TrySendError::Full(_) => "outbound queue full".to_string(),
                TrySendError::Closed(_) => "connection closed".to_string(),
            })
    }
}

pub fn start_gateway(app: AppHandle, port: Option<u16>) -> Result<(), String> {
    let state = remote_state(&app);
    if state.is_running() {
        state.set_enabled_preference(true, port.or(Some(state.port())));
        return Ok(());
    }

    let port = port.unwrap_or_else(|| state.preferred_port());
    state.set_enabled_preference(true, Some(port));
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    state.set_stop_sender(stop_tx);

    let app_for_task = app.clone();
    let state_for_task = state.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_server(app_for_task.clone(), state_for_task, port, stop_rx).await {
            tracing::warn!(error = %error, "remote gateway stopped with error");
        }
        let state = remote_state(&app_for_task);
        state.set_running(false, port);
        let _ = app_for_task.emit("remote-gateway-status", gateway_status(&app_for_task));
    });

    // Give the listener a moment; status flips when bind succeeds inside run_server.
    Ok(())
}

async fn run_server(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    port: u16,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr} failed: {e}"))?;
    state.set_running(true, port);
    let _ = app.emit("remote-gateway-status", gateway_status(&app));
    tracing::info!(%addr, "remote gateway listening");

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                tracing::info!("remote gateway stop requested");
                break;
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        let app = app.clone();
                        let state = state.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = handle_client(app, state, stream, peer).await {
                                tracing::debug!(%peer, error = %error, "remote client closed");
                            }
                        });
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "remote gateway accept failed");
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_client(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    stream: TcpStream,
    peer: SocketAddr,
) -> Result<(), String> {
    let _ = stream.set_nodelay(true);
    super::http_proxy::dispatch(app, state, stream, peer).await
}

pub(super) async fn handle_companion_stream<S>(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    stream: S,
    peer: SocketAddr,
) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let callback = |req: &Request, mut response: Response| {
        if req.uri().path() != PATH {
            return Err(Response::builder()
                .status(404)
                .body(None)
                .expect("static 404 response"));
        }
        // Cloudflare Tunnel may rewrite Accept-Encoding and then try to
        // compress the 101 upgrade, which corrupts the WebSocket stream.
        response.headers_mut().insert(
            header::CONTENT_ENCODING,
            HeaderValue::from_static("identity"),
        );
        Ok(response)
    };

    let mut ws = tokio::time::timeout(Duration::from_secs(10), accept_hdr_async(stream, callback))
        .await
        .map_err(|_| "websocket handshake timed out".to_string())?
        .map_err(|e| e.to_string())?;

    tracing::info!(%peer, "companion websocket accepted; waiting for hello");
    // Immediate origin→edge frame so proxies/DPI don't treat the 101 as a
    // finished HTTP response and RST the stream before hello arrives.
    if let Err(error) = ws.send(Message::Ping(Vec::new().into())).await {
        return Err(format!("failed to send post-upgrade ping: {error}"));
    }

    let device_id = authenticate(&mut ws, &state, peer).await?;
    let mut superseded = state.claim_session(&device_id);
    state.mark_connected(device_id.clone(), peer);
    let _ = app.emit(
        "remote-device-connected",
        json!({ "deviceId": device_id, "peer": peer.to_string() }),
    );
    let _ = app.emit("remote-gateway-status", gateway_status(&app));

    // 读写分离：写端由专职任务持有，所有出站消息（RPC 响应、事件转发、心跳）
    // 经 mpsc 队列串行发送。慢 RPC 不再阻塞读循环，心跳始终准时。
    let (mut sink, mut stream) = ws.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);
    let (writer_stop_tx, mut writer_stop_rx) = oneshot::channel::<()>();
    let writer = tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut writer_stop_rx => break,
                queued = out_rx.recv() => {
                    let Some(msg) = queued else { break };
                    match tokio::time::timeout(SEND_TIMEOUT, sink.send(msg)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            tracing::debug!(error = %error, "remote send failed");
                            break;
                        }
                        Err(_) => {
                            tracing::debug!("remote send timed out; treating connection as dead");
                            break;
                        }
                    }
                }
            }
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), sink.close()).await;
    });
    let out = Outbound { tx: out_tx };

    // Push current snapshot so the phone paints immediately after hello.
    let snapshot = super::bridge::build_session_snapshot(&app);
    let _ = out
        .send(&ServerMessage::Event {
            name: "session.snapshot".into(),
            data: snapshot.as_object().cloned().unwrap_or_default(),
        })
        .await;

    // Authoritative pending list (may be empty) so a reconnecting phone drops
    // approvals that desktop already resolved while the socket was down.
    let _ = out
        .send(&ServerMessage::Event {
            name: "interaction.snapshot".into(),
            data: build_interaction_snapshot(&app)
                .as_object()
                .cloned()
                .unwrap_or_default(),
        })
        .await;

    // Replay still-pending approvals/asks so a late phone connection doesn't miss
    // desktop-triggered problem panels. New clients prefer interaction.snapshot.
    if let Err(e) = replay_pending_interactions(&app, &out).await {
        tracing::debug!(error = %e, "failed to replay pending interactions");
    }

    let mut outbound = super::bridge::subscribe_outbound();
    let mut ping_timer = tokio::time::interval(PING_INTERVAL);
    ping_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_inbound = Instant::now();

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(frame)) => {
                        last_inbound = Instant::now();
                        match frame {
                            Message::Text(text) => {
                                // 每条 RPC 独立任务处理；处理失败只记日志/回错误响应，
                                // 不再拆掉整条连接。
                                let app = app.clone();
                                let out = out.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(error) = handle_text(&app, &out, text.as_str()).await {
                                        tracing::debug!(error = %error, "remote rpc handling failed");
                                    }
                                });
                            }
                            Message::Binary(bytes) => {
                                // Raw upload chunk (no base64). Handle on the
                                // blocking pool and ack back with the RPC id.
                                let payload = bytes.to_vec();
                                let out = out.clone();
                                tauri::async_runtime::spawn(async move {
                                    handle_binary_chunk(payload, out).await;
                                });
                            }
                            Message::Ping(payload) => {
                                if out.send_raw(Message::Pong(payload)).await.is_err() {
                                    break;
                                }
                            }
                            Message::Close(_) => break,
                            _ => {}
                        }
                    }
                    Some(Err(error)) => {
                        tracing::debug!(error = %error, "remote read failed");
                        break;
                    }
                    None => break,
                }
            }
            pushed = outbound.recv() => {
                match pushed {
                    Ok(text) => {
                        if out.send_text(text).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        // 事件积压被丢弃过：补发全量快照，让客户端恢复一致状态。
                        tracing::warn!(skipped, "remote outbound lagged; resending snapshot");
                        let snapshot = super::bridge::build_session_snapshot(&app);
                        let event = ServerMessage::Event {
                            name: "session.snapshot".into(),
                            data: snapshot.as_object().cloned().unwrap_or_default(),
                        };
                        if out.send(&event).await.is_err() {
                            break;
                        }
                        let pending = ServerMessage::Event {
                            name: "interaction.snapshot".into(),
                            data: build_interaction_snapshot(&app)
                                .as_object()
                                .cloned()
                                .unwrap_or_default(),
                        };
                        if out.send(&pending).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = ping_timer.tick() => {
                // Keep-alive for Cloudflare Tunnel / NAT — WS protocol pings are often
                // filtered by intermediate proxies, so we use app-level ping and the
                // Companion answers with app-level pong.
                if last_inbound.elapsed() >= INBOUND_DEADLINE {
                    tracing::debug!("remote connection idle beyond deadline; closing");
                    break;
                }
                let ping = ServerMessage::Ping { ts: super::state::now_ms() };
                match out.try_send(&ping) {
                    Ok(()) => {}
                    Err(error) if error.contains("full") => {
                        tracing::debug!("skipping heartbeat; outbound queue full");
                    }
                    Err(_) => break,
                }
            }
            _ = &mut superseded => {
                tracing::info!(%device_id, "remote session superseded by a new connection");
                break;
            }
        }
    }

    let _ = writer_stop_tx.send(());
    let _ = writer.await;

    let still_current = state
        .connected_peer(&device_id)
        .is_some_and(|current| current == peer);
    if still_current {
        state.mark_disconnected(&device_id, peer);
        let _ = app.emit(
            "remote-device-disconnected",
            json!({ "deviceId": device_id }),
        );
        let _ = app.emit("remote-gateway-status", gateway_status(&app));
    }
    Ok(())
}

async fn authenticate<S>(
    ws: &mut WebSocketStream<S>,
    state: &RemoteGatewayState,
    peer: SocketAddr,
) -> Result<String, String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let timeout = Duration::from_secs(20);
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let err = ServerMessage::hello_error("timeout", "hello not received");
            let _ = send_ws_msg(ws, &err).await;
            return Err("hello timeout".into());
        }
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .map_err(|_| "hello timeout".to_string())?
            .ok_or_else(|| {
                tracing::warn!(%peer, "companion closed before hello");
                "connection closed before hello".to_string()
            })?
            .map_err(|e| {
                tracing::warn!(%peer, error = %e, "companion read failed before hello");
                e.to_string()
            })?;

        match msg {
            Message::Ping(payload) => {
                let _ = ws.send(Message::Pong(payload)).await;
                continue;
            }
            Message::Pong(_) | Message::Frame(_) | Message::Binary(_) => continue,
            Message::Close(_) => {
                return Err("companion sent close before hello".into());
            }
            Message::Text(text) => {
                let parsed: ClientMessage = serde_json::from_str(&text).map_err(|e| {
                    tracing::warn!(%peer, error = %e, "invalid hello json");
                    format!("invalid hello: {e}")
                })?;
                let ClientMessage::Hello {
                    protocol_version,
                    device_id,
                    credential,
                    ..
                } = parsed
                else {
                    let err =
                        ServerMessage::hello_error("expected_hello", "first message must be hello");
                    send_ws_msg(ws, &err).await?;
                    return Err("expected hello".into());
                };

                if protocol_version != super::protocol::PROTOCOL_VERSION {
                    let err = ServerMessage::hello_error(
                        "protocol_mismatch",
                        format!("server expects {}", super::protocol::PROTOCOL_VERSION),
                    );
                    send_ws_msg(ws, &err).await?;
                    return Err("protocol mismatch".into());
                }

                match state.authorize(&device_id, &credential) {
                    Ok(_) => {
                        tracing::info!(%peer, %device_id, "companion hello accepted");
                        let version = app_version();
                        send_ws_msg(ws, &ServerMessage::hello_ok(version)).await?;
                        return Ok(device_id);
                    }
                    Err(message) => {
                        send_ws_msg(
                            ws,
                            &ServerMessage::hello_error("unauthorized", message.clone()),
                        )
                        .await?;
                        return Err(message);
                    }
                }
            }
        }
    }
}

fn ask_title(questions: &[crate::core::tools::context::AskQuestion]) -> String {
    questions
        .first()
        .map(|q| {
            if q.header.trim().is_empty() {
                q.question.clone()
            } else {
                q.header.clone()
            }
        })
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "需要回答".into())
}

fn build_interaction_snapshot(app: &AppHandle) -> serde_json::Value {
    let mut pending = Vec::new();
    if let Some(state) = app.try_state::<AppState>() {
        for item in state.core.chat().ask_store().pending_items() {
            pending.push(json!({
                "kind": "ask_user",
                "sessionId": item.session_id,
                "requestId": item.request_id,
                "title": ask_title(&item.questions),
                "questions": item.questions,
            }));
        }
        for item in state.core.chat().path_permission_store().pending_items() {
            pending.push(json!({
                "kind": "path_permission",
                "sessionId": item.session_id,
                "requestId": item.request_id,
                "toolName": item.tool_name,
                "title": format!("路径权限 · {}", item.tool_name),
                "preview": item.path,
                "operation": item.operation,
            }));
        }
    }
    for item in crate::core::tools::tool_approval::shared_tool_approval_store().pending_items() {
        pending.push(json!({
            "kind": "tool_approval",
            "sessionId": item.session_id,
            "requestId": item.request_id,
            "toolName": item.tool_name,
            "title": item.title,
        }));
    }
    for session_id in crate::core::tools::plan_mode::shared_plan_mode_store().active_session_ids() {
        if super::bridge::run_state_for(&session_id) == super::bridge::RemoteRunState::Streaming {
            continue;
        }
        if !crate::core::tools::plan_mode::shared_plan_mode_store()
            .is_awaiting_approval(&session_id)
        {
            continue;
        }
        pending.push(json!({
            "kind": "plan_approval",
            "sessionId": session_id,
            "requestId": format!("plan-{session_id}"),
            "title": "计划待批准",
        }));
    }
    json!({ "pending": pending })
}

async fn replay_pending_interactions(app: &AppHandle, ws: &Outbound) -> Result<(), String> {
    let Some(state) = app.try_state::<crate::app_state::AppState>() else {
        return Ok(());
    };

    // `ask_user` requests (includes question list/options).
    for pending in state.core.chat().ask_store().pending_items() {
        let title = pending
            .questions
            .first()
            .map(|q| {
                if q.header.trim().is_empty() {
                    q.question.clone()
                } else {
                    q.header.clone()
                }
            })
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "需要回答".into());

        send_msg(
            ws,
            &ServerMessage::Event {
                name: "ask-user".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "title": title,
                    "questions": pending.questions,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    // Tool approvals (only title/toolName are used by the current mobile UI,
    // but we keep the full record in the pending snapshot).
    for pending in crate::core::tools::tool_approval::shared_tool_approval_store().pending_items() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "tool-approval".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "toolName": pending.tool_name,
                    "title": pending.title,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    for pending in state.core.chat().path_permission_store().pending_items() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "path.permission".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "toolName": pending.tool_name,
                    "title": format!("路径权限 · {}", pending.tool_name),
                    "preview": pending.path,
                    "operation": pending.operation,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    for session_id in crate::core::tools::plan_mode::shared_plan_mode_store().active_session_ids() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "session.planMode".into(),
                data: json!({
                    "sessionId": session_id,
                    "active": true,
                    "source": "manual",
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    Ok(())
}

/// Phone `session.compose.get`: ask the desktop UI to mirror Pinia into the
/// gateway store, then fall back to app settings if the session was never opened.
async fn resolve_session_compose(
    app: &AppHandle,
    session_id: &str,
) -> super::compose::SessionCompose {
    let _ = app.emit("remote-compose-needed", json!({ "sessionId": session_id }));
    tokio::time::sleep(Duration::from_millis(80)).await;
    let mut compose = super::compose::get(session_id);
    if compose.chat_model.trim().is_empty() {
        for _ in 0..16 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            compose = super::compose::get(session_id);
            if !compose.chat_model.trim().is_empty() {
                break;
            }
        }
    }
    if compose.chat_model.trim().is_empty() {
        if let Some(state) = app.try_state::<crate::services::settings_store::SettingsState>() {
            if let Ok(settings) = state.settings.lock() {
                compose.chat_model = settings.chat_model.clone();
                compose.chat_model_provider = settings.chat_model_provider.clone();
                compose.chat_mode = settings.chat_mode;
                compose.tool_approval_mode = settings.tool_approval_mode;
            }
        }
        if !compose.chat_model.trim().is_empty() {
            super::compose::set(session_id, compose.clone());
        }
    }
    compose
}

/// Binary upload-chunk frame: `[requestId:36][uploadId:36][offset:8][data]`.
/// Writes the chunk out-of-order and replies with the matching RPC ack.
async fn handle_binary_chunk(payload: Vec<u8>, out: Outbound) {
    const HEADER_LEN: usize = 36 + 36 + 8;
    if payload.len() < HEADER_LEN {
        tracing::debug!(len = payload.len(), "binary chunk frame too short");
        return;
    }
    let request_id = match std::str::from_utf8(&payload[..36]) {
        Ok(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let upload_id = match std::str::from_utf8(&payload[36..72]) {
        Ok(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let offset = u64::from_be_bytes(payload[72..80].try_into().unwrap_or([0u8; 8]));
    let data = payload[HEADER_LEN..].to_vec();

    let (request_id, upload_id) = (request_id, upload_id);
    let result = tauri::async_runtime::spawn_blocking(move || {
        super::upload::chunk_bytes(&upload_id, offset, &data)
    })
    .await;

    match result {
        Ok(Ok(chunk_result)) => {
            if out
                .send(&ServerMessage::rpc_ok(request_id, chunk_result))
                .await
                .is_err()
            {
                tracing::debug!("failed to ack binary chunk");
            }
        }
        Ok(Err(message)) => {
            let _ = out
                .send(&ServerMessage::rpc_err(request_id, &message))
                .await;
        }
        Err(error) => {
            let _ = out
                .send(&ServerMessage::rpc_err(
                    request_id,
                    &format!("chunk task failed: {error}"),
                ))
                .await;
        }
    }
}

async fn handle_text(app: &AppHandle, ws: &Outbound, text: &str) -> Result<(), String> {
    let parsed: ClientMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(error) => {
            tracing::debug!(error = %error, "drop undecodable remote frame");
            return Ok(());
        }
    };

    match parsed {
        ClientMessage::Hello { .. } => {
            // Already authenticated; ignore duplicate hellos.
            Ok(())
        }
        ClientMessage::Pong { .. } => Ok(()),
        ClientMessage::SessionList { request_id } => {
            let sessions = super::bridge::build_session_snapshot(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, sessions)).await
        }
        ClientMessage::SessionHistory {
            request_id,
            session_id,
        } => {
            let data = session_history(app, &session_id);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, data)).await
        }
        ClientMessage::SessionDelete {
            request_id,
            session_id,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            let bound = state
                .core
                .chat()
                .conversation()
                .workspace_for_session(&session_id);
            let workspace_root = bound.and_then(|id| {
                state
                    .core
                    .workspaces()
                    .list()
                    .into_iter()
                    .find(|w| w.id == id)
                    .map(|w| w.root)
            });
            super::upload::cleanup_session_uploads(app, &session_id, workspace_root.as_deref());
            state.core.chat().conversation().delete_session(&session_id);
            // Same event the desktop settings page emits after deletes — the
            // workbench listener refreshes its list and leaves dead sessions.
            let _ = app.emit("history-updated", json!({ "sessionId": session_id }));
            // All companion clients repaint their list from the fresh snapshot.
            let snapshot = super::bridge::build_session_snapshot(app);
            super::bridge::broadcast_server_message(&ServerMessage::Event {
                name: "session.snapshot".into(),
                data: snapshot.as_object().cloned().unwrap_or_default(),
            });
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true, "sessionId": session_id })),
            )
            .await
        }
        ClientMessage::WorkspaceSnapshot {
            request_id,
            session_id,
        } => {
            let snapshot = workspace_snapshot_payload(app, session_id.as_deref());
            send_msg(ws, &ServerMessage::rpc_ok(request_id, snapshot)).await
        }
        ClientMessage::WorkspaceReadFile {
            request_id,
            path,
            max_bytes,
            session_id,
            workspace_id,
            mode,
            offset,
            length,
        } => {
            let result = read_workspace_file_payload(
                app,
                session_id.as_deref(),
                workspace_id.as_deref(),
                &path,
                max_bytes,
                mode.as_deref().unwrap_or("text"),
                offset,
                length,
            )
            .await;
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::WorkspaceFiles {
            request_id,
            session_id,
            workspace_id,
        } => {
            let payload =
                list_workspace_files_payload(app, session_id.as_deref(), workspace_id.as_deref())
                    .await;
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::SkillsList { request_id } => {
            let payload = list_skills_payload(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::McpList { request_id } => {
            let payload = list_mcp_payload(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::FileUploadBegin {
            request_id,
            session_id,
            workspace_id,
            file_name,
            size,
            mime: _,
        } => {
            let result = super::upload::begin(
                app,
                session_id.as_deref(),
                workspace_id.as_deref(),
                &file_name,
                size,
            );
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::FileUploadChunk {
            request_id,
            upload_id,
            offset,
            data_base64,
        } => match super::upload::chunk(&upload_id, offset, &data_base64) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileUploadFinish {
            request_id,
            upload_id,
        } => match super::upload::finish(&upload_id) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileUploadAbort {
            request_id,
            upload_id,
        } => match super::upload::abort(&upload_id) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileDownloadBegin {
            request_id,
            path,
            session_id,
            workspace_id,
        } => {
            let result =
                begin_file_download(app, session_id.as_deref(), workspace_id.as_deref(), &path);
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::ChatSend {
            request_id,
            session_id,
            message,
            workspace_id,
            chat_mode,
            tool_approval_mode,
            chat_model,
            chat_model_provider,
        } => {
            handle_chat_send(
                app,
                ws,
                request_id,
                session_id,
                message,
                workspace_id,
                chat_mode,
                tool_approval_mode,
                chat_model,
                chat_model_provider,
            )
            .await
        }
        ClientMessage::SessionComposeGet {
            request_id,
            session_id,
        } => {
            let compose = resolve_session_compose(&app, &session_id).await;
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({ "sessionId": session_id, "compose": compose }),
                ),
            )
            .await
        }
        ClientMessage::SessionComposeSet {
            request_id,
            session_id,
            chat_mode,
            tool_approval_mode,
            chat_model,
            chat_model_provider,
            chat_model_label,
        } => {
            let patch = super::compose::SessionComposePatch {
                chat_mode: parse_chat_mode(chat_mode.as_deref()),
                tool_approval_mode: parse_approval_mode(tool_approval_mode.as_deref()),
                chat_model,
                chat_model_provider,
                chat_model_label,
            };
            let compose = super::compose::patch(&session_id, &patch);
            // Notify desktop UI so Pinia stays in sync with the phone.
            let _ = app.emit(
                "remote-compose-changed",
                json!({
                    "sessionId": session_id,
                    "compose": compose,
                    "source": "companion",
                }),
            );
            super::bridge::broadcast_server_message(&ServerMessage::Event {
                name: "session.compose".into(),
                data: super::compose::event_payload(&session_id, &compose),
            });
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({ "sessionId": session_id, "compose": compose }),
                ),
            )
            .await
        }
        ClientMessage::ModelsList { request_id } => {
            let models = list_remote_models(app).await;
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "models": models })),
            )
            .await
        }
        ClientMessage::PlanApprove {
            request_id,
            session_id,
        } => {
            let _ = app.emit("remote-plan-approve", json!({ "sessionId": session_id }));
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true, "sessionId": session_id })),
            )
            .await
        }
        ClientMessage::ChatCancel {
            request_id,
            message_id,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            match state.core.chat().cancel(&message_id) {
                Ok(()) => {
                    send_msg(
                        ws,
                        &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
                    )
                    .await
                }
                Err(error) => {
                    send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await
                }
            }
        }
        ClientMessage::ApprovalRespond {
            request_id,
            approval_request_id,
            decision,
        } => {
            let tool_session = crate::core::tools::tool_approval::shared_tool_approval_store()
                .complete(&approval_request_id, &decision);
            // Path permission uses allow_always (not allow_session); map phone
            // "本会话允许" onto the path-store grant.
            let path_decision = match decision.as_str() {
                "allow_session" => "allow_always",
                other => other,
            };
            let path_session = if tool_session.is_none() {
                let Some(state) = app.try_state::<AppState>() else {
                    return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready"))
                        .await;
                };
                state
                    .core
                    .chat()
                    .path_permission_store()
                    .complete(&approval_request_id, path_decision)
            } else {
                None
            };
            if let Some((kind, session_id)) = tool_session
                .map(|sid| ("tool_approval", sid))
                .or_else(|| path_session.map(|sid| ("path_permission", sid)))
            {
                crate::core::remote::bridge::push_interaction_resolved(
                    &approval_request_id,
                    kind,
                    Some(&session_id),
                );
                crate::core::remote::bridge::resume_run_state_after_interaction(app, &session_id);
                crate::commands::window::dismiss_tracked_interaction_notifications(
                    app,
                    Some(&approval_request_id),
                    None,
                );
                let _ = app.emit(
                    "interaction-resolved",
                    json!({
                        "requestId": approval_request_id,
                        "kind": kind,
                    }),
                );
                send_msg(
                    ws,
                    &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
                )
                .await
            } else {
                send_msg(
                    ws,
                    &ServerMessage::rpc_err(
                        request_id,
                        "approval request not found or already completed",
                    ),
                )
                .await
            }
        }
        ClientMessage::AskRespond {
            request_id,
            ask_request_id,
            answer,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            let Some(session_id) = state
                .core
                .chat()
                .ask_store()
                .complete(&ask_request_id, answer)
            else {
                return send_msg(
                    ws,
                    &ServerMessage::rpc_err(
                        request_id,
                        "ask request not found or already completed",
                    ),
                )
                .await;
            };
            crate::core::remote::bridge::push_interaction_resolved(
                &ask_request_id,
                "ask_user",
                Some(&session_id),
            );
            crate::core::remote::bridge::resume_run_state_after_interaction(app, &session_id);
            crate::commands::window::dismiss_tracked_interaction_notifications(
                app,
                Some(&ask_request_id),
                None,
            );
            let _ = app.emit(
                "interaction-resolved",
                json!({
                    "requestId": ask_request_id,
                    "kind": "ask_user",
                }),
            );
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
            )
            .await
        }
    }
}

async fn handle_chat_send(
    app: &AppHandle,
    ws: &Outbound,
    request_id: String,
    session_id: Option<String>,
    message: String,
    workspace_id: Option<String>,
    chat_mode: Option<String>,
    tool_approval_mode: Option<String>,
    chat_model: Option<String>,
    chat_model_provider: Option<String>,
) -> Result<(), String> {
    let Some(state) = app.try_state::<AppState>() else {
        return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
    };
    let settings = match crate::services::settings_store::get_settings(app) {
        Ok(settings) => settings,
        Err(error) => {
            return send_msg(ws, &ServerMessage::rpc_err(request_id, error)).await;
        }
    };
    let preferences = crate::core::chat::SendPreferences::from(&settings);
    // Companion 新建聊天传 sessionId=null，期望桌面分配新会话。
    // ChatService::send 对 None 会落到固定 "default"，导致手机「新建」消息并入旧对话。
    // 远程路径在此 mint 新 id，与桌面 createConversation 的 session-${Date.now()} 对齐。
    let session_id = match session_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(id) => Some(id),
        None => Some(format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        )),
    };
    let sid = session_id.clone().unwrap_or_default();
    if !sid.is_empty() {
        let patch = super::compose::SessionComposePatch {
            chat_mode: parse_chat_mode(chat_mode.as_deref()),
            tool_approval_mode: parse_approval_mode(tool_approval_mode.as_deref()),
            chat_model: chat_model.clone(),
            chat_model_provider: chat_model_provider.clone(),
            chat_model_label: None,
        };
        if patch.chat_mode.is_some()
            || patch.tool_approval_mode.is_some()
            || patch.chat_model.is_some()
            || patch.chat_model_provider.is_some()
        {
            let _ = super::compose::patch(&sid, &patch);
        }
    }
    let compose = if sid.is_empty() {
        super::compose::SessionCompose::default()
    } else {
        super::compose::get(&sid)
    };
    let overrides = crate::models::chat::ChatSendOverrides {
        model_id: if compose.chat_model.is_empty() {
            None
        } else {
            Some(compose.chat_model.clone())
        },
        model_provider: if compose.chat_model_provider.is_empty() {
            None
        } else {
            Some(compose.chat_model_provider.clone())
        },
        chat_mode: Some(compose.chat_mode),
        tool_approval_mode: Some(compose.tool_approval_mode),
        skip_auto_plan: false,
        resume_plan: false,
    };
    // Unbound companion sends (FAB 随文) are treated as quick-ask inside
    // ChatService::send so they do not inherit the desktop selected workspace.
    // Workspace-folder "+" still passes workspaceId and stays bound.
    let quick_ask = false;
    match state
        .core
        .chat()
        .send(
            session_id,
            message,
            preferences,
            workspace_id,
            quick_ask,
            overrides,
            crate::core::chat::session_origin::RequestOrigin::Companion,
        )
        .await
    {
        Ok(result) => {
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({
                        "sessionId": result.session_id,
                        "userMessageId": result.user_message_id,
                        "assistantMessageId": result.assistant_message_id,
                        "agentRunId": result.agent_run_id,
                    }),
                ),
            )
            .await
        }
        Err(error) => send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await,
    }
}

fn parse_chat_mode(raw: Option<&str>) -> Option<crate::models::settings::ChatMode> {
    match raw? {
        "ask" => Some(crate::models::settings::ChatMode::Ask),
        "agent" => Some(crate::models::settings::ChatMode::Agent),
        "plan" => Some(crate::models::settings::ChatMode::Plan),
        _ => None,
    }
}

fn parse_approval_mode(raw: Option<&str>) -> Option<crate::models::settings::ToolApprovalMode> {
    match raw? {
        "ask" => Some(crate::models::settings::ToolApprovalMode::Ask),
        "auto" => Some(crate::models::settings::ToolApprovalMode::Auto),
        "alwaysAllow" => Some(crate::models::settings::ToolApprovalMode::AlwaysAllow),
        _ => None,
    }
}

async fn list_remote_models(app: &AppHandle) -> Vec<serde_json::Value> {
    match crate::commands::chat::list_chat_models(app.clone()).await {
        Ok(models) => models
            .into_iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "provider": m.provider,
                    "displayName": m.display_name,
                    "ownedBy": m.owned_by,
                })
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn session_history(app: &AppHandle, session_id: &str) -> serde_json::Value {
    let Some(state) = app.try_state::<AppState>() else {
        return json!({ "sessionId": session_id, "messages": [] });
    };
    match state.core.chat().history(session_id) {
        Ok(messages) => {
            let mapped: Vec<serde_json::Value> = messages
                .into_iter()
                .filter(|message| {
                    !matches!(
                        message.role,
                        crate::core::runtime::Role::Tool | crate::core::runtime::Role::System
                    )
                })
                .map(remote_chat_message)
                .collect();
            json!({
                "sessionId": session_id,
                "messages": mapped,
                "planModeActive": crate::core::tools::plan_mode::shared_plan_mode_store()
                    .is_active(session_id),
            })
        }
        Err(_) => json!({
            "sessionId": session_id,
            "messages": [],
            "planModeActive": crate::core::tools::plan_mode::shared_plan_mode_store()
                .is_active(session_id),
        }),
    }
}

fn remote_chat_message(message: crate::core::runtime::ChatMessage) -> serde_json::Value {
    use crate::core::runtime::{MessageStatus, Role};
    let role = match message.role {
        Role::User => "User",
        Role::Assistant => "Assistant",
        Role::System => "System",
        Role::Tool => "System",
    };
    let status = match message.status {
        MessageStatus::Pending => "Pending",
        MessageStatus::Streaming => "Streaming",
        MessageStatus::Done => "Complete",
        MessageStatus::Error => "Error",
        MessageStatus::Cancelled => "Cancelled",
    };
    let code_changes = extract_code_changes(&message);
    let plan_tasks = extract_plan_tasks(&message);
    let tool_activities = message
        .tool_activities
        .as_ref()
        .map(|activities| {
            activities
                .iter()
                .map(|activity| {
                    json!({
                        "id": activity.id,
                        "subagentId": activity.subagent_id,
                        "parentActivityId": activity.parent_activity_id,
                        "toolName": activity.tool_name,
                        "title": activity.title,
                        "kind": activity.kind,
                        "detail": activity.detail,
                        "arguments": activity.arguments,
                        "result": activity.result,
                        "preview": activity.preview.as_ref().map(|preview| json!({
                            "path": preview.path,
                            "unifiedDiff": preview.unified_diff,
                            "affectedPaths": preview.affected_paths,
                        })),
                        "success": activity.success,
                        "status": activity.status,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "id": message.id,
        "sessionId": message.session_id,
        "role": role,
        "content": message.content,
        "reasoning": message.reasoning,
        "status": status,
        "createdAtEpochMs": message.timestamp,
        "codeChanges": code_changes,
        "planTasks": plan_tasks,
        "toolActivities": tool_activities,
    })
}

fn extract_code_changes(message: &crate::core::runtime::ChatMessage) -> Vec<serde_json::Value> {
    let Some(activities) = message.tool_activities.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for activity in activities {
        if activity.status != "done" || !activity.success {
            continue;
        }
        if let Some(preview) = activity.preview.as_ref() {
            let (added, removed) = count_diff_lines(&preview.unified_diff);
            if !preview.path.is_empty()
                && (added > 0 || removed > 0 || !preview.unified_diff.is_empty())
            {
                out.push(json!({
                    "id": format!("{}:{}", message.id, activity.id),
                    "path": preview.path,
                    "added": added,
                    "removed": removed,
                }));
            }
            for path in &preview.affected_paths {
                if path != &preview.path && !path.is_empty() {
                    out.push(json!({
                        "id": format!("{}:{}:{}", message.id, activity.id, path),
                        "path": path,
                        "added": 0,
                        "removed": 0,
                    }));
                }
            }
        } else if let Some(args) = activity.arguments.as_ref() {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                if matches!(
                    activity.tool_name.as_str(),
                    "write_file" | "replace_in_file" | "replace_many_in_file" | "apply_patch"
                ) {
                    out.push(json!({
                        "id": format!("{}:{}", message.id, activity.id),
                        "path": path,
                        "added": 0,
                        "removed": 0,
                    }));
                }
            }
        }
    }
    out
}

fn extract_plan_tasks(message: &crate::core::runtime::ChatMessage) -> Vec<serde_json::Value> {
    let Some(activities) = message.tool_activities.as_ref() else {
        return Vec::new();
    };
    for activity in activities.iter().rev() {
        if !matches!(activity.tool_name.as_str(), "update_tasks" | "todo_write") {
            continue;
        }
        let Some(args) = activity.arguments.as_ref() else {
            continue;
        };
        let Some(tasks) = args.get("tasks").and_then(|v| v.as_array()) else {
            continue;
        };
        return tasks
            .iter()
            .filter_map(|task| {
                let content = task.get("content")?.as_str()?.to_string();
                let status = task
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string();
                let level = task.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(json!({
                    "content": content,
                    "status": status,
                    "level": level,
                }))
            })
            .collect();
    }
    Vec::new()
}

fn count_diff_lines(diff: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
        }
    }
    (added, removed)
}

fn workspace_snapshot_payload(app: &AppHandle, session_id: Option<&str>) -> serde_json::Value {
    let Some(workspace) = resolve_workspace(app, session_id, None) else {
        return json!({
            "workspaceId": null,
            "name": null,
            "rootPath": null,
            "sessionId": session_id,
            "runState": "idle",
            "changedFiles": []
        });
    };
    json!({
        "workspaceId": workspace.id,
        "name": workspace.name,
        "rootPath": workspace.root.to_string_lossy(),
        "sessionId": session_id,
        "runState": "idle",
        "changedFiles": []
    })
}

fn resolve_workspace(
    app: &AppHandle,
    session_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Option<crate::core::workspace::Workspace> {
    let state = app.try_state::<AppState>()?;
    let manager = state.core.workspaces();
    let list = manager.list();
    if let Some(id) = workspace_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(found) = list.into_iter().find(|w| w.id == id) {
            return Some(found);
        }
    }
    if let Some(sid) = session_id.map(str::trim).filter(|s| !s.is_empty()) {
        let snapshot = super::bridge::build_session_snapshot(app);
        if let Some(sessions) = snapshot.get("sessions").and_then(|v| v.as_array()) {
            if let Some(ws_id) = sessions
                .iter()
                .find(|item| item.get("id").and_then(|v| v.as_str()) == Some(sid))
                .and_then(|item| item.get("workspaceId"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let list = manager.list();
                if let Some(found) = list.into_iter().find(|w| w.id == ws_id) {
                    return Some(found);
                }
            }
        }
    }
    manager
        .current()
        .or_else(|| manager.list().into_iter().next())
}

fn guess_mime(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("txt") | Some("log") | Some("gitignore") => "text/plain",
        Some("md") | Some("markdown") => "text/markdown",
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("yaml") | Some("yml") => "application/yaml",
        Some("toml") => "application/toml",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz") | Some("tgz") => "application/gzip",
        Some("7z") => "application/x-7z-compressed",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    }
}

/// Resolve `rel_path` inside the workspace root, rejecting escapes
/// (absolute paths, `..`, symlinks pointing outside the root).
fn resolve_in_workspace(
    root: &std::path::Path,
    rel_path: &str,
) -> Result<std::path::PathBuf, String> {
    let rel = rel_path.trim().trim_start_matches(['/', '\\']);
    if rel.is_empty() {
        return Err("empty path".into());
    }
    // Join segment-by-segment so `.anya/shared/x` works on every platform
    // (a single `join("a/b")` can leave mixed separators on Windows).
    let mut candidate = root.to_path_buf();
    for part in rel.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("path escapes workspace root".into());
        }
        candidate.push(part);
    }
    let canonical_root =
        std::fs::canonicalize(root).map_err(|e| format!("workspace root unavailable: {e}"))?;
    let canonical =
        std::fs::canonicalize(&candidate).map_err(|_| format!("file not found: {rel}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("path escapes workspace root".into());
    }
    Ok(canonical)
}

fn begin_file_download(
    app: &AppHandle,
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    rel_path: &str,
) -> Result<serde_json::Value, String> {
    let workspace = resolve_workspace(app, session_id, workspace_id)
        .ok_or_else(|| "No workspace selected".to_string())?;
    let file = resolve_in_workspace(&workspace.root, rel_path)?;
    let meta = std::fs::metadata(&file).map_err(|e| format!("stat failed: {e}"))?;
    if !meta.is_file() {
        return Err("not a file".into());
    }
    let size = meta.len();
    if size > MAX_UPLOAD_BYTES {
        return Err(format!(
            "file too large for download: {size} bytes (max {MAX_UPLOAD_BYTES})"
        ));
    }
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.trim_start_matches(['/', '\\']).to_string());
    let mime = guess_mime(&file).to_string();
    let id = super::download::mint(file, name.clone(), mime.clone(), size)?;
    let url = super::download::public_download_url(app, &id);
    Ok(json!({
        "downloadId": id,
        "url": url,
        "size": size,
        "name": name,
        "mime": mime,
    }))
}

async fn read_workspace_file_payload(
    app: &AppHandle,
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    rel_path: &str,
    max_bytes: i32,
    mode: &str,
    offset: Option<u64>,
    length: Option<u64>,
) -> Result<serde_json::Value, String> {
    let workspace = resolve_workspace(app, session_id, workspace_id)
        .ok_or_else(|| "No workspace selected".to_string())?;
    let file = resolve_in_workspace(&workspace.root, rel_path)?;
    let rel = rel_path.trim().trim_start_matches(['/', '\\']).to_string();
    let download = mode.eq_ignore_ascii_case("download");
    let max_text_bytes = max_bytes.clamp(1, 2_000_000) as u64;
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::{Read, Seek, SeekFrom};

        let meta = std::fs::metadata(&file).map_err(|e| format!("stat failed: {e}"))?;
        if !meta.is_file() {
            return Err("not a file".into());
        }
        let size = meta.len();
        if download {
            if size > MAX_UPLOAD_BYTES {
                return Err(format!(
                    "file too large for download: {size} bytes (max {MAX_UPLOAD_BYTES})"
                ));
            }
            use base64::engine::general_purpose::STANDARD as B64;
            use base64::Engine as _;
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());
            let mime = guess_mime(&file);
            let start = offset.unwrap_or(0);
            if start > size {
                return Err(format!("offset {start} past end of file ({size})"));
            }
            let want = length
                .unwrap_or(MAX_CHUNK_BYTES as u64)
                .min(MAX_CHUNK_BYTES as u64)
                .min(size.saturating_sub(start));
            let mut buf = vec![0u8; want as usize];
            let n = if want == 0 {
                0
            } else {
                let mut f = std::fs::File::open(&file).map_err(|e| format!("read failed: {e}"))?;
                f.seek(SeekFrom::Start(start))
                    .map_err(|e| format!("seek failed: {e}"))?;
                f.read(&mut buf).map_err(|e| format!("read failed: {e}"))?
            };
            buf.truncate(n);
            let next = start + n as u64;
            Ok(json!({
                "path": rel,
                "name": name,
                "size": size,
                "mime": mime,
                "offset": start,
                "length": n,
                "eof": next >= size,
                "contentBase64": B64.encode(&buf),
            }))
        } else {
            let bytes = std::fs::read(&file).map_err(|e| format!("read failed: {e}"))?;
            let truncated = bytes.len() as u64 > max_text_bytes;
            let slice = if truncated {
                &bytes[..max_text_bytes as usize]
            } else {
                &bytes[..]
            };
            Ok(json!({
                "path": rel,
                "content": String::from_utf8_lossy(slice),
                "truncated": truncated,
                "size": size,
            }))
        }
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

fn should_descend_workspace_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git" | ".svn" | ".hg" | "node_modules" | ".next" | ".nuxt" | "target" | "dist" | "build"
    )
}

async fn list_workspace_files_payload(
    app: &AppHandle,
    session_id: Option<&str>,
    workspace_id: Option<&str>,
) -> serde_json::Value {
    let Some(workspace) = resolve_workspace(app, session_id, workspace_id) else {
        return json!({
            "workspaceId": null,
            "name": null,
            "rootPath": null,
            "files": [],
            "error": "No workspace selected"
        });
    };
    let root = workspace.root.clone();
    let files = tauri::async_runtime::spawn_blocking(move || {
        let mut files = walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(should_descend_workspace_entry)
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&root)
                    .ok()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
            })
            .take(5_000)
            .collect::<Vec<_>>();
        files.sort_unstable_by_key(|path| path.to_lowercase());
        files
    })
    .await
    .unwrap_or_default();

    json!({
        "workspaceId": workspace.id,
        "name": workspace.name,
        "rootPath": workspace.root.to_string_lossy(),
        "files": files,
    })
}

fn list_skills_payload(app: &AppHandle) -> serde_json::Value {
    let enabled_builtins: std::collections::HashSet<String> =
        crate::services::settings_store::get_settings(app)
            .ok()
            .map(|settings| {
                settings
                    .enabled_builtin_skills
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();
    let skills = crate::core::tools::skills::list_skill_infos()
        .unwrap_or_default()
        .into_iter()
        .filter(|skill| skill.source != "builtin" || enabled_builtins.contains(&skill.name))
        .map(|skill| {
            let icon_url =
                resolve_remote_icon_url(app, "skill", &skill.name, skill.icon_url.as_deref());
            json!({
                "id": skill.name,
                "name": skill.name,
                "title": skill.title,
                "description": skill.description,
                "source": skill.source,
                "iconUrl": icon_url,
            })
        })
        .collect::<Vec<_>>();
    json!({ "skills": skills })
}

fn list_mcp_payload(app: &AppHandle) -> serde_json::Value {
    let servers = crate::services::settings_store::get_settings(app)
        .ok()
        .map(|settings| settings.mcp_servers)
        .unwrap_or_default()
        .into_iter()
        .filter(|server| server.enabled)
        .map(|server| {
            let icon_url =
                resolve_remote_icon_url(app, "mcp", &server.id, server.icon_url.as_deref());
            json!({
                "id": server.id,
                "title": server.title.unwrap_or_else(|| server.id.clone()),
                "description": server.description.unwrap_or_default(),
                "qualifiedName": server.qualified_name,
                "iconUrl": icon_url,
            })
        })
        .collect::<Vec<_>>();
    json!({ "mcpServers": servers })
}

/// Prefer an http(s)/data icon URL for the phone companion; fall back to disk cache.
fn resolve_remote_icon_url(
    app: &AppHandle,
    kind: &str,
    cache_key: &str,
    remote: Option<&str>,
) -> Option<String> {
    if let Some(url) = remote.map(str::trim).filter(|u| !u.is_empty()) {
        if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("data:") {
            return Some(url.to_string());
        }
    }
    crate::commands::icons::install_icon_data_url(app, kind, cache_key)
}

fn app_version() -> Option<String> {
    Some(env!("CARGO_PKG_VERSION").to_string())
}

/// 认证阶段（读写分离前）直接写 WS；之后统一走 `Outbound` 队列。
async fn send_ws_msg<S>(ws: &mut WebSocketStream<S>, msg: &ServerMessage) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}

async fn send_msg(out: &Outbound, msg: &ServerMessage) -> Result<(), String> {
    out.send(msg).await
}
