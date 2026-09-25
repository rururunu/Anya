use std::time::Duration;

use super::messages::{build_api_body, build_responses_body, message_to_api_json};
use super::models::{
    endpoint_url_for_protocol, normalize_chat_completions_url, normalize_images_generations_url,
};
use super::multimodal::{
    multimodal_http_error_message, multimodal_transport_error_message, resolve_multimodal_endpoint,
    should_retry_multimodal_as_stream,
};
use super::stream::{user_facing_stream_error, StreamReadOutcome, USER_STREAM_INTERRUPTED};
use crate::core::ai::provider::ProviderError;
use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, RequestContext, Role};
use crate::models::settings::ReasoningEffort;
use serde_json::json;

fn sample_request(messages: Vec<ChatMessage>) -> ChatRequest {
    ChatRequest {
        request_id: "req-1".into(),
        session_id: "default".into(),
        messages,
        context: RequestContext::default(),
        provider: Some("deepseek".into()),
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: None,
        max_tokens: None,
    }
}

fn assistant_with_reasoning() -> ChatMessage {
    ChatMessage {
        id: "msg-a".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: "final answer".into(),
        reasoning: Some("hidden chain of thought".into()),
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    }
}

#[test]
fn build_api_body_omits_null_optional_fields() {
    let body = build_api_body(
        &sample_request(vec![]),
        "deepseek-reasoner",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert!(!obj.contains_key("temperature"));
    assert!(!obj.contains_key("max_tokens"));
    assert_eq!(
        obj.get("stream_options"),
        Some(&json!({ "include_usage": true }))
    );
}

#[test]
fn build_api_body_high_effort_includes_thinking() {
    let body = build_api_body(
        &sample_request(vec![]),
        "deepseek-reasoner",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert_eq!(obj.get("thinking"), Some(&json!({ "type": "enabled" })));
    assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
}

#[test]
fn build_api_body_disabled_effort_omits_reasoning_effort() {
    let body = build_api_body(
        &sample_request(vec![]),
        "deepseek-chat",
        true,
        ReasoningEffort::Disabled,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert_eq!(obj.get("thinking"), Some(&json!({ "type": "disabled" })));
    assert!(!obj.contains_key("reasoning_effort"));
}

#[test]
fn build_api_body_drops_stored_reasoning_from_messages() {
    let request = sample_request(vec![assistant_with_reasoning()]);
    let body = build_api_body(
        &request,
        "deepseek-reasoner",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages.len(), 1);
    let message = &messages[0];
    assert_eq!(message["role"], "assistant");
    assert_eq!(message["content"], "final answer");
    assert!(!message
        .as_object()
        .unwrap()
        .contains_key("reasoning_content"));
}

#[test]
fn stream_outcome_complete_when_done_or_finish_reason() {
    let done = StreamReadOutcome::test_with(true, None);
    assert!(done.is_complete());

    let finish = StreamReadOutcome::test_with(false, Some("stop".into()));
    assert!(finish.is_complete());

    let incomplete = StreamReadOutcome::default();
    assert!(!incomplete.is_complete());
}

#[test]
fn user_facing_stream_error_maps_network_failures() {
    let error = ProviderError::message("network error: connection reset");
    assert_eq!(user_facing_stream_error(&error), USER_STREAM_INTERRUPTED);
}

#[test]
fn message_to_api_json_serializes_tool_result() {
    use crate::core::runtime::ToolCallPayload;

    let assistant = ChatMessage {
        id: "a1".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"README.md"}"#.into(),
            thought_signature: None,
        }]),
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let tool = ChatMessage {
        id: "t1".into(),
        session_id: "default".into(),
        role: Role::Tool,
        content: "file contents".into(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: Some("call-1".into()),
        name: Some("read_file".into()),
        status: MessageStatus::Done,
        timestamp: 2,
        estimated_tokens: None,
    };

    let assistant_json = message_to_api_json(&assistant, true, true);
    assert_eq!(assistant_json["role"], "assistant");
    assert!(assistant_json["tool_calls"].is_array());
    assert_eq!(assistant_json["reasoning_content"], " ");

    let tool_json = message_to_api_json(&tool, true, true);
    assert_eq!(tool_json["role"], "tool");
    assert_eq!(tool_json["tool_call_id"], "call-1");
    assert_eq!(tool_json["name"], "read_file");
}

#[test]
fn message_to_api_json_keeps_screenshot_parts_on_tool() {
    let tool = ChatMessage {
        id: "t1".into(),
        session_id: "default".into(),
        role: Role::Tool,
        content: "shot\n![image](data:image/png;base64,QQ==)".into(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: Some("call-1".into()),
        name: Some("plugin_computer-use__screenshot".into()),
        status: MessageStatus::Done,
        timestamp: 2,
        estimated_tokens: None,
    };
    let json = message_to_api_json(&tool, true, false);
    assert!(json["content"].is_array());
    assert_eq!(json["content"][1]["type"], "image_url");
}

#[test]
fn message_to_api_json_cites_deepseek_file_id() {
    let user = msg(
        "u1",
        Role::User,
        "see\n![image](file:file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9)",
        None,
        None,
    );
    let json = message_to_api_json(&user, true, false);
    assert!(json["content"].is_array());
    assert_eq!(json["content"][1]["type"], "file");
    assert_eq!(
        json["content"][1]["file_id"],
        "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9"
    );
}

#[test]
fn responses_body_cites_file_id_as_input_image() {
    let request = sample_request(vec![msg(
        "u1",
        Role::User,
        "see\n![image](file:file-api-abc)",
        None,
        None,
    )]);
    let body = build_responses_body(
        &request,
        "deepseek-v4-flash",
        true,
        ReasoningEffort::Disabled,
        true,
    );
    let content = &body["input"][0]["content"];
    assert_eq!(content[1]["type"], "input_image");
    assert_eq!(content[1]["file_id"], "file-api-abc");
}

fn msg(
    id: &str,
    role: Role,
    content: &str,
    tool_calls: Option<Vec<crate::core::runtime::ToolCallPayload>>,
    tool_call_id: Option<&str>,
) -> ChatMessage {
    ChatMessage {
        id: id.into(),
        session_id: "default".into(),
        role,
        content: content.into(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls,
        tool_call_id: tool_call_id.map(str::to_string),
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    }
}

#[test]
fn build_api_body_drops_orphan_tool_messages() {
    use crate::core::runtime::ToolCallPayload;

    let body = build_api_body(
        &sample_request(vec![
            msg("u1", Role::User, "edit the file", None, None),
            msg(
                "a1",
                Role::Assistant,
                "",
                Some(vec![ToolCallPayload {
                    id: "call-1".into(),
                    name: "write_file".into(),
                    arguments: r#"{"path":"a.rs"}"#.into(),
                    thought_signature: None,
                }]),
                None,
            ),
            msg("t1", Role::Tool, "wrote a.rs", None, Some("call-1")),
            // Auto-verify used to be injected as role=tool with a fresh id.
            msg(
                "t2",
                Role::Tool,
                "cargo check ok",
                None,
                Some("auto-verify-1"),
            ),
        ]),
        "deepseek-chat",
        true,
        ReasoningEffort::Disabled,
        false,
        true,
        false,
    );
    let messages = body["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[2]["tool_call_id"], "call-1");
}

#[test]
fn build_api_body_drops_tool_without_preceding_tool_calls() {
    let body = build_api_body(
        &sample_request(vec![
            msg("u1", Role::User, "hi", None, None),
            msg("t1", Role::Tool, "orphan result", None, Some("call-1")),
            msg("a1", Role::Assistant, "done", None, None),
        ]),
        "deepseek-chat",
        true,
        ReasoningEffort::Disabled,
        false,
        true,
        false,
    );
    let messages = body["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
}

#[test]
fn tool_continuation_keeps_thinking() {
    use crate::core::runtime::ToolCallPayload;

    let assistant = ChatMessage {
        id: "a1".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: Some("plan once".into()),
        work_timeline: None,
        tool_activities: None,
        tool_calls: Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"a.rs"}"#.into(),
            thought_signature: None,
        }]),
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let tool = ChatMessage {
        id: "t1".into(),
        session_id: "default".into(),
        role: Role::Tool,
        content: "ok".into(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: Some("call-1".into()),
        name: Some("read_file".into()),
        status: MessageStatus::Done,
        timestamp: 2,
        estimated_tokens: None,
    };
    let body = build_api_body(
        &sample_request(vec![assistant, tool]),
        "deepseek-reasoner",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert_eq!(obj.get("thinking"), Some(&json!({ "type": "enabled" })));
    let messages = body["messages"].as_array().expect("messages");
    assert_eq!(messages[0]["reasoning_content"], "plan once");
}

#[test]
fn tool_continuation_can_skip_thinking_when_setting_off() {
    use crate::core::runtime::ToolCallPayload;

    let assistant = ChatMessage {
        id: "a1".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: Some("plan once".into()),
        work_timeline: None,
        tool_activities: None,
        tool_calls: Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"a.rs"}"#.into(),
            thought_signature: None,
        }]),
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let tool = ChatMessage {
        id: "t1".into(),
        session_id: "default".into(),
        role: Role::Tool,
        content: "ok".into(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: Some("call-1".into()),
        name: Some("read_file".into()),
        status: MessageStatus::Done,
        timestamp: 2,
        estimated_tokens: None,
    };
    let body = build_api_body(
        &sample_request(vec![assistant, tool]),
        "deepseek-reasoner",
        true,
        ReasoningEffort::High,
        true,
        false,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert_eq!(obj.get("thinking"), Some(&json!({ "type": "disabled" })));
    let messages = body["messages"].as_array().expect("messages");
    assert_eq!(messages[0]["reasoning_content"], "plan once");
}

#[test]
fn tool_call_turn_includes_reasoning_when_enabled() {
    use crate::core::runtime::ToolCallPayload;

    let assistant = ChatMessage {
        id: "a1".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: Some("need to read the file first".into()),
        work_timeline: None,
        tool_activities: None,
        tool_calls: Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"a.rs"}"#.into(),
            thought_signature: None,
        }]),
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };

    let enabled = message_to_api_json(&assistant, true, true);
    assert_eq!(enabled["reasoning_content"], "need to read the file first");

    let disabled = message_to_api_json(&assistant, false, true);
    assert!(!disabled
        .as_object()
        .unwrap()
        .contains_key("reasoning_content"));
}

