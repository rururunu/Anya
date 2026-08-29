use crate::core::ai::deepseek::messages::non_empty_option;
use crate::core::ai::provider::ProviderError;
use crate::core::runtime::{StreamEvent, ToolCallPayload};
use crate::core::token::TokenUsage;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use super::errors::map_read_error;
use super::responses::{append_delta, json_index, ResponsesTick};
use super::sse::next_sse_line;
use super::types::{
    StreamReadOutcome, ToolCallBuilder, STREAM_IDLE_TIMEOUT, USER_STREAM_INTERRUPTED, USER_STREAM_STALLED,
};

pub(super) async fn read_anthropic_sse_stream(
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
