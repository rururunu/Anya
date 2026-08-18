//! DeepSeek and OpenAI-compatible chat provider.

mod image_fallback;
mod messages;
mod models;
mod multimodal;
mod stream;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tracing::Instrument;

use crate::core::runtime::{ChatRequest, Role, StreamEvent};
use crate::models::settings::{ProviderApiProtocol, ReasoningEffort};

use super::provider::{AIProvider, ProviderError};

pub(crate) use messages::build_api_body;
pub(crate) use messages::build_responses_body;
pub use models::list_models;
pub(crate) use models::{endpoint_url_for_protocol, normalize_chat_completions_url};
pub use models::{list_openai_compatible_models, normalize_models_url};
use stream::RETRY_BACKOFF;

use image_fallback::{apply_image_input_fallback, FallbackPlan};
use stream::{emit_stream_error, run_chat_stream, run_responses_stream};

const API_URL: &str = "https://api.deepseek.com/chat/completions";

pub struct DeepSeekProvider {
    app: tauri::AppHandle,
    resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
    resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
    resolve_continue_thinking_after_tools: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Optional resolver that returns a custom chat-completions URL.
    /// When `None` (or the resolver returns `None`) the default `API_URL` is used.
    resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    resolve_api_protocol: Arc<dyn Fn() -> ProviderApiProtocol + Send + Sync>,
}

impl DeepSeekProvider {
    pub fn new(
        app: tauri::AppHandle,
        resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
        resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
        resolve_continue_thinking_after_tools: Arc<dyn Fn() -> bool + Send + Sync>,
        resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
        resolve_api_protocol: Arc<dyn Fn() -> ProviderApiProtocol + Send + Sync>,
    ) -> Self {
        Self {
            app,
            resolve_api_key,
            resolve_model,
            resolve_effort,
            resolve_pass_tool_reasoning,
            resolve_continue_thinking_after_tools,
            resolve_base_url,
            resolve_api_protocol,
        }
    }

    fn api_key(&self) -> Result<String, ProviderError> {
        let api_key = (self.resolve_api_key)();
        if api_key.trim().is_empty() {
            return Err(ProviderError::message(
                "Model credentials are not configured. Sign in to Gemini (Antigravity) or enter an API Key in Settings.",
            ));
        }
        Ok(api_key.trim().to_string())
    }

    fn model(&self) -> Result<String, ProviderError> {
        let model = (self.resolve_model)();
        let trimmed = model.trim();
        if trimmed.is_empty() {
            return Err(ProviderError::message(
                "No model selected. Configure a provider and choose a model in Settings first.",
            ));
        }
        Ok(trimmed.to_string())
    }

    fn effort(&self) -> ReasoningEffort {
        (self.resolve_effort)()
    }

    fn pass_tool_reasoning(&self) -> bool {
        (self.resolve_pass_tool_reasoning)()
    }

    fn continue_thinking_after_tools(&self) -> bool {
        (self.resolve_continue_thinking_after_tools)()
    }

    fn api_protocol(&self) -> ProviderApiProtocol {
        (self.resolve_api_protocol)()
    }

    fn request_url(&self, protocol: ProviderApiProtocol) -> String {
        if let Some(resolver) = &self.resolve_base_url {
            if let Some(base) = resolver() {
                return endpoint_url_for_protocol(&base, protocol);
            }
        }
        API_URL.to_string()
    }
}

#[async_trait]
impl AIProvider for DeepSeekProvider {
    fn id(&self) -> &'static str {
        "deepseek"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let span = tracing::info_span!(
            target: "peek.provider",
            "provider_stream",
            provider = "deepseek",
            session_id = %request.session_id,
            request_id = %request.request_id,
        );
        self.stream_inner(request, tx).instrument(span).await
    }
}

