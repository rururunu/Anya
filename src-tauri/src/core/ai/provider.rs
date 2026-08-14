use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc::Sender;

use crate::core::runtime::{ChatRequest, StreamEvent};

#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    #[error("provider request cancelled")]
    Cancelled,
    #[error("{0}")]
    Message(String),
}

impl ProviderError {
    pub fn message(value: impl Into<String>) -> Self {
        Self::Message(value.into())
    }

    pub fn cancelled() -> Self {
        Self::Cancelled
    }

    /// DeepSeek/OpenAI-style "maximum context length" rejection. The agent loop
    /// uses this to trigger a mid-turn compaction and retry instead of failing.
    pub fn is_context_window_exceeded(&self) -> bool {
        match self {
            ProviderError::Message(message) => {
                let lower = message.to_ascii_lowercase();
                (lower.contains("context") && lower.contains("length"))
                    || lower.contains("maximum context")
                    || lower.contains("context window")
            }
            ProviderError::Cancelled => false,
        }
    }
}

impl From<String> for ProviderError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

/// AI Provider 抽象 — 仅 `stream()` 接口。
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn id(&self) -> &'static str;

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;
}