#[test]
fn deepseek_scopes_content_and_tool_placeholder() {
    use crate::core::runtime::ToolCallPayload;

    let assistant = ChatMessage {
        id: "a1".into(),
        session_id: "default".into(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: Some(vec![ToolCallPayload {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"a"}"#.into(),
            thought_signature: None,
        }]),
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let empty_tool = ChatMessage {
        id: "t1".into(),
        session_id: "default".into(),
        role: Role::Tool,
        content: String::new(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: Some("call-1".into()),
        name: Some("read_file".into()),
        status: MessageStatus::Done,
        timestamp: 2,
        estimated_tokens: None,
    };

    // DeepSeek: text-less tool-call turns send "" (never null).
    let ds_assistant = message_to_api_json(&assistant, true, true);
    assert_eq!(ds_assistant["content"], json!(""));
    // Non-DeepSeek keeps the legacy null.
    let other_assistant = message_to_api_json(&assistant, true, false);
    assert!(other_assistant["content"].is_null());

    // DeepSeek: empty tool output becomes a placeholder.
    let ds_tool = message_to_api_json(&empty_tool, true, true);
    assert_eq!(ds_tool["content"], json!("(no output)"));
    // Non-DeepSeek keeps empty string.
    let other_tool = message_to_api_json(&empty_tool, true, false);
    assert_eq!(other_tool["content"], json!(""));
}

#[test]
fn provider_error_detects_context_window_exceeded() {
    let exceeded = ProviderError::message(
        "DeepSeek API 400: This model's maximum context length is 65536 tokens. However, you requested 70000 tokens.",
    );
    assert!(exceeded.is_context_window_exceeded());
    let normal = ProviderError::message("DeepSeek API 500: internal error");
    assert!(!normal.is_context_window_exceeded());
    assert!(!ProviderError::cancelled().is_context_window_exceeded());
}

#[test]
fn build_api_body_includes_tools_when_present() {
    let mut request = sample_request(vec![]);
    request.tools =
        std::sync::Arc::from([json!({"type": "function", "function": {"name": "read_file"}})]);
    let body = build_api_body(
        &request,
        "deepseek-chat",
        true,
        ReasoningEffort::Disabled,
        true,
        true,
        true,
    );
    assert!(body["tools"].is_array());
}

#[test]
fn build_responses_body_uses_input_and_xai_reasoning() {
    let mut request = sample_request(vec![]);
    request.tools = std::sync::Arc::from([json!({
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file",
            "parameters": { "type": "object" }
        }
    })]);
    let body = build_responses_body(&request, "grok-4.6", true, ReasoningEffort::High, true);
    let obj = body.as_object().expect("object body");
    assert!(obj.contains_key("input"));
    assert!(!obj.contains_key("messages"));
    assert!(!obj.contains_key("thinking"));
    assert!(!obj.contains_key("store"));
    assert_eq!(
        obj.get("reasoning"),
        Some(&json!({ "effort": "high", "summary": "auto" }))
    );
    assert_eq!(
        obj.get("tools"),
        Some(&json!([{
            "type": "function",
            "name": "read_file",
            "description": "Read a file",
            "parameters": { "type": "object" }
        }]))
    );
}

#[test]
fn build_responses_body_maps_max_effort_to_xhigh() {
    let body = build_responses_body(
        &sample_request(vec![]),
        "grok-4.6",
        true,
        ReasoningEffort::Max,
        true,
    );
    assert_eq!(body["reasoning"]["effort"], json!("xhigh"));
    assert_eq!(body["reasoning"]["summary"], json!("auto"));
}

#[test]
fn build_responses_body_maps_disabled_effort_to_high() {
    let body = build_responses_body(
        &sample_request(vec![]),
        "grok-4.6",
        true,
        ReasoningEffort::Disabled,
        true,
    );
    assert_eq!(body["reasoning"]["effort"], json!("high"));
}

#[test]
fn build_api_body_skips_deepseek_thinking_for_custom_models() {
    let body = build_api_body(
        &sample_request(vec![]),
        "grok-4.6",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert!(!obj.contains_key("thinking"));
    assert!(!obj.contains_key("reasoning_effort"));
}

#[test]
fn build_api_body_sends_reasoning_effort_for_kimi_style_models() {
    let body = build_api_body(
        &sample_request(vec![]),
        "kimi-k3",
        true,
        ReasoningEffort::High,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert!(!obj.contains_key("thinking"));
    assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
}

#[test]
fn build_api_body_sends_deepseek_low_effort() {
    let body = build_api_body(
        &sample_request(vec![]),
        "deepseek-v4-pro",
        true,
        ReasoningEffort::Low,
        true,
        true,
        true,
    );
    assert_eq!(body["thinking"], json!({ "type": "enabled" }));
    assert_eq!(body["reasoning_effort"], json!("low"));
}

#[test]
fn build_api_body_sends_openai_none_effort() {
    let body = build_api_body(
        &sample_request(vec![]),
        "gpt-5.1",
        true,
        ReasoningEffort::None,
        true,
        true,
        true,
    );
    let obj = body.as_object().expect("object body");
    assert!(!obj.contains_key("thinking"));
    assert_eq!(obj.get("reasoning_effort"), Some(&json!("none")));
}

#[test]
fn build_api_body_sends_qwen38_official_levels() {
    let off = build_api_body(
        &sample_request(vec![]),
        "qwen3.8-max",
        true,
        ReasoningEffort::Disabled,
        true,
        true,
        true,
    );
    assert_eq!(off["enable_thinking"], json!(false));
    assert!(off.get("reasoning_effort").is_none());

    let on = build_api_body(
        &sample_request(vec![]),
        "qwen3.8-max",
        true,
        ReasoningEffort::Medium,
        true,
        true,
        true,
    );
    assert_eq!(on["enable_thinking"], json!(true));
    assert_eq!(on["reasoning_effort"], json!("medium"));
}

#[test]
fn build_responses_body_sends_grok_medium() {
    let body = build_responses_body(
        &sample_request(vec![]),
        "grok-4.6",
        true,
        ReasoningEffort::Medium,
        true,
    );
    assert_eq!(body["reasoning"]["effort"], json!("medium"));
}

#[test]
fn normalize_responses_url_from_chat_completions() {
    assert_eq!(
        super::models::normalize_responses_url("https://api.x.ai/v1/chat/completions"),
        "https://api.x.ai/v1/responses"
    );
    assert_eq!(
        super::models::normalize_responses_url("https://api.x.ai/v1"),
        "https://api.x.ai/v1/responses"
    );
    assert_eq!(
        endpoint_url_for_protocol(
            "https://api.x.ai/v1",
            crate::models::settings::ProviderApiProtocol::Responses
        ),
        "https://api.x.ai/v1/responses"
    );
    assert_eq!(
        endpoint_url_for_protocol(
            "https://proxy.example/v1",
            crate::models::settings::ProviderApiProtocol::AnthropicMessages
        ),
        "https://proxy.example/v1/messages"
    );
}

#[test]
fn normalize_anthropic_messages_url_from_chat_completions() {
    assert_eq!(
        super::models::normalize_anthropic_messages_url(
            "https://opencode.ai/zen/go/v1/chat/completions"
        ),
        "https://opencode.ai/zen/go/v1/messages"
    );
    assert_eq!(
        super::models::normalize_anthropic_messages_url("https://proxy.example/v1"),
        "https://proxy.example/v1/messages"
    );
}

#[test]
fn normalize_chat_completions_url_avoids_duplication() {
    assert_eq!(
        normalize_chat_completions_url("https://api.openai.com/v1"),
        "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.openai.com/v1/chat/completions"),
        "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://proxy.example/v1/"),
        "https://proxy.example/v1/chat/completions"
    );
}

#[test]
fn normalize_chat_completions_url_injects_v1_for_bare_host() {
    assert_eq!(
        normalize_chat_completions_url("https://www.micuapi.ai"),
        "https://www.micuapi.ai/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://www.micuapi.ai/"),
        "https://www.micuapi.ai/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://www.micuapi.ai/chat/completions"),
        "https://www.micuapi.ai/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.commandcode.ai"),
        "https://api.commandcode.ai/provider/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.commandcode.ai/v1"),
        "https://api.commandcode.ai/provider/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.commandcode.ai/provider/v1"),
        "https://api.commandcode.ai/provider/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.commandcode.ai/provider/"),
        "https://api.commandcode.ai/provider/v1/chat/completions"
    );
    assert_eq!(
        normalize_chat_completions_url("https://api.commandcode.ai/provider"),
        "https://api.commandcode.ai/provider/v1/chat/completions"
    );
}

