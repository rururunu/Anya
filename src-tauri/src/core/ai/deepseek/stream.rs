use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
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
    /// First-party DeepSeek field. Prefer this when present.
    #[serde(default)]
    prompt_cache_hit_tokens: usize,
    #[serde(default)]
    prompt_tokens_details: ApiPromptTokensDetails,
    #[serde(default)]
    completion_tokens_details: ApiCompletionTokensDetails,
}

impl ApiTokenUsage {
    fn cache_read_tokens(&self) -> usize {
        self.prompt_cache_hit_tokens
            .max(self.prompt_tokens_details.cached_tokens)
    }
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
    reasoning: Option<String>,
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

impl ToolCallBuilder {
    /// Later SSE deltas often repeat `name: ""`. Never let that wipe a name
    /// we already collected. If the provider streams the name in growing
    /// prefixes, keep the longest value.
    fn set_name(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return;
        }
        if self.name.is_empty() || name.starts_with(&self.name) || !self.name.starts_with(name) {
            self.name = name.to_string();
        }
    }
}

#[cfg(test)]
mod tool_call_builder_tests {
    use super::ToolCallBuilder;

    #[test]
    fn empty_name_delta_does_not_wipe_existing_name() {
        let mut builder = ToolCallBuilder::default();
        builder.set_name("web_search");
        builder.set_name("");
        builder.set_name("   ");
        assert_eq!(builder.name, "web_search");
    }

