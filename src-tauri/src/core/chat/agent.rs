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
use super::agent_loop::post_edit_verify::{verify_feedback_content, VerificationQueue};
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
        if self.provider.uses_dsh_tools() && !self.tools.is_dsh() {
            let adapted = Self {
                provider: Arc::clone(&self.provider),
                tools: Arc::new(self.tools.dsh_contract()),
                max_steps: self.max_steps,
                max_turn_tokens: self.max_turn_tokens,
                tool_output_max_chars: self.tool_output_max_chars,
            };
            let mut context = tool_ctx;
            context.registry = Some(adapted.tools.registry());
            return adapted
                .run_loop(request, context, tx, cancelled, soft_queue)
                .instrument(span)
                .await;
        }
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
        let mut tool_executor =
            ToolExecutor::new(Arc::clone(&self.tools), self.tool_output_max_chars);
        if self.tools.is_dsh() {
            let text = crate::core::tools::dsh::tool_prompt(&self.tools.schemas());
            let mut guidance = request
                .messages
                .first()
                .cloned()
                .unwrap_or_else(|| ChatMessage {
                    id: String::new(),
                    session_id: request.session_id.clone(),
                    role: Role::System,
                    content: String::new(),
                    reasoning: None,
                    work_timeline: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    status: MessageStatus::Done,
                    timestamp: 0,
                    estimated_tokens: None,
                });
            if request
                .messages
                .first()
                .is_none_or(|m| m.role != Role::System)
            {
                let mut identity = guidance.clone();
                identity.id = format!("dsh-identity-{}", request.session_id);
                identity.role = Role::System;
                identity.content = crate::core::tools::dsh::IDENTITY.trim().into();
                request.messages.insert(0, identity);
            }
            guidance.id = format!("dsh-tools-{}", request.session_id);
            guidance.role = Role::System;
            guidance.content = text;
            guidance.reasoning = None;
            guidance.tool_calls = None;
            guidance.tool_call_id = None;
            request.messages.insert(1, guidance);
            if self
                .tools
                .registry()
                .names()
                .iter()
                .any(|name| name == "skill")
            {
                let mut catalog = request.messages[1].clone();
                catalog.id = format!("dsh-skills-{}", request.session_id);
                catalog.content = crate::core::tools::dsh::skill_catalog();
                request.messages.insert(2, catalog);
            }
        }
        let mut steps = 0u32;
        let mut context_recoveries = 0u32;
        let mut failure_breaker = FailureBreaker::new();
        let mut completion_gate = CompletionGate::new();
        completion_gate.set_workspace_root(tool_ctx.workspace_root.clone());
        let mut verification_queue = VerificationQueue::default();
        let mut task_state = super::agent_loop::task_state::TaskState::new(&request);
        let mut criteria_retries = 0u32;
        let mut progress_evidence = std::collections::HashSet::new();
        let mut discovered_tools = std::collections::HashSet::new();
        if crate::core::tools::image_mode::is_image_mode(&request.session_id) {
            completion_gate.require_image();
        }
        if crate::core::chat::session_origin::shared_session_origin_store()
            .is_companion(tool_ctx.root_session_id())
        {
            completion_gate.require_share_deliverable();
        }
        completion_gate.capture_goal_from_request(&request);
        let mut user_msg_index = request
            .messages
            .iter()
            .rposition(|msg| msg.role == Role::User);
        let mut used_tokens = estimate_request_tokens(&request);
        let mut last_compact_msg_len = 0usize;
        // Freeze tool schemas for the whole turn. Mid-turn Plan accept must not
        // rebuild `tools` (DeepSeek disk cache). Plan writers are blocked by
        // authorize, not by shrinking this frozen set.
        let mut frozen_tools: Option<std::sync::Arc<[serde_json::Value]>> = None;

        loop {
            let permission_mode = crate::core::tools::tool_approval::shared_tool_approval_store()
                .mode_for_session(tool_ctx.root_session_id());
            let policy = match permission_mode {
                crate::models::settings::ToolApprovalMode::Ask => "Read-only: filesystem mutations and arbitrary shell execution are denied. Reading and session bookkeeping are allowed. User approval cannot elevate this mode; the user must switch permission mode.",
                crate::models::settings::ToolApprovalMode::Auto => "Workspace-write: native file tools can modify the workspace. External paths retain their permission gate. Shell commands run without OS filesystem isolation and require separate approval for each invocation. Grants apply only once.",
                crate::models::settings::ToolApprovalMode::AlwaysAllow => "Full-access: tools can operate outside the workspace without interactive approval. Explicit tool safety rules still apply.",
            };
            if !request
                .messages
                .iter()
                .any(|message| message.id == "permission-policy" && message.content == policy)
            {
                request
                    .messages
                    .retain(|message| message.id != "permission-policy");
                let mut message = crate::core::chat::conversation_manager::create_message(
                    &request.session_id,
                    Role::System,
                    policy.into(),
                    MessageStatus::Done,
                );
                message.id = "permission-policy".into();
                request.messages.insert(0, message);
            }
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            if task_state.observe_followups(&request) {
                completion_gate.capture_goal_from_request(&request);
            }

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

            if let Some(mut outcome) = mid_turn_compact::maybe_compact(
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
                outcome.summary.content.push_str(&format!(
                    "\n\n[Preserved task state]\n{}",
                    task_state.snapshot()
                ));
                if let Some(message) = request
                    .messages
                    .iter_mut()
                    .find(|m| m.id == outcome.summary.id)
                {
                    message.content = outcome.summary.content.clone();
                }
                persist_mid_turn_compact(&tool_ctx, outcome);
            }

            if !self.tools.is_dsh() {
                task_state.inject(&mut request);
            } else {
                let plan_id = format!("dsh-plan-{}", request.session_id);
                request.messages.retain(|m| m.id != plan_id);
                if crate::core::tools::plan_mode::shared_plan_mode_store()
                    .is_active(tool_ctx.root_session_id())
                {
                    let mut policy = request.messages[0].clone();
                    policy.id = plan_id;
                    policy.content = "Plan mode is active. Inspect and research with read-only tools. Do not modify files or run shell commands. Present the complete markdown plan with exit_plan_mode for user review. Continue execution only after approval. Delegated agents must remain read-only.".into();
                    request
                        .messages
                        .insert(3.min(request.messages.len()), policy);
                }
                for notice in crate::core::tools::dsh::completion_notices(&tool_ctx.session_id) {
                    if let Some(mut message) = request.messages.last().cloned() {
                        message.id = uuid::Uuid::new_v4().to_string();
                        message.role = Role::User;
                        message.content = notice;
                        message.reasoning = None;
                        message.tool_calls = None;
                        message.tool_call_id = None;
                        message.name = None;
                        request.messages.push(message);
                    }
                }
            }
            let tools = frozen_tools
                .get_or_insert_with(|| {
                    self.tools.focused_schemas(
                        &request,
                        tool_ctx.root_session_id(),
                        &discovered_tools,
                    )
                })
                .clone();
            request.tools = tools;
            tool_executor.set_allowed_tools(
                request
                    .tools
                    .iter()
                    .filter_map(|schema| schema["function"]["name"].as_str().map(str::to_owned)),
            );
            if !self.tools.is_dsh() {
                crate::core::chat::prompt::ensure_plan_mode_prompt(
                    &mut request.messages,
                    tool_ctx.root_session_id(),
                );
            }
            let stream_turn_span = tracing::info_span!(
                target: "peek.agent",
                "agent.stream_turn",
                session_id = %request.session_id,
                step = steps,
            );
            let turn_result = self
                .stream_with_recovery(&request, &tx, &cancelled)
                .instrument(stream_turn_span)
                .await;
            let turn = match turn_result {
                Ok(turn) => turn,
                Err(error) => {
                    // Reactive compaction: the provider rejected the request for
                    // exceeding the context window. Fold history and retry only
                    // while each recovery actually reduces the request size,
                    // instead of hard-failing the turn.
                    if error.is_context_window_exceeded() && context_recoveries < 3 {
                        context_recoveries += 1;
                        let before_tokens = estimate_request_tokens(&request);
                        if let Some(mut outcome) = mid_turn_compact::force_compact(
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
                            outcome.summary.content.push_str(&format!(
                                "\n\n[Preserved task state]\n{}",
                                task_state.snapshot()
                            ));
                            if let Some(message) = request
                                .messages
                                .iter_mut()
                                .find(|m| m.id == outcome.summary.id)
                            {
                                message.content = outcome.summary.content.clone();
                            }
                            persist_mid_turn_compact(&tool_ctx, outcome);
                            if estimate_request_tokens(&request) < before_tokens {
                                let _ = tx
                                    .send(StreamEvent::Status {
                                        kind: format!("stream_retry:{context_recoveries}:3"),
                                    })
                                    .await;
                                continue;
                            }
                        }
                    }
                    return Err(error);
                }
            };
            context_recoveries = 0;
            let StreamTurnResult {
                content,
                reasoning,
                tool_calls,
                finish_reason,
            } = turn;

            used_tokens += estimate_tokens(&content) + estimate_tokens(&reasoning);

            if tool_calls.is_empty() {
                if self.tools.is_dsh() {
                    let _ = tx
                        .send(StreamEvent::TurnComplete {
                            content,
                            reasoning: non_empty(reasoning),
                            tool_calls: vec![],
                            finish_reason,
                        })
                        .await;
                    break;
                }
                if task_state.execution_paused() {
                    let _ = tx
                        .send(StreamEvent::TurnComplete {
                            content,
                            reasoning: non_empty(reasoning),
                            tool_calls: vec![],
                            finish_reason: Some("user_paused".into()),
                        })
                        .await;
                    break;
                }
                let mut pending = std::mem::take(&mut verification_queue);
                let verify_ctx = tool_ctx.clone();
                let report =
                    tauri::async_runtime::spawn_blocking(move || pending.run_pending(&verify_ctx))
                        .await
                        .map_err(|e| {
                            ProviderError::message(format!("verification task failed: {e}"))
                        })?;
                if cancelled.load(Ordering::Relaxed) {
                    return Err(ProviderError::cancelled());
                }
                completion_gate.note_verified_paths(report.verified_paths);
                if !report.outcomes.is_empty() {
                    for outcome in report.outcomes {
                        task_state.record(&outcome);
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
                        push_challenge_message(
                            &mut request,
                            &mut user_msg_index,
                            &verify_feedback_content(&outcome),
                        );
                    }
                    // The model must see actual check results before writing its final answer.
                    steps += 1;
                    continue;
                }
                let unresolved = task_state.unresolved_criteria();
                if !unresolved.is_empty() {
                    if criteria_retries < 3 {
                        criteria_retries += 1;
                        push_challenge_message(&mut request, &mut user_msg_index,
                            &format!("[System] Acceptance criteria still lack successful evidence: {}. Complete and verify them, then update_tasks with evidence call IDs, or report the blocker.", unresolved.join("; ")));
                        steps += 1;
                        continue;
                    }
                    let _ = tx
                        .send(StreamEvent::TurnComplete {
                            content: format!(
                                "任务尚未通过验收：{}。已保留任务状态和执行证据。",
                                unresolved.join("；")
                            ),
                            reasoning: non_empty(reasoning),
                            tool_calls: vec![],
                            finish_reason: Some("unverified_completion".into()),
                        })
                        .await;
                    break;
                }
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
                        request.messages.push(ChatMessage {
                            id: format!("msg-{}", now_millis()),
                            session_id: request.session_id.clone(),
                            role: Role::Assistant,
                            content: content.clone(),
                            reasoning: reasoning.clone(),
                            work_timeline: None,
                            tool_activities: None,
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            status: MessageStatus::Done,
                            timestamp: now_millis(),
                            estimated_tokens: None,
                        });
                        if drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index)
                            .await
                        {
                            steps += 1;
                            continue;
                        }
                        tokio::task::yield_now().await;
                        if drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index)
                            .await
                        {
                            steps += 1;
                            continue;
                        }
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
            completion_gate.record_tool_outcomes(&self.tools, &outcomes);
            verification_queue.record(&outcomes, &tool_ctx.workspace_root);
            for outcome in &outcomes {
                if outcome.success
                    && !matches!(
                        outcome.tool_name.as_str(),
                        "update_tasks" | "save_plan" | "wait_for_shell" | "read_shell_output"
                    )
                    && progress_evidence.insert((
                        outcome.tool_name.clone(),
                        outcome.arguments.clone(),
                        outcome.result.clone(),
                    ))
                {
                    criteria_retries = 0;
                }
                task_state.record(outcome);
                if outcome.success && outcome.tool_name == "search_tools" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&outcome.result) {
                        if let Some(tools) = value["tools"].as_array() {
                            discovered_tools.extend(tools.iter().filter_map(|s| {
                                s["function"]["name"].as_str().map(str::to_string)
                            }));
                        }
                    }
                }
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
                        content: "已停止：你拒绝了本次工具操作。".to_string(),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("user_denied".to_string()),
                    })
                    .await;
                return Ok(());
            }

            if completion_gate.repeated_without_progress() {
                let _ = tx.send(StreamEvent::TurnComplete {
                    content: "已暂停：相同工具调用反复返回相同结果，换策略提示后仍未获得新证据。任务尚未完成。".into(),
                    reasoning: None, tool_calls: vec![], finish_reason: Some("no_progress".into()),
                }).await;
                break;
            }

            // Soft-inject at tool boundary before the next provider call.
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            steps += 1;
        }

        Ok(())
    }

    /// Recover only the pending model round. Previously committed tools remain
    /// in request.messages and are never executed again by this recovery path.
    async fn stream_with_recovery(
        &self,
        request: &ChatRequest,
        tx: &mpsc::Sender<StreamEvent>,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<StreamTurnResult, ProviderError> {
        for recovery in 0..=3u32 {
            if cancelled.load(Ordering::Relaxed) || tx.is_closed() {
                return Err(ProviderError::cancelled());
            }
            let (turn_tx, turn_rx) = mpsc::channel(64);
            let provider = Arc::clone(&self.provider);
            let turn_request = request.clone();
            let provider_task =
                tauri::async_runtime::spawn(
                    async move { provider.stream(turn_request, turn_tx).await },
                );
            let collected = stream_turn::collect_stream_turn(turn_rx, tx, cancelled).await;
            let result = match collected {
                Ok(turn) => match provider_task.await {
                    Ok(Ok(())) => Ok(turn),
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(ProviderError::message(format!(
                        "provider task failed: {error}"
                    ))),
                },
                Err(error) => {
                    provider_task.abort();
                    Err(error)
                }
            };
            match result {
                Err(error)
                    if recovery < 3
                        && crate::core::ai::deepseek::is_retryable_stream_error(&error) =>
                {
                    #[cfg(not(test))]
                    let delay = std::time::Duration::from_secs(10 * (1 << recovery));
                    #[cfg(test)]
                    let delay = std::time::Duration::from_millis(20 * (1 << recovery));
                    tracing::warn!(session_id = %request.session_id, recovery = recovery + 1, ?delay, %error, "recovering pending agent round");
                    let _ = tx
                        .send(StreamEvent::Status {
                            kind: format!("stream_retry:{}:3", recovery + 1),
                        })
                        .await;
                    let deadline = tokio::time::Instant::now() + delay;
                    while tokio::time::Instant::now() < deadline {
                        if cancelled.load(Ordering::Relaxed) || tx.is_closed() {
                            return Err(ProviderError::cancelled());
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
                result => return result,
            }
        }
        unreachable!("bounded recovery returns on its final attempt")
    }

    pub async fn run_subagent(
        provider: Arc<dyn AIProvider>,
        registry: Arc<ToolRegistry>,
        tool_ctx: ToolContext,
        prompt: String,
        read_only: bool,
    ) -> Result<String, ToolError> {
        let mut tool_ctx = tool_ctx;
        tool_ctx.provider = Some(Arc::clone(&provider));
        let active_tools = Arc::new(ToolManager::new(registry.filter_for_subagent(read_only)));
        tool_ctx.registry = Some(active_tools.registry());
        let runner = AgentRunner::new(provider, active_tools).with_max_turn_tokens(48_000);
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
        let answer = Arc::new(tokio::sync::Mutex::new((
            String::new(),
            Option::<String>::None,
        )));
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
                            message_id: None,
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