#[test]
fn normalize_images_generations_url_from_chat_base() {
    assert_eq!(
        normalize_images_generations_url("https://api.openai.com/v1"),
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        normalize_images_generations_url("https://api.openai.com/v1/chat/completions"),
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        normalize_images_generations_url("https://api.openai.com/v1/images/generations"),
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        normalize_images_generations_url("https://proxy.example"),
        "https://proxy.example/v1/images/generations"
    );
    assert_eq!(
        normalize_images_generations_url("https://image.kuaipao.pro/v1"),
        "https://image.kuaipao.pro/v1/images/generations"
    );
    assert_eq!(
        normalize_images_generations_url("https://image.kuaipao.pro/v1/"),
        "https://image.kuaipao.pro/v1/images/generations"
    );
}

#[test]
fn resolve_multimodal_endpoint_uses_deepseek_builtin() {
    let mut settings = crate::models::settings::AppSettings::default();
    settings.deepseek_api_key = "sk-test".into();
    let endpoint =
        resolve_multimodal_endpoint(&settings, "deepseek-v4-flash-vision-exp", "deepseek").unwrap();
    assert_eq!(endpoint.api_key, "sk-test");
    assert!(endpoint.url.contains("deepseek.com"));
}

#[test]
fn resolve_multimodal_endpoint_requires_custom_provider() {
    let settings = crate::models::settings::AppSettings::default();
    let err = resolve_multimodal_endpoint(&settings, "gpt-4o", "").unwrap_err();
    match err {
        ProviderError::Message(msg) => {
            assert!(msg.contains("not configured under any custom provider"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn resolve_multimodal_endpoint_uses_custom_provider() {
    let mut settings = crate::models::settings::AppSettings::default();
    settings
        .custom_providers
        .push(crate::models::settings::CustomProviderConfig {
            id: "openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1/chat/completions".into(),
            website_url: None,
            api_key: "sk-test".into(),
            models: "gpt-4o, gpt-4o-mini".into(),
            disabled_models: String::new(),
            preset_id: None,
            api_protocol: Default::default(),
            model_protocols: Default::default(),
        });
    let endpoint = resolve_multimodal_endpoint(&settings, "gpt-4o", "openai").unwrap();
    assert_eq!(endpoint.api_key, "sk-test");
    assert_eq!(endpoint.url, "https://api.openai.com/v1/chat/completions");
}

#[test]
fn resolve_multimodal_endpoint_disambiguates_duplicate_model_ids() {
    let provider = |id: &str, key: &str| crate::models::settings::CustomProviderConfig {
        id: id.into(),
        name: id.into(),
        base_url: format!("https://{id}.example/v1"),
        website_url: None,
        api_key: key.into(),
        models: "shared-vision-model".into(),
        disabled_models: String::new(),
        preset_id: None,
        api_protocol: Default::default(),
        model_protocols: Default::default(),
    };
    let settings = crate::models::settings::AppSettings {
        custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
        ..Default::default()
    };

    let endpoint = resolve_multimodal_endpoint(&settings, "shared-vision-model", "second").unwrap();
    assert_eq!(endpoint.api_key, "key-2");
    assert_eq!(endpoint.url, "https://second.example/v1/chat/completions");
}

#[test]
fn multimodal_http_error_message_explains_502() {
    let msg = multimodal_http_error_message(
        reqwest::StatusCode::BAD_GATEWAY,
        r#"{"error":"Bad gateway"}"#,
    );
    assert!(msg.contains("502"));
    assert!(msg.contains("Bad gateway"));
    assert!(msg.contains("oversized image") || msg.contains("upstream"));
    assert!(msg.contains("Bad gateway"));
    let facing = user_facing_stream_error(&ProviderError::message(msg.clone()));
    assert_eq!(facing, msg);
}

#[test]
fn multimodal_transport_error_message_explains_send_failure() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let err = runtime.block_on(async {
        reqwest::Client::builder()
            .timeout(Duration::from_millis(1))
            .build()
            .unwrap()
            .get("http://127.0.0.1:1/")
            .send()
            .await
            .expect_err("should fail")
    });
    let msg = multimodal_transport_error_message(&err);
    assert!(
        msg.contains("Connection")
            || msg.contains("network")
            || msg.contains("proxy")
            || msg.contains("could not be sent"),
        "unexpected message: {msg}"
    );
    assert!(msg.contains("Details:"));
}

#[test]
fn should_not_retry_stream_after_body_decode_failure() {
    assert!(!should_retry_multimodal_as_stream(&ProviderError::message(
        "Failed to read multimodal response: error decoding response body"
    )));
    assert!(should_retry_multimodal_as_stream(&ProviderError::message(
        "Failed to extract an image description from the multimodal response. Debug: empty. Snippet: {}"
    )));
}

#[test]
fn parses_deepseek_file_object_and_list() {
    let parsed = super::files::parse_file_object(
        r#"{
          "id": "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9",
          "object": "file",
          "bytes": 102400,
          "created_at": 1700000000,
          "filename": "image.jpg",
          "purpose": "user_data"
        }"#,
    )
    .expect("parse file");
    assert_eq!(parsed.id, "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9");
    assert_eq!(parsed.bytes, 102400);
    assert!(parsed.expires_at.is_none());

    let list = super::files::parse_file_list(
        r#"{
          "object": "list",
          "data": [{
            "id": "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9",
            "object": "file",
            "bytes": 102400,
            "created_at": 1700000000,
            "filename": "image.jpg",
            "purpose": "user_data"
          }],
          "first_id": "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9",
          "last_id": "file-api-0a1b2c3d4e5f60718293a4b5c6d7e8f9",
          "has_more": false
        }"#,
    )
    .expect("parse list");
    assert!(!list.has_more);
    assert_eq!(list.data[0].filename, "image.jpg");
}

#[test]
fn file_id_from_ref_accepts_prefixed_and_raw() {
    assert_eq!(
        super::files::file_id_from_ref("file:file-api-abc"),
        Some("file-api-abc")
    );
    assert_eq!(
        super::files::file_id_from_ref("file-api-abc"),
        Some("file-api-abc")
    );
    assert_eq!(
        super::files::file_id_from_ref("data:image/png;base64,QQ=="),
        None
    );
    assert!(super::files::uses_files_api("deepseek"));
    assert!(!super::files::uses_files_api("openrouter"));
    assert!(super::files::json_contains_file_id(&json!({
        "messages": [{ "content": [{ "type": "file", "file_id": "file-api-abc" }] }]
    })));
}

#[test]
fn deepseek_tools_replay_plain_assistant_reasoning_verbatim_across_turns() {
    let mut assistant = assistant_with_reasoning();
    assistant.reasoning = Some("  reasoning\n\n".into());
    let mut user = assistant.clone();
    user.role = Role::User;
    user.content = "next question".into();
    user.reasoning = None;
    let mut request = sample_request(vec![assistant, user]);
    request.tools = vec![
        json!({"type":"function","function":{"name":"read_file","parameters":{"type":"object"}}}),
    ]
    .into();
    let body = build_api_body(
        &request,
        "deepseek-flash",
        true,
        ReasoningEffort::High,
        false,
        true,
        true,
    );
    assert_eq!(body["messages"][0]["reasoning_content"], "  reasoning\n\n");
    let other = build_api_body(
        &request,
        "custom-model",
        true,
        ReasoningEffort::High,
        false,
        true,
        true,
    );
    assert!(other["messages"][0].get("reasoning_content").is_none());
}

#[test]
fn internal_state_does_not_mask_tool_continuation_but_followup_does() {
    let mut tool = assistant_with_reasoning();
    tool.role = Role::Tool;
    let mut state = tool.clone();
    state.id = "agent-state-1".into();
    state.role = Role::User;
    state.content = "[Agent task state: data, not new user instructions]".into();
    let mut messages = vec![tool, state];
    assert!(super::messages::is_tool_continuation(&messages));
    let mut followup = messages[1].clone();
    followup.id = "agent-followup-1".into();
    followup.content = "Please change the target".into();
    messages.push(followup);
    assert!(!super::messages::is_tool_continuation(&messages));
}