    #[test]
    fn growing_name_prefix_is_kept() {
        let mut builder = ToolCallBuilder::default();
        builder.set_name("mcp__");
        builder.set_name("mcp__puppeteer__navigate");
        assert_eq!(builder.name, "mcp__puppeteer__navigate");
        builder.set_name("mcp__");
        assert_eq!(builder.name, "mcp__puppeteer__navigate");
    }
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

#[derive(Clone, Copy)]
enum SseKind {
    ChatCompletions { is_deepseek: bool },
    Responses { is_deepseek: bool },
    AnthropicMessages,
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

pub(super) async fn run_responses_stream(
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

pub(super) async fn run_anthropic_stream(
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
                        usage.cache_read_tokens(),
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
                        is_deepseek.then_some(cache_read),
                        (is_deepseek && reasoning > 0).then_some(reasoning),
                    )))
                    .await;
            }

            for choice in parsed.choices {
                if let Some(reason) = choice.finish_reason.filter(|value| !value.is_empty()) {
                    outcome.finish_reason = Some(reason);
                }
                if let Some(reasoning_chunk) = chat_reasoning_delta(&choice.delta) {
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
                                entry.set_name(name);
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

fn chat_reasoning_delta(delta: &ApiStreamDelta) -> Option<&str> {
    delta
        .reasoning_content
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| delta.reasoning.as_deref().filter(|value| !value.is_empty()))
}

async fn read_responses_sse_stream(
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
    let mut sse_event = String::new();

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
            if let Some(name) = line.strip_prefix("event:") {
                sse_event = name.trim().to_string();
                continue;
            }
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

            let mut parsed: Value = serde_json::from_str(payload).map_err(|error| {
                ProviderError::message(format!("invalid stream payload: {error}"))
            })?;
            if parsed.get("type").and_then(Value::as_str).is_none() && !sse_event.is_empty() {
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert("type".into(), json!(sse_event.clone()));
                }
            }

            let tick =
                apply_responses_event(&parsed, &mut content, &mut reasoning, &mut tool_calls, &mut outcome);
            if let Some(message) = tick.error {
                return Err(ProviderError::message(message));
            }
            if let Some((input, output, cache_read, reasoning_tokens)) = tick.usage {
                let cache = cache_read.unwrap_or(0);
                let billed_input = if is_deepseek {
                    input.saturating_sub(cache)
                } else {
                    input
                };
                let _ = tx
                    .send(StreamEvent::Usage(TokenUsage::exact_with_breakdown(
                        billed_input,
                        output,
                        "provider/usage",
                        if is_deepseek {
                            Some(cache)
                        } else {
                            cache_read.filter(|value| *value > 0)
                        },
                        reasoning_tokens.filter(|value| *value > 0),
                    )))
                    .await;
            }
            if !tick.reasoning_delta.is_empty() {
                let _ = tx
                    .send(StreamEvent::Reasoning(tick.reasoning_delta))
                    .await;
            }
            if !tick.content_delta.is_empty() {
                let _ = tx.send(StreamEvent::Delta(tick.content_delta)).await;
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

    if outcome.finish_reason.is_none() && !tool_call_payloads.is_empty() {
        outcome.finish_reason = Some("tool_calls".into());
    }

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

async fn read_anthropic_sse_stream(
    response: reqwest::Response,
    tx: &Sender<StreamEvent>,
) -> Result<StreamReadOutcome, ProviderError> {
    let mut stream = response.bytes_stream();
    let mut pending_utf8 = Vec::new();
    let mut buffer = String::new();
    let mut outcome = StreamReadOutcome::default();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tool_calls: HashMap<usize, ToolCallBuilder> = HashMap::new();
    let mut sse_event = String::new();
    let mut block_kind: HashMap<usize, String> = HashMap::new();
    let mut tool_index_for_block: HashMap<usize, usize> = HashMap::new();
    let mut next_tool = 0usize;

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
            if let Some(name) = line.strip_prefix("event:") {
                sse_event = name.trim().to_string();
                continue;
            }
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

            let mut parsed: Value = serde_json::from_str(payload).map_err(|error| {
                ProviderError::message(format!("invalid stream payload: {error}"))
            })?;
            if parsed.get("type").and_then(Value::as_str).is_none() && !sse_event.is_empty() {
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert("type".into(), json!(sse_event.clone()));
                }
            }

            let tick = apply_anthropic_event(
                &parsed,
                &mut content,
                &mut reasoning,
                &mut tool_calls,
                &mut block_kind,
                &mut tool_index_for_block,
                &mut next_tool,
                &mut outcome,
            );
            if let Some(message) = tick.error {
                return Err(ProviderError::message(message));
            }
            if let Some((input, output, _, _)) = tick.usage {
                let _ = tx
                    .send(StreamEvent::Usage(TokenUsage::exact_with_breakdown(
                        input,
                        output,
                        "provider/usage",
                        None,
                        None,
                    )))
                    .await;
            }
            if !tick.reasoning_delta.is_empty() {
                let _ = tx
                    .send(StreamEvent::Reasoning(tick.reasoning_delta))
                    .await;
            }
            if !tick.content_delta.is_empty() {
                let _ = tx.send(StreamEvent::Delta(tick.content_delta)).await;
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

    if outcome.finish_reason.is_none() && !tool_call_payloads.is_empty() {
        outcome.finish_reason = Some("tool_calls".into());
    }

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

fn apply_anthropic_event(
    event: &Value,
    content: &mut String,
    reasoning: &mut String,
    tool_calls: &mut HashMap<usize, ToolCallBuilder>,
    block_kind: &mut HashMap<usize, String>,
    tool_index_for_block: &mut HashMap<usize, usize>,
    next_tool: &mut usize,
    outcome: &mut StreamReadOutcome,
) -> ResponsesTick {
    let mut tick = ResponsesTick::default();
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
    match event_type {
        "error" => {
            tick.error = Some(
                event
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .or_else(|| event.get("message").and_then(Value::as_str))
                    .unwrap_or("Anthropic API request failed")
                    .to_string(),
            );
        }
        "message_start" => {
            if let Some(input) = event
                .pointer("/message/usage/input_tokens")
                .and_then(Value::as_u64)
            {
                tick.usage = Some((input as usize, 0, None, None));
            }
        }
        "content_block_start" => {
            let index = json_index(event, "index");
            let block = event.get("content_block").unwrap_or(event);
            let kind = block
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            block_kind.insert(index, kind.clone());
            if kind == "tool_use" {
                let tool_idx = *next_tool;
                *next_tool += 1;
                tool_index_for_block.insert(index, tool_idx);
                let entry = tool_calls.entry(tool_idx).or_default();
                if let Some(id) = block.get("id").and_then(Value::as_str) {
                    entry.id = id.to_string();
                }
                if let Some(name) = block.get("name").and_then(Value::as_str) {
                    entry.set_name(name);
                }
                if let Some(input) = block.get("input") {
                    if !input.is_null() && input != &json!({}) {
                        entry.arguments = input.to_string();
                    }
                }
                outcome.emitted = true;
            }
        }
        "content_block_delta" => {
            let index = json_index(event, "index");
            let delta = event.get("delta").unwrap_or(event);
            let delta_type = delta.get("type").and_then(Value::as_str).unwrap_or("");
            match delta_type {
                "text_delta" => {
                    append_delta(
                        delta.get("text").and_then(Value::as_str),
                        content,
                        &mut tick.content_delta,
                        outcome,
                    );
                }
                "thinking_delta" => {
                    append_delta(
                        delta
                            .get("thinking")
                            .and_then(Value::as_str)
                            .or_else(|| delta.get("text").and_then(Value::as_str)),
                        reasoning,
                        &mut tick.reasoning_delta,
                        outcome,
                    );
                }
                "input_json_delta" => {
                    if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                        if !partial.is_empty() {
                            let tool_idx = tool_index_for_block
                                .get(&index)
                                .copied()
                                .unwrap_or_else(|| {
                                    let kind = block_kind.get(&index).map(String::as_str);
                                    if kind == Some("tool_use") {
                                        index
                                    } else {
                                        0
                                    }
                                });
                            tool_calls
                                .entry(tool_idx)
                                .or_default()
                                .arguments
                                .push_str(partial);
                            outcome.emitted = true;
                        }
                    }
                }
                _ => {
                    if block_kind.get(&index).map(String::as_str) == Some("thinking") {
                        append_delta(
                            delta.get("thinking").and_then(Value::as_str),
                            reasoning,
                            &mut tick.reasoning_delta,
                            outcome,
                        );
                    } else {
                        append_delta(
                            delta.get("text").and_then(Value::as_str),
                            content,
                            &mut tick.content_delta,
                            outcome,
                        );
                    }
                }
            }
        }
        "message_delta" => {
            if let Some(reason) = event
                .pointer("/delta/stop_reason")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                outcome.finish_reason = Some(if reason == "tool_use" {
                    "tool_calls".into()
                } else {
                    reason.to_string()
                });
            }
            let input = event
                .pointer("/usage/input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if let Some(output) = event
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64)
            {
                tick.usage = Some((input, output as usize, None, None));
            }
        }
        "message_stop" => {
            outcome.saw_done = true;
        }
        _ => {}
    }
    tick
}

#[derive(Default)]
struct ResponsesTick {
    content_delta: String,
    reasoning_delta: String,
    error: Option<String>,
    usage: Option<(usize, usize, Option<usize>, Option<usize>)>,
}

fn apply_responses_event(
    event: &Value,
    content: &mut String,
    reasoning: &mut String,
    tool_calls: &mut HashMap<usize, ToolCallBuilder>,
    outcome: &mut StreamReadOutcome,
) -> ResponsesTick {
    let mut tick = ResponsesTick::default();
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");

    match event_type {
        "response.output_text.delta" => {
            append_delta(
                event_text_delta(event).as_deref(),
                content,
                &mut tick.content_delta,
                outcome,
            );
        }
        "response.reasoning_text.delta"
        | "response.reasoning_summary_text.delta"
        | "response.reasoning_summary_part.added" => {
            append_delta(
                event_text_delta(event).as_deref(),
                reasoning,
                &mut tick.reasoning_delta,
                outcome,
            );
        }
        "response.output_item.added" | "response.output_item.done" => {
            let reasoning_at = reasoning.len();
            let content_at = content.len();
            apply_responses_output_item(event, content, reasoning, tool_calls, outcome);
            if reasoning.len() > reasoning_at {
                tick.reasoning_delta = reasoning[reasoning_at..].to_string();
            }
            if content.len() > content_at {
                tick.content_delta = content[content_at..].to_string();
            }
        }
        "response.function_call_arguments.delta" => {
            let index = json_index(event, "output_index");
            if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                if !delta.is_empty() {
                    tool_calls.entry(index).or_default().arguments.push_str(delta);
                    outcome.emitted = true;
                }
            }
        }
        "response.function_call_arguments.done" => {
            let index = json_index(event, "output_index");
            if let Some(args) = event.get("arguments").and_then(Value::as_str) {
                tool_calls.entry(index).or_default().arguments = args.to_string();
                outcome.emitted = true;
            }
        }
        "response.completed" | "response.done" | "response.incomplete" => {
            outcome.saw_done = true;
            if let Some(response) = event.get("response") {
                tick.usage = responses_usage(response);
                if content.is_empty() {
                    if let Some(text) = collect_responses_message_text(response) {
                        content.push_str(&text);
                        tick.content_delta = text;
                        outcome.emitted = true;
                    }
                }
                if reasoning.is_empty() {
                    if let Some(text) = collect_responses_reasoning_text(response) {
                        reasoning.push_str(&text);
                        tick.reasoning_delta = text;
                        outcome.emitted = true;
                    }
                }
                collect_responses_tool_calls(response, tool_calls, outcome);
            }
            if outcome.finish_reason.is_none() {
                outcome.finish_reason = Some(if tool_calls.is_empty() {
                    "stop".into()
                } else {
                    "tool_calls".into()
                });
            }
        }
        "response.failed" | "error" => {
            let message = event
                .get("response")
                .and_then(|response| response.get("error"))
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .or_else(|| event.get("message").and_then(Value::as_str))
                .unwrap_or("Responses API request failed");
            tick.error = Some(message.to_string());
        }
        _ => {}
    }

    tick
}

