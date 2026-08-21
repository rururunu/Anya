//! Anthropic Messages wire format (`POST /v1/messages`).
//!
//! Aggregators such as OpenCode Go / Console Go list MiniMax on `/models` but
//! reject OpenAI Chat Completions with
//! `Model minimax-m3 is not supported for format openai`. Those models must be
//! posted as Anthropic Messages instead.

use serde_json::{json, Map, Value};

use crate::core::runtime::{ChatMessage, ChatRequest, Role};
use crate::models::settings::{ModelWireProtocol, ProviderApiProtocol, ReasoningEffort};

use super::messages::{
    is_thinking_off, is_tool_continuation, prepared_messages, resolve_round_effort,
};

const DEFAULT_MAX_TOKENS: u32 = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WireProtocol {
    ChatCompletions,
    Responses,
    AnthropicMessages,
}

impl From<ModelWireProtocol> for WireProtocol {
    fn from(value: ModelWireProtocol) -> Self {
        match value {
            ModelWireProtocol::ChatCompletions => Self::ChatCompletions,
            ModelWireProtocol::Responses => Self::Responses,
            ModelWireProtocol::AnthropicMessages => Self::AnthropicMessages,
        }
    }
}

impl From<WireProtocol> for ModelWireProtocol {
    fn from(value: WireProtocol) -> Self {
        match value {
            WireProtocol::ChatCompletions => Self::ChatCompletions,
            WireProtocol::Responses => Self::Responses,
            WireProtocol::AnthropicMessages => Self::AnthropicMessages,
        }
    }
}

impl From<ProviderApiProtocol> for WireProtocol {
    fn from(value: ProviderApiProtocol) -> Self {
        match value {
            ProviderApiProtocol::ChatCompletions => Self::ChatCompletions,
            ProviderApiProtocol::Responses => Self::Responses,
        }
    }
}

const WIRE_PROTOCOL_FALLBACK_ORDER: [WireProtocol; 3] = [
    WireProtocol::AnthropicMessages,
    WireProtocol::ChatCompletions,
    WireProtocol::Responses,
];

pub(super) fn remaining_wire_protocols(failed: WireProtocol) -> Vec<WireProtocol> {
    WIRE_PROTOCOL_FALLBACK_ORDER
        .into_iter()
        .filter(|protocol| *protocol != failed)
        .collect()
}

pub(super) fn url_for_wire_protocol(base_url: Option<&str>, protocol: WireProtocol) -> String {
    match (base_url, protocol) {
        (None, WireProtocol::ChatCompletions) => {
            "https://api.deepseek.com/chat/completions".into()
        }
        (base, WireProtocol::ChatCompletions) => {
            super::models::normalize_chat_completions_url(base.unwrap_or("https://api.deepseek.com/v1"))
        }
        (base, WireProtocol::Responses) => {
            super::models::normalize_responses_url(base.unwrap_or("https://api.deepseek.com/v1"))
        }
        (base, WireProtocol::AnthropicMessages) => super::models::normalize_anthropic_messages_url(
            base.unwrap_or("https://api.deepseek.com/v1"),
        ),
    }
}

pub(super) fn anthropic_format_model(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    model.contains("minimax")
        || model.contains("claude")
        || model.contains("anthropic")
        || model.contains("qwen3.7")
}

pub(super) fn prefers_anthropic_messages(model: &str, url: &str) -> bool {
    if !anthropic_format_model(model) {
        return false;
    }
    let url = url.trim().to_ascii_lowercase();
    if url.contains("/anthropic") || url.contains("/messages") {
        return true;
    }
    // First-party MiniMax OpenAI hosts speak Chat Completions.
    if (url.contains("api.minimax.io") || url.contains("api.minimaxi.com"))
        && !url.contains("/anthropic")
    {
        return false;
    }
    // Official DeepSeek cannot host MiniMax on either protocol.
    if url.contains("api.deepseek.com") {
        return false;
    }
    // Custom / aggregator endpoints serve MiniMax on Anthropic Messages.
    true
}

