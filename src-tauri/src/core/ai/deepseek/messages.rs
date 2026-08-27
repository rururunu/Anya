use std::collections::HashSet;

use serde_json::{json, Map, Value};

use crate::core::runtime::{ChatMessage, ChatRequest, Role};
use crate::models::settings::ReasoningEffort;

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

fn parse_multimodal_content(content: &str) -> Value {
    let content = crate::core::ai::image_gen::strip_edit_region_images(content);
    if !content.contains("![image](") {
        return json!(content);
    }

    let re = match regex::Regex::new(r"!\[image\]\((.*?)\)") {
        Ok(re) => re,
        Err(_) => return json!(content),
    };

    let mut parts = Vec::new();
    let mut last_index = 0;

    for cap in re.captures_iter(&content) {
        if let Some(mat) = cap.get(0) {
            let before = &content[last_index..mat.start()];
            if !before.trim().is_empty() {
                parts.push(json!({
                    "type": "text",
                    "text": before,
                }));
            }

            if let Some(url_match) = cap.get(1) {
                let url = url_match.as_str();
                parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": url,
                    }
                }));
            }

            last_index = mat.end();
        }
    }

    if last_index < content.len() {
        let after = &content[last_index..];
        if !after.trim().is_empty() {
            parts.push(json!({
                "type": "text",
                "text": after,
            }));
        }
    }

    if parts.is_empty() {
        json!(content)
    } else {
        Value::Array(parts)
    }
}

/// Drop `role=tool` rows that are not a response to the nearest preceding
/// assistant `tool_calls`. DeepSeek (and other OpenAI-compatible APIs) return
/// 400 otherwise. Also fill empty tool-call ids so the pair can still match.
pub(super) fn normalize_tool_protocol(messages: &mut Vec<ChatMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role == Role::Assistant {
            if let Some(calls) = messages[i].tool_calls.as_mut() {
                for (index, call) in calls.iter_mut().enumerate() {
                    if call.id.trim().is_empty() {
                        call.id = format!("call-{index}");
                    }
                }
                if calls.is_empty() {
                    messages[i].tool_calls = None;
                }
            }
            i += 1;
            continue;
        }

        if messages[i].role != Role::Tool {
            i += 1;
            continue;
        }

        let mut block_start = i;
        while block_start > 0 && messages[block_start - 1].role == Role::Tool {
            block_start -= 1;
        }
        let assistant_idx = block_start.checked_sub(1);
        let pending: Vec<String> = assistant_idx
            .and_then(|idx| {
                let assistant = &messages[idx];
                if assistant.role != Role::Assistant {
                    return None;
                }
                assistant.tool_calls.as_ref().and_then(|calls| {
                    if calls.is_empty() {
                        None
                    } else {
                        Some(calls.iter().map(|call| call.id.clone()).collect())
                    }
                })
            })
            .unwrap_or_default();

        if pending.is_empty() {
            messages.remove(i);
            continue;
        }

        let used: HashSet<String> = messages[block_start..i]
            .iter()
            .filter_map(|message| message.tool_call_id.clone())
            .collect();
        let raw_id = messages[i]
            .tool_call_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let matched = if raw_id.is_empty() {
            pending.iter().find(|id| !used.contains(*id)).cloned()
        } else if pending.contains(&raw_id) && !used.contains(&raw_id) {
            Some(raw_id)
        } else {
            None
        };

        match matched {
            Some(id) => {
                messages[i].tool_call_id = Some(id);
                i += 1;
            }
            None => {
                messages.remove(i);
            }
        }
    }
}

