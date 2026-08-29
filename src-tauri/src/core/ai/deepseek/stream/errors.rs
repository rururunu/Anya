use crate::core::ai::provider::ProviderError;
use crate::core::runtime::StreamEvent;
use tokio::sync::mpsc::Sender;

use super::types::{USER_STREAM_INTERRUPTED, USER_STREAM_STALLED};

pub(crate) async fn emit_stream_error(
    tx: &Sender<StreamEvent>,
    error: ProviderError,
) -> Result<(), ProviderError> {
    let message = user_facing_stream_error(&error);
    let _ = tx.send(StreamEvent::Error(message.clone())).await;
    Err(ProviderError::message(message))
}

pub(crate) fn user_facing_stream_error(error: &ProviderError) -> String {
    match error {
        ProviderError::Cancelled => "Request cancelled".to_string(),
        ProviderError::Message(message) => {
            if message.starts_with("DeepSeek API") {
                return message.clone();
            }
            if message.contains("API Key") {
                return message.clone();
            }
            if message.contains("multimodal")
                || message.contains("Multimodal")
                || message.contains("image analysis")
                || message.contains("vision")
                || message.contains("Vision")
            {
                return message.clone();
            }
            if message.contains("invalid stream payload") {
                return USER_STREAM_INTERRUPTED.to_string();
            }
            if message == USER_STREAM_STALLED || message == USER_STREAM_INTERRUPTED {
                return message.clone();
            }
            if is_connection_error(message) {
                return USER_STREAM_INTERRUPTED.to_string();
            }
            message.clone()
        }
    }
}

fn is_connection_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    [
        "connection reset",
        "connection refused",
        "broken pipe",
        "unexpected eof",
        "incomplete",
        "stalled",
        "timed out",
        "network error",
        "error sending request",
        "error decoding response body",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(super) fn is_retryable_stream_error(error: &ProviderError) -> bool {
    match error {
        ProviderError::Cancelled => false,
        ProviderError::Message(message) => {
            if message.contains("API Key") {
                return false;
            }
            if let Some(status) = deepseek_http_status(message) {
                return matches!(status, 429 | 500 | 502 | 503 | 504);
            }
            message == USER_STREAM_INTERRUPTED
                || message == USER_STREAM_STALLED
                || is_connection_error(message)
        }
    }
}

pub(super) fn deepseek_http_status(message: &str) -> Option<u16> {
    let rest = message.strip_prefix("DeepSeek API ")?;
    rest.split_whitespace().next()?.parse().ok()
}

pub(super) fn map_read_error(message: String, emitted: bool) -> ProviderError {
    if emitted {
        ProviderError::message(USER_STREAM_INTERRUPTED)
    } else if is_connection_error(&message) {
        ProviderError::message(format!("network error: {message}"))
    } else {
        ProviderError::message(message)
    }
}
