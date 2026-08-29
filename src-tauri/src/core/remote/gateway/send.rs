use futures_util::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::outbound::Outbound;
use crate::core::remote::protocol::ServerMessage;

pub(super) fn app_version() -> Option<String> {
    Some(env!("CARGO_PKG_VERSION").to_string())
}

/// 认证阶段（读写分离前）直接写 WS；之后统一走 `Outbound` 队列。
pub(super) async fn send_ws_msg<S>(ws: &mut WebSocketStream<S>, msg: &ServerMessage) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn send_msg(out: &Outbound, msg: &ServerMessage) -> Result<(), String> {
    out.send(msg).await
}
