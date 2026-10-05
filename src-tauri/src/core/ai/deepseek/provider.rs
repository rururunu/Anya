//! DeepSeek provider construction, credentials, and stream dispatch.
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tracing::Instrument;

use crate::core::runtime::{ChatRequest, StreamEvent};
use crate::models::settings::{ProviderApiProtocol, ReasoningEffort};

use super::anthropic::{
    build_anthropic_body, resolve_wire_protocol, url_for_wire_protocol, WireProtocol,
};
use super::image_fallback::{apply_image_input_fallback, FallbackPlan};
use super::messages::{build_api_body, build_responses_body};
use super::models::endpoint_url_for_protocol;
use super::stream::{
    emit_stream_error, run_anthropic_stream, run_chat_stream, run_responses_stream,
};
use crate::core::ai::provider::{AIProvider, ProviderError};

const API_URL: &str = "https://api.deepseek.com/chat/completions";

pub struct DeepSeekProvider {
    app: tauri::AppHandle,
    provider_id: String,
    resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
    resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
    resolve_continue_thinking_after_tools: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Optional resolver that returns a custom chat-completions URL.
    /// When `None` (or the resolver returns `None`) the default `API_URL` is used.
    resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    resolve_api_protocol: Arc<dyn Fn() -> ProviderApiProtocol + Send + Sync>,
    /// Returns `Some(reason)` when the resolved model/provider pair has been
    /// switched off in Settings — the send is refused before any network call.
    resolve_blocked_reason: Arc<dyn Fn() -> Option<String> + Send + Sync>,
}

