use crate::core::ai::deepseek::messages::non_empty_option;
use crate::core::ai::provider::ProviderError;
use crate::core::runtime::{StreamEvent, ToolCallPayload};
use crate::core::token::TokenUsage;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use super::errors::map_read_error;
use super::sse::next_sse_line;
use super::types::{
    StreamReadOutcome, ToolCallBuilder, STREAM_IDLE_TIMEOUT, USER_STREAM_INTERRUPTED, USER_STREAM_STALLED,
};

pub(super) async fn read_responses_sse_stream(
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

            let tick = apply_responses_event(
                &parsed,
                &mut content,
                &mut reasoning,
                &mut tool_calls,
                &mut outcome,
            );
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
                let _ = tx.send(StreamEvent::Reasoning(tick.reasoning_delta)).await;
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
#[derive(Default)]
pub(super) struct ResponsesTick {
    pub(super) content_delta: String,
    pub(super) reasoning_delta: String,
    pub(super) error: Option<String>,
    pub(super) usage: Option<(usize, usize, Option<usize>, Option<usize>)>,
}

pub(super) fn apply_responses_event(
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
                    tool_calls
                        .entry(index)
                        .or_default()
                        .arguments
                        .push_str(delta);
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
            if let Some(name) = item.get("name").and_then(Value::as_str) {
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

pub(super) fn append_delta(
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

pub(super) fn json_index(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|item| {
            item.as_u64()
                .or_else(|| item.as_i64().and_then(|n| u64::try_from(n).ok()))
        })
        .unwrap_or(0) as usize
}

pub(super) fn responses_usage(response: &Value) -> Option<(usize, usize, Option<usize>, Option<usize>)> {
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
