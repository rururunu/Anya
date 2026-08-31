//! DeepSeek provider construction, credentials, and stream dispatch.
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tracing::Instrument;

use crate::core::runtime::{ChatRequest, Role, StreamEvent};
use crate::models::settings::{ProviderApiProtocol, ReasoningEffort};

use crate::core::ai::provider::{AIProvider, ProviderError};
use super::anthropic::{
    build_anthropic_body, resolve_wire_protocol, url_for_wire_protocol, WireProtocol,
};
use super::image_fallback::{apply_image_input_fallback, FallbackPlan};
use super::messages::{build_api_body, build_responses_body};
use super::models::endpoint_url_for_protocol;
use super::stream::{emit_stream_error, run_anthropic_stream, run_chat_stream, run_responses_stream};

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
        let effort = self.effort();
        let pass_tool_reasoning = self.pass_tool_reasoning();
        let continue_thinking_after_tools = self.continue_thinking_after_tools();
        let include_thinking = !guess_url.contains("generativelanguage.googleapis.com");

        let has_images = request
            .messages
            .iter()
            .any(|msg| msg.role == Role::User && msg.content.contains("![image]("));

        let _ = tx.send(StreamEvent::Start).await;
        let client = reqwest::Client::new();

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

        // Only DeepSeek models get the reactive "context window exceeded →
        // compact and retry" path; custom OpenAI-compatible providers keep the
        // existing surface-error behavior.
        let is_deepseek = model.trim().to_ascii_lowercase().starts_with("deepseek");
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
            Err(error) if is_deepseek && error.is_context_window_exceeded() => Err(error),
            Err(error) => emit_stream_error(&tx, error).await,
        }
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
