use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError, ProviderTaskGuard};
use crate::core::runtime::{ChatMessage, ChatRequest, Role, StreamEvent, ToolCallPayload};

use super::{TokenAccuracy, TokenCategory, TokenUsage, TokenizerRegistry};

// Also end calls cancelled by dropping the stream future, before normal usage
// collection can finish. A process crash can still leave a start-only record.
struct MetricsEndGuard {
    started: std::time::Instant,
    identity: Option<serde_json::Value>,
}
impl Drop for MetricsEndGuard {
    fn drop(&mut self) {
        if let Some(mut value) = self.identity.take() {
            value["kind"] = serde_json::json!("model_call");
            value["timestamp"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
            value["duration_ms"] = serde_json::json!(self.started.elapsed().as_millis());
            value["succeeded"] = serde_json::json!(false);
            value["outcome"] = serde_json::json!("interrupted");
            crate::core::chat::telemetry::record_provider_metrics(&value);
        }
    }
}

pub struct TokenAccountant {
    registry: Arc<TokenizerRegistry>,
}

impl Default for TokenAccountant {
    fn default() -> Self {
        Self::new(Arc::new(TokenizerRegistry::default()))
    }
}

impl TokenAccountant {
    pub fn new(registry: Arc<TokenizerRegistry>) -> Self {
        Self { registry }
    }

    pub fn count_request(&self, model: &str, provider: &str, request: &ChatRequest) -> TokenUsage {
        let selection = self.registry.resolve(model, provider);
        let _matched_by = selection.matched_by;
        let mut usage = TokenUsage {
            accuracy: selection.tokenizer.count("").accuracy,
            source: Some(selection.tokenizer.name().to_string()),
            ..TokenUsage::default()
        };

        for message in &request.messages {
            let tokens = count_message(selection.tokenizer.as_ref(), message);
            usage.add_category(TokenCategory::Input, tokens);
            match message.role {
                Role::System if message.id.starts_with("context-") => {
                    usage.add_category(TokenCategory::Context, tokens)
                }
                Role::System if message.id.starts_with("memories-") => {
                    usage.add_category(TokenCategory::Memory, tokens)
                }
                Role::System => usage.add_category(TokenCategory::System, tokens),
                Role::Tool => usage.add_category(TokenCategory::ToolResult, tokens),
                Role::Assistant
                    if message
                        .tool_calls
                        .as_ref()
                        .is_some_and(|calls| !calls.is_empty()) =>
                {
                    usage.add_category(
                        TokenCategory::ToolCall,
                        count_tool_calls(
                            selection.tokenizer.as_ref(),
                            message.tool_calls.as_deref().unwrap_or(&[]),
                        ),
                    )
                }
                _ => {}
            }
        }

        if !request.tools.is_empty() {
            let schemas = serde_json::to_string(request.tools.as_ref()).unwrap_or_default();
            let tokens = selection.tokenizer.count(&schemas).tokens;
            usage.add_category(TokenCategory::Input, tokens);
            usage.add_category(TokenCategory::ToolCall, tokens);
        }
        usage
    }

    pub fn count_output(
        &self,
        model: &str,
        provider: &str,
        content: &str,
        reasoning: &str,
        tool_calls: &[ToolCallPayload],
    ) -> TokenUsage {
        let selection = self.registry.resolve(model, provider);
        let _matched_by = selection.matched_by;
        let mut usage = TokenUsage {
            accuracy: selection.tokenizer.count("").accuracy,
            source: Some(selection.tokenizer.name().to_string()),
            ..TokenUsage::default()
        };
        let output = selection.tokenizer.count(content).tokens
            + selection.tokenizer.count(reasoning).tokens
            + count_tool_calls(selection.tokenizer.as_ref(), tool_calls);
        usage.add_category(TokenCategory::Output, output);
        usage.add_category(
            TokenCategory::ToolCall,
            count_tool_calls(selection.tokenizer.as_ref(), tool_calls),
        );
        usage
    }
}

fn count_message(tokenizer: &dyn super::Tokenizer, message: &ChatMessage) -> usize {
    let mut tokens = tokenizer.count(&message.content).tokens;
    if let Some(reasoning) = &message.reasoning {
        tokens += tokenizer.count(reasoning).tokens;
    }
    if let Some(calls) = &message.tool_calls {
        tokens += count_tool_calls(tokenizer, calls);
    }
    tokens + 1
}

fn count_tool_calls(tokenizer: &dyn super::Tokenizer, calls: &[ToolCallPayload]) -> usize {
    calls
        .iter()
        .map(|call| tokenizer.count(&call.name).tokens + tokenizer.count(&call.arguments).tokens)
        .sum()
}

pub struct AccountingProvider {
    inner: Arc<dyn AIProvider>,
    model: String,
    provider: String,
    accountant: TokenAccountant,
}

impl AccountingProvider {
    pub fn new(inner: Arc<dyn AIProvider>, model: impl Into<String>) -> Self {
        let provider = inner.id().to_string();
        Self {
            inner,
            model: model.into(),
            provider,
            accountant: TokenAccountant::default(),
        }
    }
}

#[async_trait]
impl AIProvider for AccountingProvider {
    fn id(&self) -> &'static str {
        self.inner.id()
    }

