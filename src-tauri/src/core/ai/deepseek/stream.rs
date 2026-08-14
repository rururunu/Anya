use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use crate::core::runtime::{StreamEvent, ToolCallPayload};
use crate::core::token::TokenUsage;

use super::messages::non_empty_option;
use super::ProviderError;

pub(crate) const RETRY_BACKOFF: Duration = Duration::from_millis(500);

const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_STREAM_ATTEMPTS: u32 = 5;

pub(super) const USER_STREAM_INTERRUPTED: &str = "Connection interrupted, please retry";
const USER_STREAM_STALLED: &str = "Response timed out, please retry";

#[derive(Debug, Deserialize)]
struct ApiStreamResponse {
    #[serde(default)]
    choices: Vec<ApiStreamChoice>,
    #[serde(default)]
    usage: Option<ApiTokenUsage>,
}

#[derive(Debug, Deserialize)]
struct ApiTokenUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    #[serde(default)]
    prompt_tokens_details: ApiPromptTokensDetails,
    #[serde(default)]
    completion_tokens_details: ApiCompletionTokensDetails,
}

#[derive(Debug, Default, Deserialize)]
struct ApiPromptTokensDetails {
    #[serde(default)]
    cached_tokens: usize,
}

#[derive(Debug, Default, Deserialize)]
struct ApiCompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct ApiStreamChoice {
    delta: ApiStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ApiToolCallDelta>>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    #[serde(default)]
    function: Option<ApiToolCallFunction>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiToolCallFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Default)]
struct ToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
pub(super) struct StreamReadOutcome {
    pub(super) saw_done: bool,
    emitted: bool,
    finish_reason: Option<String>,
}

impl StreamReadOutcome {
    pub(super) fn is_complete(&self) -> bool {
        self.saw_done || self.finish_reason.is_some()
    }
}

#[cfg(test)]
impl StreamReadOutcome {
    pub(crate) fn test_with(saw_done: bool, finish_reason: Option<String>) -> Self {
        Self {
            emitted: false,
            saw_done,
            finish_reason,
        }
    }
}

pub(super) async fn run_chat_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    // DeepSeek-only behaviors (cache-hit token accounting, etc.) are keyed off
    // the wire model so custom OpenAI-compatible providers are untouched.
    let is_deepseek = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|model| model.trim().to_ascii_lowercase().starts_with("deepseek"));
    let mut last_error: Option<ProviderError> = None;

    for attempt in 0..MAX_STREAM_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(RETRY_BACKOFF * attempt).await;
        }

        let response = match post_stream_request(client, url, api_key, body).await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error.clone());
                if attempt + 1 < MAX_STREAM_ATTEMPTS && is_retryable_stream_error(&error) {
                    continue;
                }
                return Err(error);
            }
        };

        match read_sse_stream(response, tx, is_deepseek).await {
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
) -> Result<reqwest::Response, ProviderError> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
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

