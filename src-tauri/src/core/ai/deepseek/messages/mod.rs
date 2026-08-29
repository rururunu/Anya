//! Chat Completions and Responses API request bodies.

mod convert;
mod reasoning;

use serde_json::{json, Map, Value};

use crate::core::runtime::{ChatMessage, ChatRequest, Role};
use crate::models::settings::ReasoningEffort;

use convert::{normalize_tool_protocol, parse_multimodal_content};
use reasoning::apply_chat_reasoning;

pub(crate) use convert::message_to_api_json;

pub(crate) fn build_api_body(
    request: &ChatRequest,
    model: &str,
    stream: bool,
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    continue_thinking_after_tools: bool,
    include_thinking: bool,
) -> Value {
    let continuing = is_tool_continuation(&request.messages);
    let effective_effort = resolve_round_effort(effort, continuing, continue_thinking_after_tools);

    // Thinking + tools requires prior reasoning text on tool-call history.
    // Force pass when effort is on, or when a later round follows a thought
    // that already ran tools — even if this round itself skips thinking.
    let effective_pass =
        resolve_pass_tool_reasoning(effort, pass_tool_reasoning, continuing, &request.messages);

    // DeepSeek-specific wire quirks (empty-content passback, empty tool output,
    // cache-hit token accounting) only apply to actual DeepSeek models, never to
    // custom OpenAI-compatible providers routed through the same adapter.
    let is_deepseek = model.trim().to_ascii_lowercase().starts_with("deepseek");
    let messages: Vec<_> = prepared_messages(request)
        .iter()
        .map(|message| message_to_api_json(message, effective_pass, is_deepseek))
        .collect();

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("messages".into(), Value::Array(messages));
    body.insert("stream".into(), json!(stream));

    if stream {
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }

    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_tokens".into(), json!(max_tokens));
    }

    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(request.tools.iter().cloned().collect()),
        );
    }

    if include_thinking {
        apply_chat_reasoning(&mut body, model, is_deepseek, effective_effort);
    }

    Value::Object(body)
}

/// xAI / OpenAI Responses API body. Grok only returns visible reasoning here,
/// not on `/chat/completions`.
pub(crate) fn build_responses_body(
    request: &ChatRequest,
    model: &str,
    stream: bool,
    effort: ReasoningEffort,
    continue_thinking_after_tools: bool,
) -> Value {
    let continuing = is_tool_continuation(&request.messages);
    let effective_effort = resolve_round_effort(effort, continuing, continue_thinking_after_tools);
    let messages = prepared_messages(request);
    let input = messages_to_responses_input(&messages);

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("input".into(), Value::Array(input));
    body.insert("stream".into(), json!(stream));
    apply_xai_reasoning(&mut body, effective_effort);

    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_output_tokens".into(), json!(max_tokens));
    }
    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(
                request
                    .tools
                    .iter()
                    .cloned()
                    .map(flatten_function_tool)
                    .collect(),
            ),
        );
    }

    Value::Object(body)
}

pub(super) fn prepared_messages(request: &ChatRequest) -> Vec<ChatMessage> {
    let mut messages: Vec<_> = request
        .messages
        .iter()
        .filter(|message| message.contributes_to_api())
        .cloned()
        .collect();
    normalize_tool_protocol(&mut messages);
    messages
}

fn messages_to_responses_input(messages: &[ChatMessage]) -> Vec<Value> {
    let mut input = Vec::new();
    for message in messages {
        match message.role {
            Role::Tool => {
                let output = if message.content.trim().is_empty() {
                    "(no output)".to_string()
                } else {
                    message.content.clone()
                };
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": message.tool_call_id.clone().unwrap_or_default(),
                    "output": output,
                }));
            }
            Role::Assistant => {
                if let Some(tool_calls) = message
                    .tool_calls
                    .as_ref()
                    .filter(|calls| !calls.is_empty())
                {
                    if !message.content.trim().is_empty() {
                        input.push(json!({
                            "role": "assistant",
                            "content": message.content,
                        }));
                    }
                    for call in tool_calls {
                        input.push(json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.arguments,
                        }));
                    }
                } else {
                    input.push(json!({
                        "role": "assistant",
                        "content": message.content,
                    }));
                }
            }
            Role::User => {
                input.push(json!({
                    "role": "user",
                    "content": to_responses_user_content(&message.content),
                }));
            }
            Role::System => {
                input.push(json!({
                    "role": "system",
                    "content": message.content,
                }));
            }
        }
    }
    input
}