pub(super) fn resolve_wire_protocol(
    configured: ProviderApiProtocol,
    model: &str,
    url: &str,
    cached: Option<WireProtocol>,
) -> WireProtocol {
    if let Some(cached) = cached {
        return cached;
    }
    if prefers_anthropic_messages(model, url) {
        return WireProtocol::AnthropicMessages;
    }
    WireProtocol::from(configured)
}

pub(super) fn is_format_rejected(error: &str) -> bool {
    let text = error.to_ascii_lowercase();
    text.contains("not supported for format")
        || (text.contains("modelerror") && text.contains("not supported"))
}

pub(super) fn build_anthropic_body(
    request: &ChatRequest,
    model: &str,
    stream: bool,
    effort: ReasoningEffort,
    continue_thinking_after_tools: bool,
) -> Value {
    let continuing = is_tool_continuation(&request.messages);
    let effective_effort = resolve_round_effort(effort, continuing, continue_thinking_after_tools);
    let prepared = prepared_messages(request);
    let (system, messages) = anthropic_messages(&prepared);

    let thinking_budget = anthropic_thinking_budget(model, effective_effort);
    let mut max_tokens = request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
    if let Some(budget) = thinking_budget {
        max_tokens = max_tokens.max(budget.saturating_add(4_096));
    }

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("max_tokens".into(), json!(max_tokens));
    body.insert("stream".into(), json!(stream));
    body.insert("messages".into(), Value::Array(messages));
    if !system.is_empty() {
        body.insert("system".into(), json!(system));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if !request.tools.is_empty() {
        body.insert("tools".into(), Value::Array(anthropic_tools(&request.tools)));
    }
    apply_anthropic_thinking(&mut body, model, effective_effort);
    Value::Object(body)
}

fn anthropic_tools(tools: &[Value]) -> Vec<Value> {
    tools.iter().filter_map(openai_tool_to_anthropic).collect()
}

fn openai_tool_to_anthropic(tool: &Value) -> Option<Value> {
    let function = if tool.get("type").and_then(Value::as_str) == Some("function") {
        tool.get("function")?
    } else {
        tool
    };
    let name = function.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let description = function
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let input_schema = function
        .get("parameters")
        .cloned()
        .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
    Some(json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
    }))
}

fn anthropic_messages(messages: &[ChatMessage]) -> (String, Vec<Value>) {
    let mut system = Vec::new();
    let mut api_messages = Vec::new();
    let mut pending_tool_results: Vec<Value> = Vec::new();

    let flush_tools = |pending: &mut Vec<Value>, out: &mut Vec<Value>| {
        if pending.is_empty() {
            return;
        }
        out.push(json!({
            "role": "user",
            "content": std::mem::take(pending),
        }));
    };

    for message in messages {
        match message.role {
            Role::System => {
                let trimmed = message.content.trim();
                if !trimmed.is_empty() {
                    system.push(trimmed.to_string());
                }
            }
            Role::Tool => {
                pending_tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": message.tool_call_id.clone().unwrap_or_default(),
                    "content": if message.content.trim().is_empty() {
                        "(no output)".to_string()
                    } else {
                        message.content.clone()
                    },
                }));
            }
            Role::User => {
                flush_tools(&mut pending_tool_results, &mut api_messages);
                api_messages.push(json!({
                    "role": "user",
                    "content": anthropic_user_content(&message.content),
                }));
            }
            Role::Assistant => {
                flush_tools(&mut pending_tool_results, &mut api_messages);
                api_messages.push(anthropic_assistant_message(message));
            }
        }
    }
    flush_tools(&mut pending_tool_results, &mut api_messages);
    (system.join("\n\n"), api_messages)
}

