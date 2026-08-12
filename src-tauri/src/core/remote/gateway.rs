use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};

use crate::app_state::AppState;

use super::protocol::{ClientMessage, ServerMessage};
use super::state::{gateway_status, remote_state, RemoteGatewayState};

const PATH: &str = "/remote/v1";

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
    let callback = |req: &Request, response: Response| {
        if req.uri().path() != PATH {
            return Err(
                Response::builder()
                    .status(404)
                    .body(None)
                    .expect("static 404 response"),
            );
        }
        Ok(response)
    };

    let mut ws = accept_hdr_async(stream, callback)
        .await
        .map_err(|e| e.to_string())?;

    let device_id = authenticate(&mut ws, &state).await?;
    state.mark_connected(device_id.clone(), peer);
    let _ = app.emit(
        "remote-device-connected",
        json!({ "deviceId": device_id, "peer": peer.to_string() }),
    );
    let _ = app.emit("remote-gateway-status", gateway_status(&app));

    // Push current snapshot so the phone paints immediately after hello.
    let snapshot = super::bridge::build_session_snapshot(&app);
    send_msg(
        &mut ws,
        &ServerMessage::Event {
            name: "session.snapshot".into(),
            data: snapshot.as_object().cloned().unwrap_or_default(),
        },
    )
    .await?;

    // Replay still-pending approvals/asks so a late phone connection doesn't miss
    // desktop-triggered problem panels.
    if let Err(e) = replay_pending_interactions(&app, &mut ws).await {
        tracing::debug!(error = %e, "failed to replay pending interactions");
    }

    let mut outbound = super::bridge::subscribe_outbound();

    loop {
        tokio::select! {
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_text(&app, &mut ws, &text).await?;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        ws.send(Message::Pong(payload)).await.map_err(|e| e.to_string())?;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error.to_string()),
                }
            }
            pushed = outbound.recv() => {
                match pushed {
                    Ok(text) => {
                        ws.send(Message::Text(text.into())).await.map_err(|e| e.to_string())?;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(15)) => {
                // Keep-alive for Cloudflare Tunnel / NAT. Do not disconnect solely on
                // "idle" — WS protocol pings are often filtered by intermediate proxies.
                // Companion answers with app-level `pong` so traffic is bidirectional.
                let ping = ServerMessage::Ping { ts: super::state::now_ms() };
                if send_msg(&mut ws, &ping).await.is_err() {
                    break;
                }
            }
        }
    }

    state.mark_disconnected(&device_id);
    let _ = app.emit(
        "remote-device-disconnected",
        json!({ "deviceId": device_id }),
    );
    let _ = app.emit("remote-gateway-status", gateway_status(&app));
    Ok(())
}

async fn authenticate(
    ws: &mut WebSocketStream<TcpStream>,
    state: &RemoteGatewayState,
) -> Result<String, String> {
    let timeout = Duration::from_secs(20);
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let err = ServerMessage::hello_error("timeout", "hello not received");
            let _ = send_msg(ws, &err).await;
            return Err("hello timeout".into());
        }
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .map_err(|_| "hello timeout".to_string())?
            .ok_or_else(|| "connection closed before hello".to_string())?
            .map_err(|e| e.to_string())?;

        let Message::Text(text) = msg else {
            continue;
        };
        let parsed: ClientMessage =
            serde_json::from_str(&text).map_err(|e| format!("invalid hello: {e}"))?;
        let ClientMessage::Hello {
            protocol_version,
            device_id,
            credential,
            ..
        } = parsed
        else {
            let err = ServerMessage::hello_error("expected_hello", "first message must be hello");
            send_msg(ws, &err).await?;
            return Err("expected hello".into());
        };

        if protocol_version != super::protocol::PROTOCOL_VERSION {
            let err = ServerMessage::hello_error(
                "protocol_mismatch",
                format!("server expects {}", super::protocol::PROTOCOL_VERSION),
            );
            send_msg(ws, &err).await?;
            return Err("protocol mismatch".into());
        }

        match state.authorize(&device_id, &credential) {
            Ok(_) => {
                let version = app_version();
                send_msg(ws, &ServerMessage::hello_ok(version)).await?;
                return Ok(device_id);
            }
            Err(message) => {
                send_msg(ws, &ServerMessage::hello_error("unauthorized", message.clone())).await?;
                return Err(message);
            }
        }
    }
}