impl DeepSeekProvider {
    pub fn new(
        app: tauri::AppHandle,
        provider_id: String,
        resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
        resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
        resolve_continue_thinking_after_tools: Arc<dyn Fn() -> bool + Send + Sync>,
        resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
        resolve_api_protocol: Arc<dyn Fn() -> ProviderApiProtocol + Send + Sync>,
        resolve_blocked_reason: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> Self {
        Self {
            app,
            provider_id,
            resolve_api_key,
            resolve_model,
            resolve_effort,
            resolve_pass_tool_reasoning,
            resolve_continue_thinking_after_tools,
            resolve_base_url,
            resolve_api_protocol,
            resolve_blocked_reason,
        }
    }

    fn api_key(&self) -> Result<String, ProviderError> {
        let api_key = (self.resolve_api_key)();
        if api_key.trim().is_empty() {
            let model = (self.resolve_model)();
            let model_trimmed = model.trim();
            if self.provider_id == "deepseek" {
                return Err(ProviderError::message(
                    "Model credentials are not configured. Enter a DeepSeek API Key or configure the provider in Settings.",
                ));
            }
            let target = if !model_trimmed.is_empty() {
                format!("for '{model_trimmed}'")
            } else {
                format!("for provider '{}'", self.provider_id)
            };
            return Err(ProviderError::message(format!(
                "Model credentials are not configured. Enter an API Key {target} or configure the provider in Settings.",
            )));
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
        if let Some(reason) = (self.resolve_blocked_reason)() {
            return Err(ProviderError::message(reason));
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

    fn provider_base_url(&self) -> Option<String> {
        self.resolve_base_url
            .as_ref()
            .and_then(|resolver| resolver())
    }

    fn request_url(&self, protocol: ProviderApiProtocol) -> String {
        if let Some(base) = self.provider_base_url() {
            return endpoint_url_for_protocol(&base, protocol);
        }
        API_URL.to_string()
    }
}

#[async_trait]
impl AIProvider for DeepSeekProvider {
    fn id(&self) -> &'static str {
        "deepseek"
    }

    fn uses_dsh_tools(&self) -> bool {
        crate::core::ai::registry::looks_like_deepseek_model(&(self.resolve_model)())
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
        let mut endpoint_base = self.provider_base_url();
        let guess_url = endpoint_base
            .clone()
            .unwrap_or_else(|| self.request_url(primary_protocol));
        let cached = crate::core::ai::registry::cached_model_protocol(
            &settings,
            &self.provider_id,
            &primary_model,
        )
        .map(WireProtocol::from);
        let primary_wire = resolve_wire_protocol(primary_protocol, cached);
        let configured_policy = std::env::var("ANYA_DEEPSEEK_REASONING_POLICY").ok().as_deref() == Some("configured");
        let effort = if self.uses_dsh_tools() && !configured_policy { dsh_task_effort(&request, self.effort()) } else { self.effort() };
        if self.uses_dsh_tools() {
            let task_class = if request.request_id.starts_with("title-") { "title" }
                else if request.request_id.starts_with("compact-") { "summary" }
                else if dsh_task_effort(&request, ReasoningEffort::High) == ReasoningEffort::Low { "simple_question" }
                else { "configured_task" };
            crate::core::chat::telemetry::record_provider_metrics(&serde_json::json!({
                "kind":"reasoning_policy", "model":primary_model, "task_class":task_class,
                "configured_effort":self.effort(), "selected_effort":effort,
                "policy_mode":if configured_policy { "configured" } else { "adaptive" },
                "policy_version":1, "timestamp":chrono::Utc::now().to_rfc3339()
            }));
        }
        let pass_tool_reasoning = self.pass_tool_reasoning();
        let continue_thinking_after_tools = self.continue_thinking_after_tools();
        let include_thinking = !guess_url.contains("generativelanguage.googleapis.com");

        let has_images = request
            .messages
            .iter()
            .any(|msg| msg.content.contains("![image]("));
        let restore_original = super::files::uses_files_api(&self.provider_id) && has_images;
        let original_request = restore_original.then(|| request.clone());
        if restore_original {
            super::files::attach_files_to_messages(&primary_api_key, &mut request.messages).await;
        }

        let _ = tx.send(StreamEvent::Start).await;
        // Bound connection/header/body silence, without imposing a total
        // duration limit on a response that keeps streaming.
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|error| ProviderError::message(format!("network error: {error}")))?;

        let mut model = primary_model.clone();
        let mut api_key = primary_api_key.clone();
        let mut protocol = primary_wire;

        if has_images {
            match dispatch_configured_protocol(
                &client,
                &request,
                &primary_model,
                &primary_api_key,
                effort,
                pass_tool_reasoning,
                continue_thinking_after_tools,
                include_thinking,
                primary_wire,
                endpoint_base.as_deref(),
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
                    if let Some(original) = original_request {
                        request = original;
                    }
                    match apply_image_input_fallback(&mut request, &settings, &self.app, &tx).await
                    {
                        Ok(FallbackPlan::RetryPrimary) => {
                            model = primary_model;
                            api_key = primary_api_key;
                            protocol = primary_wire;
                            endpoint_base = self.provider_base_url();
                        }
                        Ok(FallbackPlan::SwitchToMultimodal {
                            model: mm_model,
                            api_key: mm_key,
                            url: mm_url,
                            protocol: mm_protocol,
                        }) => {
                            model = mm_model;
                            api_key = mm_key;
                            endpoint_base = Some(mm_url.clone());
                            let mm_cached = crate::core::ai::registry::cached_model_protocol(
                                &settings,
                                &self.provider_id,
                                &model,
                            )
                            .map(WireProtocol::from);
                            protocol = resolve_wire_protocol(mm_protocol, mm_cached);
                        }
                        Err(error) => return emit_stream_error(&tx, error).await,
                    }
                }
                Err(error) => return emit_stream_error(&tx, error).await,
            }
        }

        // Leave context errors to the agent's compaction recovery for every model.
        match dispatch_configured_protocol(
            &client,
            &request,
            &model,
            &api_key,
            effort,
            pass_tool_reasoning,
            continue_thinking_after_tools,
            include_thinking,
            protocol,
            endpoint_base.as_deref(),
            &tx,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(error) if error.is_context_window_exceeded() => Err(error),
            Err(error) => emit_stream_error(&tx, error).await,
        }
    }
}

fn dsh_task_effort(request: &ChatRequest, configured: ReasoningEffort) -> ReasoningEffort {
    let question = request.messages.iter().rev().find(|m| m.role == crate::core::runtime::Role::User)
        .map(|m| m.content.trim()).unwrap_or("");
    dsh_task_effort_for(&request.request_id, question, configured)
}

fn dsh_task_effort_for(request_id: &str, question: &str, configured: ReasoningEffort) -> ReasoningEffort {
    if request_id.starts_with("title-") || request_id.starts_with("compact-") { return ReasoningEffort::Disabled; }
    let lower = question.to_ascii_lowercase();
    let complex = ["project", "code", "function", "debug", "review", "analy", "prove", "test", "why", "how", "this", "that", "项目", "代码", "函数", "报错", "分析", "证明", "推导", "为什么", "如何", "上面", "刚才", "之前", "http", "\\", "```"].iter().any(|term| lower.contains(term));
    if question.chars().count() <= 120 && !complex && crate::runtime::tool::is_question_only_text(question)
        && matches!(configured, ReasoningEffort::Medium | ReasoningEffort::High | ReasoningEffort::Xhigh | ReasoningEffort::Max) {
        return ReasoningEffort::Low;
    }
    configured
}

#[cfg(test)]
mod task_effort_tests {
    use super::*;
    #[test]
    fn auxiliary_and_simple_tasks_save_reasoning_without_lowering_complex_work() {
        assert_eq!(dsh_task_effort_for("title-1", "Title", ReasoningEffort::High), ReasoningEffort::Disabled);
        assert_eq!(dsh_task_effort_for("compact-1", "Summarize", ReasoningEffort::Max), ReasoningEffort::Disabled);
        assert_eq!(dsh_task_effort_for("r", "What is Rust?", ReasoningEffort::High), ReasoningEffort::Low);
        assert_eq!(dsh_task_effort_for("r", "为什么这个项目的测试失败？", ReasoningEffort::High), ReasoningEffort::High);
        assert_eq!(dsh_task_effort_for("r", "为我修复这个错误", ReasoningEffort::Max), ReasoningEffort::Max);
        assert_eq!(dsh_task_effort_for("r", "What is Rust?", ReasoningEffort::Disabled), ReasoningEffort::Disabled);
        assert_eq!(dsh_task_effort_for("r", "What is Rust?\n\nAnalyze this project code", ReasoningEffort::High), ReasoningEffort::High);
    }
}

async fn dispatch_configured_protocol(
    client: &reqwest::Client,
    request: &ChatRequest,
    model: &str,
    api_key: &str,
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    continue_thinking_after_tools: bool,
    include_thinking: bool,
    protocol: WireProtocol,
    base_url: Option<&str>,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let url = url_for_wire_protocol(base_url, protocol);
    dispatch_stream(
        client,
        &url,
        api_key,
        request,
        model,
        effort,
        pass_tool_reasoning,
        continue_thinking_after_tools,
        include_thinking,
        protocol,
        tx,
    )
    .await
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
    protocol: WireProtocol,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    match protocol {
        WireProtocol::Responses => {
            let body =
                build_responses_body(request, model, true, effort, continue_thinking_after_tools);
            run_responses_stream(client, url, api_key, &body, tx).await
        }
        WireProtocol::AnthropicMessages => {
            let body =
                build_anthropic_body(request, model, true, effort, continue_thinking_after_tools);
            run_anthropic_stream(client, url, api_key, &body, tx).await
        }
        WireProtocol::ChatCompletions => {
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
