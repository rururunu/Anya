use std::net::SocketAddr;
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::core::remote::protocol::{ClientMessage, ServerMessage};
use super::send::{app_version, send_ws_msg};
use crate::core::remote::state::RemoteGatewayState;

pub(super) async fn authenticate<S>(
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

                if protocol_version != crate::core::remote::protocol::PROTOCOL_VERSION {
                    let err = ServerMessage::hello_error(
                        "protocol_mismatch",
                        format!("server expects {}", crate::core::remote::protocol::PROTOCOL_VERSION),
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