fn apply_responses_output_item(
    event: &Value,
    content: &mut String,
    reasoning: &mut String,
    tool_calls: &mut HashMap<usize, ToolCallBuilder>,
    outcome: &mut StreamReadOutcome,
) {
    let Some(item) = event.get("item") else {
        return;
    };
    let index = json_index(event, "output_index");
    match item.get("type").and_then(Value::as_str).unwrap_or("") {
        "function_call" => {
            let entry = tool_calls.entry(index).or_default();
            if let Some(id) = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                entry.id = id.to_string();
            }
            if let Some(name) = item
                .get("name")
                .and_then(Value::as_str)
            {
                entry.set_name(name);
            }
            if let Some(args) = item.get("arguments").and_then(Value::as_str) {
                if !args.is_empty() {
                    entry.arguments = args.to_string();
                }
            }
            outcome.emitted = true;
        }
        "reasoning" if reasoning.is_empty() => {
            if let Some(text) = collect_item_reasoning(item) {
                reasoning.push_str(&text);
                outcome.emitted = true;
            }
        }
        "message" if content.is_empty() => {
            if let Some(text) = collect_item_message_text(item) {
                content.push_str(&text);
                outcome.emitted = true;
            }
        }
        _ => {}
    }
}

fn append_delta(
    delta: Option<&str>,
    sink: &mut String,
    tick_delta: &mut String,
    outcome: &mut StreamReadOutcome,
) {
    let Some(delta) = delta.filter(|value| !value.is_empty()) else {
        return;
    };
    sink.push_str(delta);
    tick_delta.push_str(delta);
    outcome.emitted = true;
}

