use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::{header, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_hdr_async;

use super::auth::authenticate;
use super::constants::{INBOUND_DEADLINE, PATH, PING_INTERVAL, SEND_TIMEOUT};
use super::interactions::{build_interaction_snapshot, replay_pending_interactions};
use super::outbound::Outbound;
use super::rpc::{handle_binary_chunk, handle_text};
use crate::core::remote::protocol::ServerMessage;
use crate::core::remote::state::{gateway_status, RemoteGatewayState};

pub async fn handle_companion_stream<S>(
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

    let snapshot = crate::core::remote::bridge::build_session_snapshot(&app);
    let _ = out
        .send(&ServerMessage::Event {
            name: "session.snapshot".into(),
            data: snapshot.as_object().cloned().unwrap_or_default(),
        })
        .await;

    let _ = out
        .send(&ServerMessage::Event {
            name: "interaction.snapshot".into(),
            data: build_interaction_snapshot(&app)
                .as_object()
                .cloned()
                .unwrap_or_default(),
        })
        .await;

    if let Err(e) = replay_pending_interactions(&app, &out).await {
        tracing::debug!(error = %e, "failed to replay pending interactions");
    }

    let mut outbound = crate::core::remote::bridge::subscribe_outbound();
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
                                let app = app.clone();
                                let out = out.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(error) = handle_text(&app, &out, text.as_str()).await {
                                        tracing::debug!(error = %error, "remote rpc handling failed");
                                    }
                                });
                            }
                            Message::Binary(bytes) => {
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
                        tracing::warn!(skipped, "remote outbound lagged; resending snapshot");
                        let snapshot = crate::core::remote::bridge::build_session_snapshot(&app);
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
                if last_inbound.elapsed() >= INBOUND_DEADLINE {
                    tracing::debug!("remote connection idle beyond deadline; closing");
                    break;
                }
                let ping = ServerMessage::Ping { ts: crate::core::remote::state::now_ms() };
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
