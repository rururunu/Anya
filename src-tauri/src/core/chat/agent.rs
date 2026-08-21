use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::limits::{
    estimate_tokens, DEFAULT_MAX_STEPS, DEFAULT_MAX_TURN_TOKENS, TOOL_OUTPUT_MAX_CHARS,
};
use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent};
use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::runtime::ToolManager;
use tracing::Instrument;

use super::agent_loop::challenge::push_challenge_message;
use super::agent_loop::challenge::{ChallengeOutcome, CompletionGate};
use super::agent_loop::failure::{FailureAction, FailureBreaker};
use super::agent_loop::mid_turn_compact;
use super::agent_loop::post_edit_verify::{
    maybe_run_post_edit_verification, verify_feedback_content,
};
use super::agent_loop::soft_inject::drain_soft_injects;
use super::agent_loop::stream_turn::{self, StreamTurnResult};
use super::agent_loop::tools::ToolExecutor;
use super::agent_loop::types::{estimate_request_tokens, non_empty, now_millis};

pub struct AgentRunner {
    provider: Arc<dyn AIProvider>,
    tools: Arc<ToolManager>,
    max_steps: u32,
    max_turn_tokens: usize,
    tool_output_max_chars: usize,
}

impl AgentRunner {
    /// Create a runner with default step / token / tool-output limits.
    pub fn new(provider: Arc<dyn AIProvider>, tools: Arc<ToolManager>) -> Self {
        Self {
            provider,
            tools,
            max_steps: DEFAULT_MAX_STEPS,
            max_turn_tokens: DEFAULT_MAX_TURN_TOKENS,
            tool_output_max_chars: TOOL_OUTPUT_MAX_CHARS,
        }
    }

    /// Override the per-turn token budget used for soft truncation.
    pub fn with_max_turn_tokens(mut self, max_turn_tokens: usize) -> Self {
        self.max_turn_tokens = max_turn_tokens;
        self
    }