async fn read_sse_stream(
    response: reqwest::Response,
    tx: &Sender<StreamEvent>,
    is_deepseek: bool,
) -> Result<StreamReadOutcome, ProviderError> {
    let mut stream = response.bytes_stream();
    let mut pending_utf8 = Vec::new();
    let mut buffer = String::new();
    let mut outcome = StreamReadOutcome::default();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tool_calls: HashMap<usize, ToolCallBuilder> = HashMap::new();

    loop {
        let chunk = match tokio::time::timeout(STREAM_IDLE_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(chunk))) => chunk,
            Ok(Some(Err(error))) => {
                return Err(map_read_error(error.to_string(), outcome.emitted));
            }
            Ok(None) => break,
            Err(_) => return Err(ProviderError::message(USER_STREAM_STALLED)),
        };

        crate::runtime::encoding::append_utf8_chunk(&mut pending_utf8, &chunk, &mut buffer);

        while let Some(line) = next_sse_line(&mut buffer) {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };

            let payload = payload.trim();
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                outcome.saw_done = true;
                break;
            }

            let parsed: ApiStreamResponse = serde_json::from_str(payload).map_err(|error| {
                ProviderError::message(format!("invalid stream payload: {error}"))
            })?;

            if let Some(usage) = parsed.usage {
                // DeepSeek's prompt_tokens INCLUDES cache reads; subtract them so
                // `inputTokens` reflects actual (non-cached) input.
                let (cache_read, reasoning) = if is_deepseek {
                    (
                        usage.prompt_tokens_details.cached_tokens,
                        usage.completion_tokens_details.reasoning_tokens,
                    )
                } else {
                    (0, 0)
                };
                let input = usage.prompt_tokens.saturating_sub(cache_read);
                let _ = tx
                    .send(StreamEvent::Usage(TokenUsage::exact_with_breakdown(
                        input,
                        usage.completion_tokens,
                        "provider/usage",
                        (is_deepseek && cache_read > 0).then_some(cache_read),
                        (is_deepseek && reasoning > 0).then_some(reasoning),
                    )))
                    .await;
            }

            for choice in parsed.choices {
                if let Some(reason) = choice.finish_reason.filter(|value| !value.is_empty()) {
                    outcome.finish_reason = Some(reason);
                }
                if let Some(reasoning_chunk) = choice.delta.reasoning_content.as_deref() {
                    if !reasoning_chunk.is_empty() {
                        outcome.emitted = true;
                        reasoning.push_str(reasoning_chunk);
                        let _ = tx
                            .send(StreamEvent::Reasoning(reasoning_chunk.to_string()))
                            .await;
                    }
                }
                if let Some(content_chunk) = choice.delta.content.as_deref() {
                    if !content_chunk.is_empty() {
                        outcome.emitted = true;
                        content.push_str(content_chunk);
                        let _ = tx.send(StreamEvent::Delta(content_chunk.to_string())).await;
                    }
                }
                if let Some(calls) = choice.delta.tool_calls {
                    for call in calls {
                        let index = call.index.unwrap_or(0);
                        let entry = tool_calls.entry(index).or_default();
                        if let Some(id) = call.id {
                            entry.id = id;
                        }
                        if let Some(function) = call.function {
                            if let Some(name) = function.name {
                                entry.name = name;
                            }
                            if let Some(args) = function.arguments {
                                entry.arguments.push_str(&args);
                            }
                        }
                        outcome.emitted = true;
                    }
                }
            }

            if outcome.saw_done {
                break;
            }
        }

        if outcome.saw_done {
            break;
        }
    }

    if !outcome.is_complete() {
        return Err(ProviderError::message(USER_STREAM_INTERRUPTED));
    }

    let mut merged_calls: Vec<_> = tool_calls.into_iter().collect();
    merged_calls.sort_by_key(|(index, _)| *index);
    let tool_call_payloads: Vec<ToolCallPayload> = merged_calls
        .into_iter()
        .map(|(index, builder)| ToolCallPayload {
            id: if builder.id.trim().is_empty() {
                format!("call-{index}")
            } else {
                builder.id
            },
            name: builder.name,
            arguments: builder.arguments,
            thought_signature: None,
        })
        .collect();

    let _ = tx
        .send(StreamEvent::TurnComplete {
            content,
            reasoning: non_empty_option(reasoning),
            tool_calls: tool_call_payloads,
            finish_reason: outcome.finish_reason.clone(),
        })
        .await;

    Ok(outcome)
}

pub(super) async fn emit_stream_error(
    tx: &Sender<StreamEvent>,
    error: ProviderError,
) -> Result<(), ProviderError> {
    let message = user_facing_stream_error(&error);
    let _ = tx.send(StreamEvent::Error(message.clone())).await;
    Err(ProviderError::message(message))
}

pub(super) fn user_facing_stream_error(error: &ProviderError) -> String {
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

fn is_retryable_stream_error(error: &ProviderError) -> bool {
    match error {
        ProviderError::Cancelled => false,
        ProviderError::Message(message) => {
            if message.starts_with("DeepSeek API") {
                return false;
            }
            if message.contains("API Key") {
                return false;
            }
            message == USER_STREAM_INTERRUPTED
                || message == USER_STREAM_STALLED
                || is_connection_error(message)
        }
    }
}

fn map_read_error(message: String, emitted: bool) -> ProviderError {
    if emitted {
        ProviderError::message(USER_STREAM_INTERRUPTED)
    } else if is_connection_error(&message) {
        ProviderError::message(format!("network error: {message}"))
    } else {
        ProviderError::message(message)
    }
}

fn next_sse_line(buffer: &mut String) -> Option<String> {
    let newline_index = buffer.find('\n')?;
    let mut line = buffer.drain(..=newline_index).collect::<String>();
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Some(line)
}

#[cfg(test)]
mod utf8_stream_tests {
    use crate::runtime::encoding::append_utf8_chunk;

    #[test]
    fn sse_buffer_survives_split_multibyte_utf8() {
        // Simulate JSON SSE payload containing CJK split across TCP chunks.
        let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n";
        let bytes = payload.as_bytes();
        let mut pending = Vec::new();
        let mut buffer = String::new();
        for window in bytes.chunks(3) {
            append_utf8_chunk(&mut pending, window, &mut buffer);
        }
        assert!(pending.is_empty());
        assert!(buffer.contains("你好"));
        assert!(!buffer.contains('\u{FFFD}'));
    }
}