fn event_text_delta(event: &Value) -> Option<String> {
    if let Some(text) = event.get("delta").and_then(json_text) {
        return Some(text);
    }
    if let Some(text) = event.get("text").and_then(json_text) {
        return Some(text);
    }
    event
        .get("part")
        .and_then(|part| part.get("text"))
        .and_then(json_text)
}

fn json_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Object(map) => map
            .get("text")
            .or_else(|| map.get("content"))
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

fn json_index(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|item| {
            item.as_u64()
                .or_else(|| item.as_i64().and_then(|n| u64::try_from(n).ok()))
        })
        .unwrap_or(0) as usize
}

fn responses_usage(response: &Value) -> Option<(usize, usize, Option<usize>, Option<usize>)> {
    let usage = response.get("usage")?;
    let input = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(Value::as_u64)? as usize;
    let output = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(Value::as_u64)? as usize;
    let cache_read = {
        let hit = usage
            .get("prompt_cache_hit_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let details = usage
            .get("input_tokens_details")
            .or_else(|| usage.get("prompt_tokens_details"))
            .and_then(|details| details.get("cached_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        (hit > 0 || details > 0 || usage.get("prompt_cache_hit_tokens").is_some())
            .then_some(hit.max(details) as usize)
    };
    let reasoning_tokens = usage
        .get("output_tokens_details")
        .or_else(|| usage.get("completion_tokens_details"))
        .and_then(|details| details.get("reasoning_tokens"))
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    Some((input, output, cache_read, reasoning_tokens))
}

fn collect_responses_message_text(response: &Value) -> Option<String> {
    let output = response.get("output")?.as_array()?;
    let mut text = String::new();
    for item in output {
        if let Some(part) = collect_item_message_text(item) {
            text.push_str(&part);
        }
    }
    (!text.is_empty()).then_some(text)
}

fn collect_responses_reasoning_text(response: &Value) -> Option<String> {
    let output = response.get("output")?.as_array()?;
    let mut text = String::new();
    for item in output {
        if let Some(part) = collect_item_reasoning(item) {
            text.push_str(&part);
        }
    }
    (!text.is_empty()).then_some(text)
}

fn collect_responses_tool_calls(
    response: &Value,
    tool_calls: &mut HashMap<usize, ToolCallBuilder>,
    outcome: &mut StreamReadOutcome,
) {
    let Some(output) = response.get("output").and_then(Value::as_array) else {
        return;
    };
    for (index, item) in output.iter().enumerate() {
        if item.get("type").and_then(Value::as_str) != Some("function_call") {
            continue;
        }
        let entry = tool_calls.entry(index).or_default();
        if let Some(id) = item
            .get("call_id")
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            entry.id = id.to_string();
        }
        if let Some(name) = item
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            entry.set_name(name);
        }
        if let Some(args) = item.get("arguments").and_then(Value::as_str) {
            if entry.arguments.is_empty() {
                entry.arguments = args.to_string();
            }
        }
        outcome.emitted = true;
    }
}

fn collect_item_message_text(item: &Value) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    let parts = item.get("content")?.as_array()?;
    let mut text = String::new();
    for part in parts {
        let kind = part.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "output_text" || kind == "text" {
            if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                text.push_str(chunk);
            }
        }
    }
    (!text.is_empty()).then_some(text)
}

