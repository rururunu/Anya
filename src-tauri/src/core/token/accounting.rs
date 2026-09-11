use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::runtime::{ChatMessage, ChatRequest, Role, StreamEvent, ToolCallPayload};

use super::{TokenAccuracy, TokenCategory, TokenUsage, TokenizerRegistry};

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

    async fn stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let mut input = self
            .accountant
            .count_request(&self.model, &self.provider, &request);
        let (inner_tx, mut inner_rx) = mpsc::channel(64);
        let inner = Arc::clone(&self.inner);
        let task =
            tauri::async_runtime::spawn(async move { inner.stream(request, inner_tx).await });
        let mut provider_usage: Option<TokenUsage> = None;
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        let mut saw_finish = false;

        while let Some(event) = inner_rx.recv().await {
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
                break;
            }
        }

        let result = task
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
