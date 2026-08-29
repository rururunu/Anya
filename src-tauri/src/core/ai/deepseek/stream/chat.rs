use crate::core::ai::deepseek::messages::non_empty_option;
use crate::core::ai::provider::ProviderError;
use crate::core::runtime::{StreamEvent, ToolCallPayload};
use crate::core::token::TokenUsage;
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use super::errors::map_read_error;
use super::sse::next_sse_line;
use super::types::{
    ApiStreamResponse, ApiStreamDelta, StreamReadOutcome, ToolCallBuilder, STREAM_IDLE_TIMEOUT, USER_STREAM_INTERRUPTED,
    USER_STREAM_STALLED,
};

pub(super) async fn read_sse_stream(
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
                            if let Some(name) = &function.name {
                                entry.set_name(name.as_str());
                            }
                            if let Some(args) = &function.arguments {
                                entry.arguments.push_str(args);
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
