use std::time::Duration;

use super::messages::{build_api_body, build_responses_body, message_to_api_json};
use super::models::{
    endpoint_url_for_protocol, normalize_chat_completions_url, normalize_images_generations_url,
};
use super::multimodal::{
    antigravity_model_for_image_describe, multimodal_http_error_message,
    multimodal_transport_error_message, resolve_multimodal_endpoint,
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
    let endpoint = resolve_multimodal_endpoint(
        &settings,
        "deepseek-v4-flash-vision-exp",
        "deepseek",
    )
    .unwrap();
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

    let endpoint =
        resolve_multimodal_endpoint(&settings, "shared-vision-model", "second").unwrap();
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
fn antigravity_describe_model_only_when_multimodal_is_gemini() {
    let mut settings = crate::models::settings::AppSettings {
        chat_model: "gemini-3.5-flash-low".into(),
        multimodal_model: "gpt-4o".into(),
        ..Default::default()
    };
    settings.gemini_oauth.refresh_token = "rt".into();
    // Chat Gemini must not hijack multimodal describe — chat already sees images natively.
    assert!(antigravity_model_for_image_describe(&settings, "gpt-4o").is_none());
    assert_eq!(
        antigravity_model_for_image_describe(&settings, "gemini-3-flash").as_deref(),
        Some("gemini-3-flash")
    );

    settings.multimodal_model_provider = "custom-gemini".into();
    assert!(antigravity_model_for_image_describe(&settings, "gemini-3-flash").is_none());
}