pub(super) fn message_to_api_json(
    message: &ChatMessage,
    pass_tool_reasoning: bool,
    is_deepseek: bool,
) -> Value {
    if message.role == Role::Tool {
        // DeepSeek rejects empty tool output; other providers tolerate "".
        let content = if is_deepseek && message.content.trim().is_empty() {
            "(no output)".to_string()
        } else {
            message.content.clone()
        };
        let mut payload = json!({
            "role": "tool",
            "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
            "content": content,
        });
        if let Some(name) = message.name.as_deref().filter(|name| !name.is_empty()) {
            payload
                .as_object_mut()
                .expect("tool payload object")
                .insert("name".into(), json!(name));
        }
        return payload;
    }

    if message.role == Role::Assistant {
        if let Some(tool_calls) = message
            .tool_calls
            .as_ref()
            .filter(|calls| !calls.is_empty())
        {
            let calls: Vec<Value> = tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.arguments,
                        }
                    })
                })
                .collect();
            let mut payload = json!({
                "role": "assistant",
                // DeepSeek (and some gateways) reject null content on text-less
                // tool-call turns — send an empty string instead.
                "content": if message.content.is_empty() {
                    if is_deepseek { json!("") } else { Value::Null }
                } else { json!(message.content) },
                "tool_calls": calls,
            });
            if pass_tool_reasoning {
                // Protocol requires reasoning_content on every tool-call
                // assistant message once thinking was used; space placeholder
                // if empty.
                let reasoning = message
                    .reasoning
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(" ");
                payload
                    .as_object_mut()
                    .expect("assistant payload object")
                    .insert("reasoning_content".into(), json!(reasoning));
            }
            return payload;
        }
    }

    json!({
        "role": role_to_api(message.role),
        "content": parse_multimodal_content(&message.content),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReasoningFamily {
    DeepSeek,
    OpenAi,
    KimiK3,
    KimiK2,
    Qwen38,
    Qwen,
    Glm52,
    Glm,
    Claude,
    MiniMax,
    Other,
}

fn reasoning_family(model: &str) -> ReasoningFamily {
    let model = model.trim().to_ascii_lowercase();
    if model.contains("deepseek") {
        return ReasoningFamily::DeepSeek;
    }
    if model.contains("kimi-k3") || model.contains("kimi_k3") || model.contains("kimik3") {
        return ReasoningFamily::KimiK3;
    }
    if model.contains("kimi") || model.contains("moonshot") {
        return ReasoningFamily::KimiK2;
    }
    if model.contains("qwen3.8") || model.contains("qwen3-8") || model.contains("qwen38") {
        return ReasoningFamily::Qwen38;
    }
    if model.contains("qwen") || model.contains("qwq") || model.contains("qvq") {
        return ReasoningFamily::Qwen;
    }
    if model.contains("glm-5") || model.contains("glm5") || model.contains("glm_5") {
        return ReasoningFamily::Glm52;
    }
    if model.contains("glm") || model.contains("chatglm") {
        return ReasoningFamily::Glm;
    }
    if model.contains("claude") || model.contains("anthropic") {
        return ReasoningFamily::Claude;
    }
    if model.contains("minimax") || model.contains("mimo") {
        return ReasoningFamily::MiniMax;
    }
    if model.contains("gpt-5") || is_openai_o_series(&model) {
        return ReasoningFamily::OpenAi;
    }
    ReasoningFamily::Other
}

fn is_openai_o_series(model: &str) -> bool {
    ["o1", "o3", "o4"].iter().any(|needle| {
        model == *needle
            || model.contains(&format!("{needle}-"))
            || model.contains(&format!("-{needle}"))
            || model.contains(&format!("/{needle}"))
            || model.contains(&format!(".{needle}"))
    })
}

fn apply_chat_reasoning(
    body: &mut Map<String, Value>,
    model: &str,
    is_deepseek: bool,
    effort: ReasoningEffort,
) {
    if is_deepseek {
        apply_deepseek_thinking(body, effort);
        return;
    }
    match reasoning_family(model) {
        ReasoningFamily::DeepSeek => apply_deepseek_thinking(body, effort),
        ReasoningFamily::KimiK3 => apply_kimi_k3_effort(body, effort),
        ReasoningFamily::KimiK2 => apply_toggle_thinking(body, effort),
        ReasoningFamily::Qwen38 => apply_qwen38_effort(body, effort),
        ReasoningFamily::Qwen => apply_qwen_thinking(body, effort),
        ReasoningFamily::Glm52 => apply_glm52_effort(body, effort),
        ReasoningFamily::Glm => apply_toggle_thinking(body, effort),
        ReasoningFamily::OpenAi | ReasoningFamily::Claude | ReasoningFamily::MiniMax => {
            apply_openai_reasoning_effort(body, effort);
        }
        ReasoningFamily::Other => {}
    }
}

fn openai_effort_wire(effort: ReasoningEffort) -> Option<&'static str> {
    match effort {
        ReasoningEffort::Disabled => None,
        ReasoningEffort::None => Some("none"),
        ReasoningEffort::Minimal => Some("minimal"),
        ReasoningEffort::Low => Some("low"),
        ReasoningEffort::Medium => Some("medium"),
        ReasoningEffort::High => Some("high"),
        ReasoningEffort::Xhigh => Some("xhigh"),
        ReasoningEffort::Max => Some("max"),
    }
}

fn apply_openai_reasoning_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}

fn apply_deepseek_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        return;
    }
    let mapped = match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Max => "max",
        _ => "high",
    };
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_toggle_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("enable_thinking".into(), json!(true));
}

fn apply_kimi_k3_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    let mapped = match effort {
        ReasoningEffort::Disabled | ReasoningEffort::None => "max",
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Max | ReasoningEffort::Xhigh => "max",
        ReasoningEffort::Medium | ReasoningEffort::High => "high",
    };
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_qwen38_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    let mapped = match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        _ => "xhigh",
    };
    body.insert("enable_thinking".into(), json!(true));
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_qwen_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    body.insert("enable_thinking".into(), json!(true));
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}

fn apply_glm52_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) || matches!(effort, ReasoningEffort::Minimal) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        body.insert("enable_thinking".into(), json!(false));
        if matches!(effort, ReasoningEffort::None | ReasoningEffort::Minimal) {
            if let Some(value) = openai_effort_wire(effort) {
                body.insert("reasoning_effort".into(), json!(value));
            }
        }
        return;
    }
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("enable_thinking".into(), json!(true));
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}

fn role_to_api(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

pub(super) fn non_empty_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