impl DeepSeekProvider {
    async fn stream_inner(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let settings = crate::services::settings_store::get_settings(&self.app).unwrap_or_default();
        let mut request = request;
        let primary_model = self.model()?;
        let primary_api_key = self.api_key()?;
        let primary_protocol = self.api_protocol();
        let primary_url = self.request_url(primary_protocol);
        let effort = self.effort();
        let pass_tool_reasoning = self.pass_tool_reasoning();
        let continue_thinking_after_tools = self.continue_thinking_after_tools();
        let include_thinking = !primary_url.contains("generativelanguage.googleapis.com");

        let has_images = request
            .messages
            .iter()
            .any(|msg| msg.role == Role::User && msg.content.contains("![image]("));

        let _ = tx.send(StreamEvent::Start).await;
        let client = reqwest::Client::new();

        let mut model = primary_model.clone();
        let mut api_key = primary_api_key.clone();
        let mut url = primary_url.clone();
        let mut protocol = primary_protocol;

        if has_images {
            match dispatch_stream(
                &client,
                &primary_url,
                &primary_api_key,
                &request,
                &primary_model,
                effort,
                pass_tool_reasoning,
                continue_thinking_after_tools,
                include_thinking,
                primary_protocol,
                &tx,
            )
            .await
            {
                Ok(()) => return Ok(()),
                // Multimodal split-analysis is only for text-only primaries.
                // Gemini / gpt-4o / Claude already see images natively — never describe→reask.
                Err(error)
                    if crate::core::ai::multimodal::is_vision_unsupported_error(&error)
                        && !crate::core::ai::multimodal::primary_model_has_native_vision(
                            &primary_model,
                        ) =>
                {
                    match apply_image_input_fallback(&mut request, &settings, &self.app, &tx).await
                    {
                        Ok(FallbackPlan::RetryPrimary) => {
                            model = primary_model;
                            api_key = primary_api_key;
                            url = primary_url;
                            protocol = primary_protocol;
                        }
                        Ok(FallbackPlan::SwitchToMultimodal {
                            model: mm_model,
                            api_key: mm_key,
                            url: mm_url,
                            protocol: mm_protocol,
                        }) => {
                            model = mm_model;
                            api_key = mm_key;
                            url = mm_url;
                            protocol = mm_protocol;
                        }
                        Err(error) => return emit_stream_error(&tx, error).await,
                    }
                }
                Err(error) => return emit_stream_error(&tx, error).await,
            }
        }

        // Only DeepSeek models get the reactive "context window exceeded →
        // compact and retry" path; custom OpenAI-compatible providers keep the
        // existing surface-error behavior.
        let is_deepseek = model.trim().to_ascii_lowercase().starts_with("deepseek");
        match dispatch_stream(
            &client,
            &url,
            &api_key,
            &request,
            &model,
            effort,
            pass_tool_reasoning,
            continue_thinking_after_tools,
            include_thinking,
            protocol,
            &tx,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(error) if is_deepseek && error.is_context_window_exceeded() => Err(error),
            Err(error) => emit_stream_error(&tx, error).await,
        }
    }
}

async fn dispatch_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    model: &str,
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    continue_thinking_after_tools: bool,
    include_thinking: bool,
    protocol: ProviderApiProtocol,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    match protocol {
        ProviderApiProtocol::Responses => {
            let body = build_responses_body(
                request,
                model,
                true,
                effort,
                continue_thinking_after_tools,
            );
            run_responses_stream(client, url, api_key, &body, tx).await
        }
        ProviderApiProtocol::ChatCompletions => {
            let body = build_api_body(
                request,
                model,
                true,
                effort,
                pass_tool_reasoning,
                continue_thinking_after_tools,
                include_thinking,
            );
            run_chat_stream(client, url, api_key, &body, tx).await
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role};
    use messages::message_to_api_json;
    use multimodal::{
        antigravity_model_for_image_describe, multimodal_http_error_message,
        multimodal_transport_error_message, resolve_multimodal_endpoint,
        should_retry_multimodal_as_stream,
    };
    use serde_json::json;
    use stream::{user_facing_stream_error, StreamReadOutcome, USER_STREAM_INTERRUPTED};

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
        let body = build_responses_body(
            &request,
            "grok-4.6",
            true,
            ReasoningEffort::High,
            true,
        );
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
                preset_id: None,
                api_protocol: Default::default(),
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
            preset_id: None,
            api_protocol: Default::default(),
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
}
