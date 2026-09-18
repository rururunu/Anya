use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ChatError {
    #[error("message cannot be empty")]
    EmptyMessage,
    #[error("message not found")]
    MessageNotFound,
    /// A turn is still active and the client did not request soft-inject.
    /// Staged / queue-continue sends must wait for ChatFinished instead of
    /// being appended onto the in-flight (or cancelling) agent.
    #[error("turn already in progress")]
    TurnInProgress,
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    Internal(String),
}

impl From<crate::core::ai::ProviderError> for ChatError {
    fn from(error: crate::core::ai::ProviderError) -> Self {
        match error {
            crate::core::ai::ProviderError::Cancelled => {
                Self::Provider("request cancelled".to_string())
            }
            crate::core::ai::ProviderError::Message(message) => Self::Provider(message),
        }
    }
}
