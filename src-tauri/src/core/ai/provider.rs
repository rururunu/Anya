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

/// Spawned provider work stays owned by its caller when cancellation or a
/// timeout drops the collection future.
pub(crate) struct ProviderTaskGuard(
    pub tauri::async_runtime::JoinHandle<Result<(), ProviderError>>,
);

impl Drop for ProviderTaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
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

    /// Model identity, not the transport implementation, selects the dsh contract.
    fn uses_dsh_tools(&self) -> bool {
        false
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;
}
