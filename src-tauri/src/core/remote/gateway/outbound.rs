use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio_tungstenite::tungstenite::Message;

use crate::core::remote::protocol::ServerMessage;

#[derive(Clone)]
pub(in crate::core::remote::gateway) struct Outbound {
    pub(in crate::core::remote::gateway) tx: mpsc::Sender<Message>,
}

impl Outbound {
    pub(in crate::core::remote::gateway) async fn send(&self, msg: &ServerMessage) -> Result<(), String> {
        let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        self.send_text(text).await
    }

    pub(in crate::core::remote::gateway) async fn send_text(&self, text: String) -> Result<(), String> {
        self.send_raw(Message::Text(text.into())).await
    }

    pub(in crate::core::remote::gateway) async fn send_raw(&self, msg: Message) -> Result<(), String> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| "connection closed".to_string())
    }

    /// Non-blocking send for keep-alives. A full queue must not stall the read/ping loop.
    pub(in crate::core::remote::gateway) fn try_send(&self, msg: &ServerMessage) -> Result<(), String> {
        let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        self.tx
            .try_send(Message::Text(text.into()))
            .map_err(|err| match err {
                TrySendError::Full(_) => "outbound queue full".to_string(),
                TrySendError::Closed(_) => "connection closed".to_string(),
            })
    }
}