fn collect_item_reasoning(item: &Value) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("reasoning") {
        return None;
    }
    let mut text = String::new();
    if let Some(parts) = item.get("summary").and_then(Value::as_array) {
        for part in parts {
            if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                text.push_str(chunk);
            }
        }
    }
    if text.is_empty() {
        if let Some(parts) = item.get("content").and_then(Value::as_array) {
            for part in parts {
                if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                    text.push_str(chunk);
                }
            }
        }
    }
    (!text.is_empty()).then_some(text)
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

fn deepseek_http_status(message: &str) -> Option<u16> {
    let rest = message.strip_prefix("DeepSeek API ")?;
    rest.split_whitespace().next()?.parse().ok()
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

#[cfg(test)]
mod responses_event_tests {
    use super::*;
    use serde_json::json;

    fn apply(event: Value) -> (String, String, StreamReadOutcome, HashMap<usize, ToolCallBuilder>) {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = HashMap::new();
        let mut outcome = StreamReadOutcome::default();
        apply_responses_event(
            &event,
            &mut content,
            &mut reasoning,
            &mut tool_calls,
            &mut outcome,
        );
        (content, reasoning, outcome, tool_calls)
    }

    #[test]
    fn reasoning_summary_delta_is_collected() {
        let (content, reasoning, outcome, _) = apply(json!({
            "type": "response.reasoning_summary_text.delta",
            "delta": "step one"
        }));
        assert_eq!(reasoning, "step one");
        assert!(content.is_empty());
        assert!(outcome.emitted);
    }

    #[test]
    fn output_text_delta_is_collected() {
        let (content, reasoning, outcome, _) = apply(json!({
            "type": "response.output_text.delta",
            "delta": "hello"
        }));
        assert_eq!(content, "hello");
        assert!(reasoning.is_empty());
        assert!(outcome.emitted);
    }

    #[test]
    fn reasoning_summary_part_added_is_collected() {
        let (_, reasoning, outcome, _) = apply(json!({
            "type": "response.reasoning_summary_part.added",
            "part": { "type": "summary_text", "text": "consider energy" }
        }));
        assert_eq!(reasoning, "consider energy");
        assert!(outcome.emitted);
    }

    #[test]
    fn reasoning_item_done_is_collected() {
        let (_, reasoning, outcome, _) = apply(json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "item": {
                "type": "reasoning",
                "summary": [{ "type": "summary_text", "text": "full trace" }]
            }
        }));
        assert_eq!(reasoning, "full trace");
        assert!(outcome.emitted);
    }

    #[test]
    fn function_call_item_is_collected() {
        let (_, _, outcome, tool_calls) = apply(json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "item": {
                "type": "function_call",
                "call_id": "call-1",
                "name": "read_file",
                "arguments": "{\"path\":\"a.rs\"}"
            }
        }));
        let call = tool_calls.get(&0).expect("tool call");
        assert_eq!(call.id, "call-1");
        assert_eq!(call.name, "read_file");
        assert_eq!(call.arguments, "{\"path\":\"a.rs\"}");
        assert!(outcome.emitted);
    }

    #[test]
    fn completed_marks_stream_done() {
        let (_, _, outcome, _) = apply(json!({
            "type": "response.completed",
            "response": { "status": "completed" }
        }));
        assert!(outcome.is_complete());
        assert_eq!(outcome.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn cache_read_prefers_first_party_hit_tokens() {
        let usage: ApiTokenUsage = serde_json::from_value(json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "prompt_cache_hit_tokens": 80,
            "prompt_tokens_details": { "cached_tokens": 0 }
        }))
        .unwrap();
        assert_eq!(usage.cache_read_tokens(), 80);

        let usage: ApiTokenUsage = serde_json::from_value(json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "prompt_tokens_details": { "cached_tokens": 72 }
        }))
        .unwrap();
        assert_eq!(usage.cache_read_tokens(), 72);
    }

    #[test]
    fn responses_usage_reads_deepseek_cache_hit_field() {
        let parsed = responses_usage(&json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 8,
                "prompt_cache_hit_tokens": 90
            }
        }))
        .expect("usage");
        assert_eq!(parsed, (100, 8, Some(90), None));
    }
}

#[cfg(test)]
mod retry_tests {
    use super::{deepseek_http_status, is_retryable_stream_error};
    use crate::core::ai::provider::ProviderError;

    #[test]
    fn retries_transient_deepseek_http_errors_but_not_client_errors() {
        assert!(is_retryable_stream_error(&ProviderError::message(
            r#"DeepSeek API 500 Internal Server Error: {"type":"error"}"#
        )));
        assert!(is_retryable_stream_error(&ProviderError::message(
            "DeepSeek API 429 Too Many Requests: rate limited"
        )));
        assert!(!is_retryable_stream_error(&ProviderError::message(
            r#"DeepSeek API 400 Bad Request: {"error":{"message":"Unsupported model mimo-v2-omni"}}"#
        )));
        assert!(!is_retryable_stream_error(&ProviderError::message(
            "DeepSeek API 401 Unauthorized: bad key"
        )));
        assert_eq!(
            deepseek_http_status("DeepSeek API 500 Internal Server Error: x"),
            Some(500)
        );
    }
}