    /// Override the max tool-loop steps (`0` = unlimited).
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = max_steps;
        self
    }

    #[cfg(test)]
    pub fn with_limits(
        provider: Arc<dyn AIProvider>,
        tools: Arc<ToolManager>,
        max_steps: u32,
        max_turn_tokens: usize,
        tool_output_max_chars: usize,
    ) -> Self {
        Self {
            provider,
            tools,
            max_steps,
            max_turn_tokens,
            tool_output_max_chars,
        }
    }

    /// Drive the agent loop: stream model output, execute tools, enforce
    /// completion / verification challenges, and honor cancellation.
    ///
    /// This is orchestration only — the actual policies live in
    /// `super::agent_loop` so new turn behaviors can be added there without
    /// growing this function.
    pub async fn run(
        &self,
        request: ChatRequest,
        tool_ctx: ToolContext,
        tx: mpsc::Sender<StreamEvent>,
        cancelled: Arc<AtomicBool>,
        soft_queue: Arc<Mutex<VecDeque<String>>>,
    ) -> Result<(), ProviderError> {
        let span = tracing::info_span!(
            target: "peek.agent",
            "agent_run",
            session_id = %request.session_id,
            request_id = %request.request_id,
            provider = ?request.provider,
        );
        self.run_loop(request, tool_ctx, tx, cancelled, soft_queue)
            .instrument(span)
            .await
    }

    async fn run_loop(
        &self,
        mut request: ChatRequest,
        tool_ctx: ToolContext,
        tx: mpsc::Sender<StreamEvent>,
        cancelled: Arc<AtomicBool>,
        soft_queue: Arc<Mutex<VecDeque<String>>>,
    ) -> Result<(), ProviderError> {
        request.tools = self
            .tools
            .schemas_for_request(&request, tool_ctx.root_session_id());
        let tool_executor = ToolExecutor::new(Arc::clone(&self.tools), self.tool_output_max_chars);
        let mut steps = 0u32;
        let mut context_compacted = false;
        let mut failure_breaker = FailureBreaker::new();
        let mut completion_gate = CompletionGate::new();
        completion_gate.capture_goal_from_request(&request);
        let mut user_msg_index = request
            .messages
            .iter()
            .rposition(|msg| msg.role == Role::User);
        let mut used_tokens = estimate_request_tokens(&request);
        let mut last_compact_msg_len = 0usize;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            if self.max_steps > 0 && steps >= self.max_steps {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: format!(
                            "已停止：本轮达到最大工具步数上限（{}）。可发送「继续」让我接着做未完成的部分。",
                            self.max_steps
                        ),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("max_steps".to_string()),
                    })
                    .await;
                break;
            }

            if let Some(outcome) = mid_turn_compact::maybe_compact(
                &self.provider,
                self.max_turn_tokens,
                &mut request,
                &mut user_msg_index,
                &mut used_tokens,
                &mut last_compact_msg_len,
                &tx,
            )
            .await
            {
                persist_mid_turn_compact(&tool_ctx, outcome);
            }

            let (turn_tx, turn_rx) = mpsc::channel::<StreamEvent>(64);
            let provider = Arc::clone(&self.provider);
            let turn_request = request.clone();
            let stream_turn_span = tracing::info_span!(
                target: "peek.agent",
                "agent.stream_turn",
                session_id = %request.session_id,
                step = steps,
            );
            let provider_task = tauri::async_runtime::spawn(
                async move { provider.stream(turn_request, turn_tx).await }
                    .instrument(stream_turn_span.clone()),
            );

            let StreamTurnResult {
                content,
                reasoning,
                tool_calls,
                finish_reason,
            } = stream_turn::collect_stream_turn(turn_rx, &tx, &cancelled)
                .instrument(stream_turn_span)
                .await?;

            let provider_result = provider_task.await.map_err(|error| {
                ProviderError::message(format!("provider task failed: {error}"))
            })?;
            if let Err(error) = provider_result {
                // Reactive compaction: the provider rejected the request for
                // exceeding the context window. Fold prior history once and retry
                // instead of hard-failing the turn.
                if error.is_context_window_exceeded() && !context_compacted {
                    context_compacted = true;
                    if let Some(outcome) = mid_turn_compact::force_compact(
                        &self.provider,
                        self.max_turn_tokens,
                        &mut request,
                        &mut user_msg_index,
                        &mut used_tokens,
                        &tx,
                    )
                    .await
                    {
                        last_compact_msg_len = request.messages.len();
                        persist_mid_turn_compact(&tool_ctx, outcome);
                        continue;
                    }
                }
                return Err(error);
            }

            used_tokens += estimate_tokens(&content) + estimate_tokens(&reasoning);

            if tool_calls.is_empty() {
                match completion_gate.evaluate_final_answer(
                    &mut request,
                    &mut user_msg_index,
                    content,
                    reasoning,
                    finish_reason,
                ) {
                    ChallengeOutcome::ContinueWithChallenge { status_kind } => {
                        let _ = tx.send(StreamEvent::Status { kind: status_kind }).await;
                        steps += 1;
                        continue;
                    }
                    ChallengeOutcome::Finish {
                        content,
                        reasoning,
                        finish_reason,
                    } => {
                        let _ = tx
                            .send(StreamEvent::TurnComplete {
                                content,
                                reasoning,
                                tool_calls: vec![],
                                finish_reason,
                            })
                            .await;
                        break;
                    }
                }
            }

            let _ = tx
                .send(StreamEvent::Status {
                    kind: format!("tools:{}", tool_calls.len()),
                })
                .await;

            let assistant = ChatMessage {
                id: format!("msg-{}", now_millis()),
                session_id: request.session_id.clone(),
                role: Role::Assistant,
                content: content.clone(),
                reasoning: non_empty(reasoning.clone()),
                work_timeline: None,
                tool_activities: None,
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: now_millis(),
                estimated_tokens: None,
            };
            request.messages.push(assistant);

            let parallel = tool_executor.should_run_parallel(&tool_calls);

            let outcomes = if parallel {
                tool_executor
                    .execute_tools_parallel(&tool_calls, &tool_ctx, &cancelled)
                    .await?
            } else {
                tool_executor
                    .execute_tools_serial(&tool_calls, &tool_ctx, &cancelled)
                    .await?
            };

            let mut user_denied = false;
            for outcome in &outcomes {
                completion_gate.record_tool_outcome(&self.tools, outcome);
                used_tokens += estimate_tokens(&outcome.result);
                tool_ctx.conversation.journal().record_tool_outcome(
                    tool_ctx.root_session_id(),
                    &request.request_id,
                    &tool_ctx.assistant_message_id,
                    &outcome.tool_name,
                    &outcome.arguments,
                    outcome.success,
                    &outcome.result,
                );
                request.messages.push(ChatMessage {
                    id: format!("msg-{}", now_millis()),
                    session_id: request.session_id.clone(),
                    role: Role::Tool,
                    content: outcome.result.clone(),
                    reasoning: None,
                    work_timeline: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: Some(outcome.call_id.clone()),
                    name: Some(outcome.tool_name.clone()),
                    status: MessageStatus::Done,
                    timestamp: now_millis(),
                    estimated_tokens: None,
                });
                if outcome.user_denied {
                    user_denied = true;
                }
            }

            // Optional hard verification pass after successful file mutations.
            if let Some(verify_outcome) = maybe_run_post_edit_verification(&outcomes, &tool_ctx) {
                completion_gate.record_tool_outcome(&self.tools, &verify_outcome);
                used_tokens += estimate_tokens(&verify_outcome.result);
                tool_ctx.conversation.journal().record_tool_outcome(
                    tool_ctx.root_session_id(),
                    &request.request_id,
                    &tool_ctx.assistant_message_id,
                    &verify_outcome.tool_name,
                    &verify_outcome.arguments,
                    verify_outcome.success,
                    &verify_outcome.result,
                );
                request.messages.push(ChatMessage {
                    id: format!("msg-{}", now_millis()),
                    session_id: request.session_id.clone(),
                    role: Role::User,
                    content: verify_feedback_content(&verify_outcome),
                    reasoning: None,
                    work_timeline: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    status: MessageStatus::Done,
                    timestamp: now_millis(),
                    estimated_tokens: None,
                });
            }

            match failure_breaker.check(&outcomes) {
                FailureAction::Stop { reason } => {
                    let _ = tx
                        .send(StreamEvent::TurnComplete {
                            content: format!("已停止：{reason}"),
                            reasoning: None,
                            tool_calls: vec![],
                            finish_reason: Some("tool_failure_breaker".to_string()),
                        })
                        .await;
                    return Ok(());
                }
                FailureAction::Challenge {
                    status_kind,
                    message,
                } => {
                    push_challenge_message(&mut request, &mut user_msg_index, &message);
                    let _ = tx.send(StreamEvent::Status { kind: status_kind }).await;
                }
                FailureAction::Continue => {}
            }

            if let Some(status_kind) =
                completion_gate.maybe_challenge_stall(&mut request, &mut user_msg_index)
            {
                let _ = tx.send(StreamEvent::Status { kind: status_kind }).await;
            }

            if user_denied {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: "已停止：你拒绝了文件访问权限。".to_string(),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("user_denied".to_string()),
                    })
                    .await;
                return Ok(());
            }

            // Soft-inject at tool boundary before the next provider call.
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            steps += 1;
        }

        Ok(())
    }

    pub async fn run_subagent(
        provider: Arc<dyn AIProvider>,
        registry: Arc<ToolRegistry>,
        tool_ctx: ToolContext,
        prompt: String,
        read_only: bool,
    ) -> Result<String, ToolError> {
        let active_tools = Arc::new(ToolManager::new(registry.filter_for_subagent(read_only)));
        let runner = AgentRunner::new(provider, active_tools)
            .with_max_steps(24)
            .with_max_turn_tokens(48_000);
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
        let cancelled = Arc::clone(&tool_ctx.cancelled);
        let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
        let request = ChatRequest {
            request_id: format!("sub-{}", now_millis()),
            session_id: tool_ctx.session_id.clone(),
            messages: vec![ChatMessage {
                id: format!("msg-{}", now_millis()),
                session_id: tool_ctx.session_id.clone(),
                role: Role::User,
                content: prompt,
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: now_millis(),
                estimated_tokens: None,
            }],
            context: Default::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        };

        // Spawn a background task to receive from rx concurrently to avoid channel deadlocks.
        let answer = Arc::new(tokio::sync::Mutex::new((String::new(), Option::<String>::None)));
        let answer_clone = Arc::clone(&answer);
        let progress_bus = Arc::clone(&tool_ctx.event_bus);
        let progress_subagent_id = tool_ctx.subagent_id.clone();
        let rx_task = tauri::async_runtime::spawn(async move {
            let mut response_reported = false;
            let mut reasoning_reported = false;
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TurnComplete {
                        content,
                        finish_reason,
                        ..
                    } => {
                        let mut lock = answer_clone.lock().await;
                        lock.0 = content;
                        lock.1 = finish_reason;
                    }
                    StreamEvent::Delta(delta) => {
                        if !response_reported {
                            if let Some(subagent_id) = &progress_subagent_id {
                                progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                    subagent_id: subagent_id.clone(),
                                    kind: "responding".to_string(),
                                    content: "Generating response".to_string(),
                                    timestamp_ms: now_millis(),
                                });
                            }
                            response_reported = true;
                        }
                        let mut lock = answer_clone.lock().await;
                        lock.0.push_str(&delta);
                    }
                    StreamEvent::Reasoning(_) => {
                        if !reasoning_reported {
                            if let Some(subagent_id) = &progress_subagent_id {
                                progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                    subagent_id: subagent_id.clone(),
                                    kind: "reasoning".to_string(),
                                    content: "Reasoning".to_string(),
                                    timestamp_ms: now_millis(),
                                });
                            }
                            reasoning_reported = true;
                        }
                    }
                    StreamEvent::Usage(usage) => {
                        progress_bus.emit(crate::core::event::BusEvent::TokenUsage {
                            session_id: None,
                            model: "subagent".to_string(),
                            usage,
                        });
                    }
                    StreamEvent::Status { kind } => {
                        if let Some(subagent_id) = &progress_subagent_id {
                            progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                subagent_id: subagent_id.clone(),
                                kind: "status".to_string(),
                                content: kind,
                                timestamp_ms: now_millis(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        });

        Box::pin(runner.run(request, tool_ctx, tx, cancelled, soft_queue))
            .await
            .map_err(|error| ToolError::new(error.to_string()))?;

        // Wait for the receiver task to finish draining
        let _ = rx_task.await;

        let (final_answer, finish_reason) = answer.lock().await.clone();
        if finish_reason.as_deref() == Some("tool_failure_breaker") {
            return Err(ToolError::new(final_answer));
        }
        Ok(final_answer)
    }
}

fn persist_mid_turn_compact(
    tool_ctx: &ToolContext,
    outcome: mid_turn_compact::MidTurnCompactOutcome,
) {
    let Some(before_id) = outcome.persist_before_id else {
        return;
    };
    tool_ctx.conversation.insert_compaction_summary(
        tool_ctx.root_session_id(),
        outcome.summary,
        Some(before_id.as_str()),
    );
}