fn to_responses_user_content(content: &str) -> Value {
    match parse_multimodal_content(content) {
        Value::Array(parts) => Value::Array(
            parts
                .into_iter()
                .map(|part| {
                    let kind = part.get("type").and_then(Value::as_str).unwrap_or("");
                    match kind {
                        "image_url" => {
                            let url = part
                                .get("image_url")
                                .and_then(|image| {
                                    image
                                        .as_str()
                                        .or_else(|| image.get("url").and_then(Value::as_str))
                                })
                                .unwrap_or("");
                            json!({ "type": "input_image", "image_url": url })
                        }
                        "text" => json!({
                            "type": "input_text",
                            "text": part.get("text").and_then(Value::as_str).unwrap_or(""),
                        }),
                        _ => part,
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

fn flatten_function_tool(tool: Value) -> Value {
    let Some(obj) = tool.as_object() else {
        return tool;
    };
    if obj.get("type").and_then(Value::as_str) != Some("function") {
        return tool;
    }
    if obj.contains_key("name") {
        return tool;
    }
    let Some(func) = obj.get("function").and_then(Value::as_object) else {
        return tool;
    };
    let mut out = Map::new();
    out.insert("type".into(), json!("function"));
    for key in ["name", "description", "parameters", "strict"] {
        if let Some(value) = func.get(key) {
            out.insert(key.into(), value.clone());
        }
    }
    Value::Object(out)
}

fn apply_xai_reasoning(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    // grok-4.5 / 4.6 cannot disable reasoning. App default is Disabled (DeepSeek
    // off); map that to xAI's default `high` so summaries actually come back.
    let effort = match effort {
        ReasoningEffort::Disabled | ReasoningEffort::None | ReasoningEffort::High => "high",
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::Xhigh | ReasoningEffort::Max => "xhigh",
    };
    body.insert(
        "reasoning".into(),
        json!({ "effort": effort, "summary": "auto" }),
    );
}

/// Session-stable thinking policy for one provider round.
///
/// Default (`continue_thinking_after_tools = true`): keep the configured
/// effort for every agent-loop round, including after tools.
/// Opt-out: disable thinking on continuation rounds to save tokens.
pub(super) fn is_thinking_off(effort: ReasoningEffort) -> bool {
    matches!(effort, ReasoningEffort::Disabled | ReasoningEffort::None)
}

/// Default (`continue_thinking_after_tools = true`): keep the configured
/// effort for every agent-loop round, including after tools.
/// Opt-out: disable thinking on continuation rounds to save tokens.
pub(super) fn resolve_round_effort(
    effort: ReasoningEffort,
    continuing: bool,
    continue_thinking_after_tools: bool,
) -> ReasoningEffort {
    if continuing && !continue_thinking_after_tools {
        ReasoningEffort::Disabled
    } else {
        effort
    }
}

fn resolve_pass_tool_reasoning(
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    continuing: bool,
    messages: &[ChatMessage],
) -> bool {
    let protocol_requires = continuing && messages_have_tool_call_reasoning(messages);
    if is_thinking_off(effort) {
        protocol_requires
    } else {
        // Prefer the user setting; still force-pass when the protocol requires it.
        pass_tool_reasoning || protocol_requires
    }
}

/// True when this request already includes tool results after the latest real
/// user message — i.e. an agent-loop continuation, not the opening model call.
pub(super) fn is_tool_continuation(messages: &[crate::core::runtime::ChatMessage]) -> bool {
    for message in messages.iter().rev() {
        match message.role {
            Role::Tool => return true,
            Role::User if message.content.starts_with("[System]") => return true,
            Role::User => return false,
            _ => {}
        }
    }
    false
}

pub(super) fn messages_have_tool_call_reasoning(
    messages: &[crate::core::runtime::ChatMessage],
) -> bool {
    messages.iter().any(|message| {
        message
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
            && message
                .reasoning
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
    })
}

pub(super) fn non_empty_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