fn anthropic_user_content(content: &str) -> Value {
    if !content.contains("![image](") {
        return json!(content);
    }
    let Ok(re) = regex::Regex::new(r"!\[image\]\((.*?)\)") else {
        return json!(content);
    };
    let mut parts = Vec::new();
    let mut last = 0;
    for cap in re.captures_iter(content) {
        let Some(mat) = cap.get(0) else { continue };
        let before = &content[last..mat.start()];
        if !before.trim().is_empty() {
            parts.push(json!({ "type": "text", "text": before }));
        }
        if let Some(url) = cap.get(1).map(|m| m.as_str()).filter(|url| !url.is_empty()) {
            if let Some(b64) = url.strip_prefix("data:") {
                if let Some((_, data)) = b64.split_once("base64,") {
                    let media_type = b64
                        .split(';')
                        .next()
                        .unwrap_or("image/png")
                        .trim()
                        .to_string();
                    parts.push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": data,
                        }
                    }));
                }
            } else {
                parts.push(json!({
                    "type": "image",
                    "source": { "type": "url", "url": url }
                }));
            }
        }
        last = mat.end();
    }
    if last < content.len() {
        let after = &content[last..];
        if !after.trim().is_empty() {
            parts.push(json!({ "type": "text", "text": after }));
        }
    }
    if parts.is_empty() {
        json!(content)
    } else {
        Value::Array(parts)
    }
}

fn anthropic_assistant_message(message: &ChatMessage) -> Value {
    let mut blocks = Vec::new();
    if let Some(reasoning) = message
        .reasoning
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        blocks.push(json!({
            "type": "thinking",
            "thinking": reasoning,
        }));
    }
    if !message.content.trim().is_empty() {
        blocks.push(json!({
            "type": "text",
            "text": message.content,
        }));
    }
    if let Some(calls) = message.tool_calls.as_ref().filter(|calls| !calls.is_empty()) {
        for call in calls {
            blocks.push(json!({
                "type": "tool_use",
                "id": call.id,
                "name": call.name,
                "input": parse_tool_input(&call.arguments),
            }));
        }
    }
    if blocks.is_empty() {
        json!({ "role": "assistant", "content": "" })
    } else {
        json!({ "role": "assistant", "content": blocks })
    }
}

fn parse_tool_input(arguments: &str) -> Value {
    let trimmed = arguments.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| json!({}))
}

fn anthropic_thinking_budget(model: &str, effort: ReasoningEffort) -> Option<u32> {
    if is_thinking_off(effort) {
        return None;
    }
    let model = model.trim().to_ascii_lowercase();
    if model.contains("minimax") {
        return None;
    }
    Some(match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => 2_048,
        ReasoningEffort::Medium => 8_192,
        ReasoningEffort::High => 16_384,
        ReasoningEffort::Xhigh | ReasoningEffort::Max => 32_768,
        ReasoningEffort::Disabled | ReasoningEffort::None => 0,
    })
}