    fn uses_dsh_tools(&self) -> bool {
        self.inner.uses_dsh_tools()
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let started = std::time::Instant::now();
        let call_id = uuid::Uuid::new_v4().to_string();
        let request_id = request.request_id.clone();
        let session_id = request.session_id.clone();
        let mut metrics_end = MetricsEndGuard { started, identity: self.uses_dsh_tools().then(|| serde_json::json!({
            "call_id":call_id,"request_id":request_id,"session_id":session_id,"model":self.model,
            "cache_hit_tokens":null,"cache_hit_ratio":null,"reasoning_tokens":null,"completion_tokens":null,
            "first_provider_event_ms":null,"first_content_ms":null,"retry_count":null
        })) };
        if self.uses_dsh_tools() {
            crate::core::chat::telemetry::record_provider_metrics(
                &serde_json::json!({"kind":"model_call_start", "call_id":call_id, "request_id":request_id, "session_id":session_id, "model":self.model, "timestamp":chrono::Utc::now().to_rfc3339()}),
            );
        }
        let mut first_event_ms = None;
        let mut first_content_ms = None;
        let mut retries = 0usize;
        let mut input = self
            .accountant
            .count_request(&self.model, &self.provider, &request);
        let (inner_tx, mut inner_rx) = mpsc::channel(64);
        let inner = Arc::clone(&self.inner);
        let metrics_context = (call_id.clone(), request_id.clone(), session_id.clone());
        let mut task = ProviderTaskGuard(tauri::async_runtime::spawn(async move {
            crate::core::chat::telemetry::PROVIDER_METRICS_CONTEXT
                .scope(metrics_context, inner.stream(request, inner_tx))
                .await
        }));
        let mut provider_usage: Option<TokenUsage> = None;
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        let mut saw_finish = false;

        while let Some(event) = inner_rx.recv().await {
            if !matches!(&event, StreamEvent::Start | StreamEvent::Status { .. }) {
                first_event_ms.get_or_insert(started.elapsed().as_millis());
            }
            if matches!(&event, StreamEvent::Delta(text) if !text.is_empty()) {
                first_content_ms.get_or_insert(started.elapsed().as_millis());
            }
            if matches!(&event, StreamEvent::Status { kind } if kind.starts_with("stream_retry:")) {
                retries += 1;
            }
            if let Some(value) = metrics_end.identity.as_mut() {
                value["first_provider_event_ms"] = serde_json::json!(first_event_ms);
                value["first_content_ms"] = serde_json::json!(first_content_ms);
                value["retry_count"] = serde_json::json!(retries);
            }
            if self.uses_dsh_tools()
                && matches!(&event, StreamEvent::Status { kind } if kind.starts_with("stream_retry:"))
            {
                content.clear();
                reasoning.clear();
                tool_calls.clear();
                provider_usage = None;
            }
            match &event {
                StreamEvent::Delta(value) => content.push_str(value),
                StreamEvent::Reasoning(value) => reasoning.push_str(value),
                StreamEvent::ToolCall(call) => tool_calls.push(call.clone()),
                StreamEvent::TurnComplete {
                    content: value,
                    reasoning: turn_reasoning,
                    tool_calls: calls,
                    ..
                } => {
                    content.clone_from(value);
                    if let Some(value) = turn_reasoning {
                        reasoning.clone_from(value);
                    }
                    tool_calls.clone_from(calls);
                }
                StreamEvent::Usage(usage) => {
                    provider_usage = Some(usage.clone());
                    continue;
                }
                StreamEvent::Finish => {
                    saw_finish = true;
                    continue;
                }
                _ => {}
            }
            if tx.send(event).await.is_err() {
                return Err(ProviderError::cancelled());
            }
        }

        let result = (&mut task.0)
            .await
            .map_err(|error| ProviderError::message(error.to_string()))?;
        let output = self.accountant.count_output(
            &self.model,
            &self.provider,
            &content,
            &reasoning,
            &tool_calls,
        );
        let mut usage = if let Some(usage) = provider_usage {
            usage
        } else {
            input.accumulate(&output);
            input.clone()
        };
        usage.system_tokens = input.system_tokens;
        usage.context_tokens = input.context_tokens;
        usage.memory_tokens = input.memory_tokens;
        usage.tool_result_tokens = input.tool_result_tokens;
        usage.tool_call_tokens = input.tool_call_tokens + output.tool_call_tokens;
        if usage.accuracy == TokenAccuracy::Exact && input.accuracy != TokenAccuracy::Exact {
            usage.accuracy = TokenAccuracy::Mixed;
        }
        if self.uses_dsh_tools() {
            let cached = usage.cache_read_tokens;
            let prompt_total = usage.input_tokens + cached.unwrap_or(0);
            crate::core::chat::telemetry::record_provider_metrics(&serde_json::json!({
                "kind":"model_call", "timestamp": chrono::Utc::now().to_rfc3339(),
                "call_id":call_id, "request_id":request_id, "session_id":session_id, "model":self.model,
                "duration_ms":started.elapsed().as_millis(), "first_provider_event_ms":first_event_ms, "first_content_ms":first_content_ms,
                "retry_count":retries, "succeeded":result.is_ok(), "cache_hit_tokens":cached,
                "cache_hit_ratio":cached.and_then(|n| (prompt_total > 0).then_some(n as f64 / prompt_total as f64)),
                "reasoning_tokens":usage.reasoning_tokens, "completion_tokens":usage.output_tokens,
                "body_output_tokens":usage.reasoning_tokens.map(|n| usage.output_tokens.saturating_sub(n)),
                "usage_accuracy":usage.accuracy
            }));
            metrics_end.identity = None;
        }
        let _ = tx.send(StreamEvent::Usage(usage)).await;
        if saw_finish {
            let _ = tx.send(StreamEvent::Finish).await;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{MessageStatus, RequestContext};

    #[tokio::test]
    async fn cancelling_accounting_wrapper_aborts_silent_inner_provider() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct SilentProvider(Arc<AtomicBool>);
        struct Dropped(Arc<AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        #[async_trait]
        impl AIProvider for SilentProvider {
            fn id(&self) -> &'static str {
                "silent-test"
            }
            async fn stream(
                &self,
                _request: ChatRequest,
                tx: mpsc::Sender<StreamEvent>,
            ) -> Result<(), ProviderError> {
                let _probe = Dropped(self.0.clone());
                let _ = tx.send(StreamEvent::Start).await;
                std::future::pending::<()>().await;
                Ok(())
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let provider = AccountingProvider::new(Arc::new(SilentProvider(dropped.clone())), "test");
        let request = ChatRequest {
            request_id: "cancel".into(),
            session_id: "s".into(),
            messages: vec![],
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: Arc::from([]),
            temperature: None,
            max_tokens: None,
        };
        let (tx, mut rx) = mpsc::channel(16);
        let wrapper = tokio::spawn(async move { provider.stream(request, tx).await });
        rx.recv().await.unwrap();
        wrapper.abort();
        let _ = wrapper.await;
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !dropped.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("silent inner request leaked after cancellation");
    }

    struct UsageProvider;

    #[async_trait]
    impl AIProvider for UsageProvider {
        fn id(&self) -> &'static str {
            "deepseek"
        }

        async fn stream(
            &self,
            _request: ChatRequest,
            tx: mpsc::Sender<StreamEvent>,
        ) -> Result<(), ProviderError> {
            let _ = tx
                .send(StreamEvent::Usage(TokenUsage::exact(
                    12,
                    3,
                    "test-provider",
                )))
                .await;
            let _ = tx.send(StreamEvent::Finish).await;
            Ok(())
        }
    }

    #[test]
    fn agent_request_categories_accumulate_without_double_counting_total() {
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: Arc::from([]),
            temperature: None,
            max_tokens: None,
            messages: vec![ChatMessage {
                id: "context-s-1".into(),
                session_id: "s".into(),
                role: Role::System,
                content: "workspace context".into(),
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 0,
                estimated_tokens: None,
            }],
        };
        let usage = TokenAccountant::default().count_request("unknown", "unknown", &request);
        assert!(usage.input_tokens > 0);
        assert_eq!(usage.context_tokens, usage.input_tokens);
        assert_eq!(usage.total_tokens, usage.input_tokens);
        assert_eq!(usage.accuracy, TokenAccuracy::Estimated);
    }

    #[tokio::test]
    async fn accounting_emits_usage_before_finish() {
        let provider = AccountingProvider::new(Arc::new(UsageProvider), "deepseek-chat");
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: Arc::from([]),
            temperature: None,
            max_tokens: None,
            messages: Vec::new(),
        };
        let (tx, mut rx) = mpsc::channel(8);
        provider.stream(request, tx).await.unwrap();
        assert!(matches!(rx.recv().await, Some(StreamEvent::Usage(_))));
        assert!(matches!(rx.recv().await, Some(StreamEvent::Finish)));
    }
}