async fn replay_pending_interactions(
    app: &AppHandle,
    ws: &mut WebSocketStream<TcpStream>,
) -> Result<(), String> {
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

    Ok(())
}

async fn handle_text(
    app: &AppHandle,
    ws: &mut WebSocketStream<TcpStream>,
    text: &str,
) -> Result<(), String> {
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
        ClientMessage::WorkspaceSnapshot { request_id, session_id } => {
            let snapshot = workspace_snapshot_payload(app, session_id.as_deref());
            send_msg(ws, &ServerMessage::rpc_ok(request_id, snapshot)).await
        }
        ClientMessage::WorkspaceReadFile { request_id, .. } => {
            send_msg(
                ws,
                &ServerMessage::rpc_err(request_id, "workspace.readFile not enabled yet"),
            )
            .await
        }
        ClientMessage::WorkspaceFiles {
            request_id,
            session_id,
            workspace_id,
        } => {
            let payload = list_workspace_files_payload(
                app,
                session_id.as_deref(),
                workspace_id.as_deref(),
            )
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
            let compose = super::compose::get(&session_id);
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
            send_msg(ws, &ServerMessage::rpc_ok(request_id, json!({ "models": models }))).await
        }
        ClientMessage::PlanApprove {
            request_id,
            session_id,
        } => {
            let _ = app.emit(
                "remote-plan-approve",
                json!({ "sessionId": session_id }),
            );
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
                Ok(()) => send_msg(ws, &ServerMessage::rpc_ok(request_id, json!({ "ok": true }))).await,
                Err(error) => send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await,
            }
        }
        ClientMessage::ApprovalRespond {
            request_id,
            approval_request_id,
            decision,
        } => {
            let ok = crate::core::tools::tool_approval::shared_tool_approval_store()
                .complete(&approval_request_id, &decision);
            if ok {
                crate::core::remote::bridge::push_interaction_resolved(
                    &approval_request_id,
                    "tool_approval",
                );
                let _ = app.emit(
                    "interaction-resolved",
                    json!({
                        "requestId": approval_request_id,
                        "kind": "tool_approval",
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
                        "tool approval request not found or already completed",
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
            let ok = state
                .core
                .chat()
                .ask_store()
                .complete(&ask_request_id, answer);
            if ok {
                crate::core::remote::bridge::push_interaction_resolved(&ask_request_id, "ask_user");
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
            } else {
                send_msg(
                    ws,
                    &ServerMessage::rpc_err(
                        request_id,
                        "ask request not found or already completed",
                    ),
                )
                .await
            }
        }
    }
}

async fn handle_chat_send(
    app: &AppHandle,
    ws: &mut WebSocketStream<TcpStream>,
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
            json!({ "sessionId": session_id, "messages": mapped })
        }
        Err(_) => json!({ "sessionId": session_id, "messages": [] }),
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
            if !preview.path.is_empty() && (added > 0 || removed > 0 || !preview.unified_diff.is_empty()) {
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
    manager.current()
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
            let icon_url = resolve_remote_icon_url(
                app,
                "skill",
                &skill.name,
                skill.icon_url.as_deref(),
            );
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
            let icon_url = resolve_remote_icon_url(
                app,
                "mcp",
                &server.id,
                server.icon_url.as_deref(),
            );
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
        if url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("data:")
        {
            return Some(url.to_string());
        }
    }
    crate::commands::icons::install_icon_data_url(app, kind, cache_key)
}

fn app_version() -> Option<String> {
    Some(env!("CARGO_PKG_VERSION").to_string())
}

async fn send_msg(
    ws: &mut WebSocketStream<TcpStream>,
    msg: &ServerMessage,
) -> Result<(), String> {
    let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}