fn apply_anthropic_thinking(body: &mut Map<String, Value>, model: &str, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        return;
    }
    let model = model.trim().to_ascii_lowercase();
    if model.contains("minimax") {
        body.insert("thinking".into(), json!({ "type": "adaptive" }));
        return;
    }
    if let Some(budget) = anthropic_thinking_budget(&model, effort) {
        body.insert(
            "thinking".into(),
            json!({ "type": "enabled", "budget_tokens": budget }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role, ToolCallPayload};

    fn msg(role: Role, content: &str) -> ChatMessage {
        ChatMessage {
            id: "m".into(),
            session_id: "s".into(),
            role,
            content: content.into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 0,
            estimated_tokens: None,
        }
    }

    #[test]
    fn minimax_on_aggregator_prefers_anthropic() {
        assert!(prefers_anthropic_messages(
            "minimax-m3",
            "https://opencode.ai/zen/go/v1/chat/completions"
        ));
        assert!(prefers_anthropic_messages(
            "minimax-m3",
            "https://console.example/v1/chat/completions"
        ));
        assert!(!prefers_anthropic_messages(
            "minimax-m3",
            "https://api.minimax.io/v1/chat/completions"
        ));
        assert!(!prefers_anthropic_messages(
            "deepseek-v4-pro",
            "https://opencode.ai/zen/go/v1/chat/completions"
        ));
        assert!(!prefers_anthropic_messages(
            "minimax-m3",
            "https://api.deepseek.com/chat/completions"
        ));
    }

    #[test]
    fn anthropic_body_lifts_system_and_tools() {
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            messages: vec![
                msg(Role::System, "be brief"),
                msg(Role::User, "hello"),
            ],
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([json!({
                "type": "function",
                "function": {
                    "name": "web_search",
                    "description": "search",
                    "parameters": { "type": "object" }
                }
            })]),
            temperature: None,
            max_tokens: None,
        };
        let body = build_anthropic_body(
            &request,
            "minimax-m3",
            true,
            ReasoningEffort::Disabled,
            true,
        );
        assert_eq!(body["model"], "minimax-m3");
        assert_eq!(body["system"], "be brief");
        assert_eq!(body["max_tokens"], 16384);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["tools"][0]["name"], "web_search");
        assert_eq!(body["thinking"]["type"], "disabled");
    }

    #[test]
    fn anthropic_body_converts_tool_roundtrip() {
        let mut assistant = msg(Role::Assistant, "");
        assistant.tool_calls = Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "web_search".into(),
            arguments: r#"{"q":"news"}"#.into(),
            thought_signature: None,
        }]);
        let mut tool = msg(Role::Tool, "result");
        tool.tool_call_id = Some("call-1".into());
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            messages: vec![msg(Role::User, "hi"), assistant, tool],
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: Some(1024),
        };
        let body = build_anthropic_body(
            &request,
            "minimax-m3",
            true,
            ReasoningEffort::High,
            true,
        );
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_use");
        assert_eq!(body["messages"][1]["content"][0]["input"]["q"], "news");
        assert_eq!(body["messages"][2]["content"][0]["type"], "tool_result");
        assert_eq!(body["thinking"]["type"], "adaptive");
    }

    #[test]
    fn resolve_protocol_cache_then_heuristic_then_default() {
        assert_eq!(
            resolve_wire_protocol(
                ProviderApiProtocol::Responses,
                "minimax-m3",
                "https://proxy.example/v1/chat/completions",
                Some(WireProtocol::ChatCompletions),
            ),
            WireProtocol::ChatCompletions
        );
        assert_eq!(
            resolve_wire_protocol(
                ProviderApiProtocol::Responses,
                "minimax-m3",
                "https://proxy.example/v1/chat/completions",
                None,
            ),
            WireProtocol::AnthropicMessages
        );
        assert_eq!(
            resolve_wire_protocol(
                ProviderApiProtocol::Responses,
                "deepseek-v4-pro",
                "https://proxy.example/v1/chat/completions",
                None,
            ),
            WireProtocol::Responses
        );
        assert_eq!(
            resolve_wire_protocol(
                ProviderApiProtocol::ChatCompletions,
                "minimax-m3",
                "https://api.minimax.io/v1/chat/completions",
                None,
            ),
            WireProtocol::ChatCompletions
        );
    }

    #[test]
    fn remaining_protocols_skip_the_failed_one() {
        assert_eq!(
            remaining_wire_protocols(WireProtocol::ChatCompletions),
            vec![WireProtocol::AnthropicMessages, WireProtocol::Responses]
        );
        assert_eq!(
            remaining_wire_protocols(WireProtocol::AnthropicMessages),
            vec![WireProtocol::ChatCompletions, WireProtocol::Responses]
        );
        assert_eq!(
            remaining_wire_protocols(WireProtocol::Responses),
            vec![WireProtocol::AnthropicMessages, WireProtocol::ChatCompletions]
        );
    }

    #[test]
    fn format_rejection_covers_anthropic_and_openai() {
        assert!(is_format_rejected(
            r#"DeepSeek API 401 Unauthorized: {"type":"error","error":{"type":"ModelError","message":"Model minimax-m3 is not supported for format openai"}}"#
        ));
        assert!(is_format_rejected(
            r#"API 401: {"type":"error","error":{"type":"ModelError","message":"not supported for format anthropic"}}"#
        ));
        assert!(!is_format_rejected(
            "DeepSeek API 500 Internal Server Error: Internal server error"
        ));
    }

    #[test]
    fn url_for_wire_protocol_uses_provider_base() {
        assert_eq!(
            url_for_wire_protocol(
                Some("https://proxy.example/v1/chat/completions"),
                WireProtocol::AnthropicMessages
            ),
            "https://proxy.example/v1/messages"
        );
        assert_eq!(
            url_for_wire_protocol(Some("https://proxy.example/v1"), WireProtocol::Responses),
            "https://proxy.example/v1/responses"
        );
        assert_eq!(
            url_for_wire_protocol(None, WireProtocol::ChatCompletions),
            "https://api.deepseek.com/chat/completions"
        );
    }
}
