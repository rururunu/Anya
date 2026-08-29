use crate::core::ai::provider::ProviderError;
use crate::core::runtime::StreamEvent;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use super::anthropic::read_anthropic_sse_stream;
use super::chat::read_sse_stream;
use super::errors::is_retryable_stream_error;
use super::responses::read_responses_sse_stream;
use super::types::{SseKind, MAX_STREAM_ATTEMPTS, RETRY_BACKOFF, USER_STREAM_INTERRUPTED};

pub(crate) async fn run_chat_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let is_deepseek = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|model| model.trim().to_ascii_lowercase().starts_with("deepseek"));
    run_sse_stream(
        client,
        url,
        api_key,
        body,
        tx,
        SseKind::ChatCompletions { is_deepseek },
    )
    .await
}

pub(crate) async fn run_responses_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let is_deepseek = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|model| model.trim().to_ascii_lowercase().starts_with("deepseek"));
    run_sse_stream(
        client,
        url,
        api_key,
        body,
        tx,
        SseKind::Responses { is_deepseek },
    )
    .await
}

pub(crate) async fn run_anthropic_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    run_sse_stream(client, url, api_key, body, tx, SseKind::AnthropicMessages).await
}

async fn run_sse_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
    kind: SseKind,
) -> Result<(), ProviderError> {
    let mut last_error: Option<ProviderError> = None;

    for attempt in 0..MAX_STREAM_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(RETRY_BACKOFF * attempt).await;
        }

        let response = match post_stream_request(client, url, api_key, body, kind).await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error.clone());
                if attempt + 1 < MAX_STREAM_ATTEMPTS && is_retryable_stream_error(&error) {
                    continue;
                }
                return Err(error);
            }
        };

        let read = match kind {
            SseKind::ChatCompletions { is_deepseek } => {
                read_sse_stream(response, tx, is_deepseek).await
            }
            SseKind::Responses { is_deepseek } => {
                read_responses_sse_stream(response, tx, is_deepseek).await
            }
            SseKind::AnthropicMessages => read_anthropic_sse_stream(response, tx).await,
        };

        match read {
            Ok(outcome) if outcome.is_complete() => {
                let _ = tx.send(StreamEvent::Finish).await;
                return Ok(());
            }
            Ok(outcome) if outcome.emitted => {
                last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                if attempt + 1 < MAX_STREAM_ATTEMPTS {
                    signal_stream_retry(tx, attempt).await;
                    continue;
                }
                return Err(ProviderError::message(USER_STREAM_INTERRUPTED));
            }
            Ok(_) if attempt + 1 < MAX_STREAM_ATTEMPTS => {
                last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                continue;
            }
            Ok(_) => return Err(ProviderError::message(USER_STREAM_INTERRUPTED)),
            Err(error)
                if attempt + 1 < MAX_STREAM_ATTEMPTS && is_retryable_stream_error(&error) =>
            {
                last_error = Some(error.clone());
                if matches!(
                    &error,
                    ProviderError::Message(message) if message == USER_STREAM_INTERRUPTED
                ) {
                    signal_stream_retry(tx, attempt).await;
                }
                continue;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| ProviderError::message(USER_STREAM_INTERRUPTED)))
}

async fn signal_stream_retry(tx: &Sender<StreamEvent>, attempt: u32) {
    let _ = tx
        .send(StreamEvent::Status {
            kind: format!("stream_retry:{}:{}", attempt + 1, MAX_STREAM_ATTEMPTS),
        })
        .await;
}

async fn post_stream_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    kind: SseKind,
) -> Result<reqwest::Response, ProviderError> {
    let mut request = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json");
    if matches!(kind, SseKind::AnthropicMessages) {
        request = request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    }
    let response = request
        .json(body)
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(ProviderError::message(format!(
            "DeepSeek API {status}: {text}"
        )));
    }

    Ok(response)
}
