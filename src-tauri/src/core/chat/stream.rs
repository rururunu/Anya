use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::async_runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::conversation_manager::{ConversationManager, TimelineTextKind};
use crate::core::chat::error::ChatError;
use crate::core::chat::limits::truncate_chars;
use crate::core::chat::telemetry::TurnSpan;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, RequestContext, Role, StreamEvent,
};
use crate::core::token::AccountingProvider;
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem, ToolContext};
use crate::runtime::ToolManager;

struct ActiveTask {
    session_id: String,
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    soft_queue: Arc<Mutex<VecDeque<String>>>,
    content: Arc<Mutex<String>>,
    reasoning: Arc<Mutex<String>>,
}

pub struct StreamManager {
    active_tasks: Arc<Mutex<HashMap<String, ActiveTask>>>,
    epoch_counter: Arc<AtomicU64>,
}

pub(crate) struct StreamSpawnInput {
    pub provider: Arc<dyn AIProvider>,
    pub tools: Arc<ToolManager>,
    pub event_bus: Arc<dyn EventBus>,
    pub conversation: Arc<ConversationManager>,
    pub ask_store: Arc<AskStore>,
    pub path_permission_store: Arc<PathPermissionStore>,
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub app_handle: Option<tauri::AppHandle>,
    pub request: ChatRequest,
    pub assistant_message_id: String,
    pub session_id: String,
    pub max_turn_tokens: usize,
    pub model: String,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            epoch_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn active_assistant_for_session(&self, session_id: &str) -> Option<String> {
        let active = self.active_tasks.lock().ok()?;
        active
            .iter()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, _)| id.clone())
    }

    pub fn soft_inject(&self, session_id: &str, content: String) -> Result<String, ChatError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }
        let mut active = self
            .active_tasks
            .lock()
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        let (message_id, task) = active
            .iter_mut()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, task)| (id.clone(), task))
            .ok_or(ChatError::MessageNotFound)?;
        if let Ok(mut queue) = task.soft_queue.lock() {
            queue.push_back(content);
        }
        Ok(message_id)
    }

    pub(crate) fn spawn(&self, input: StreamSpawnInput) {
        let StreamSpawnInput {
            provider,
            tools,
            event_bus,
            conversation,
            ask_store,
            path_permission_store,
            tasks,
            app_handle,
            request,
            assistant_message_id,
            session_id,
            max_turn_tokens,
            model,
        } = input;
        let cancelled = Arc::new(AtomicBool::new(false));
        let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
        let content_ref = Arc::new(Mutex::new(String::new()));
        let reasoning_ref = Arc::new(Mutex::new(String::new()));
        let epoch = self.epoch_counter.fetch_add(1, Ordering::Relaxed);
        let turn_id = Uuid::new_v4().to_string();
        let usage_pool = conversation.db_pool();
        let usage_session_id = session_id.clone();
        let usage_model = model.clone();
        let usage_provider = provider.id().to_string();

        if let Ok(mut active) = self.active_tasks.lock() {
            active.insert(
                assistant_message_id.clone(),
                ActiveTask {
                    session_id: session_id.clone(),
                    epoch,
                    cancelled: cancelled.clone(),
                    soft_queue: Arc::clone(&soft_queue),
                    content: Arc::clone(&content_ref),
                    reasoning: Arc::clone(&reasoning_ref),
                },
            );
        }

        let journal = conversation.journal().clone();
        let active_tasks = Arc::clone(&self.active_tasks);
        let workspace_root = request
            .context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root))
            .unwrap_or_else(public_workspace_root);
        let provider_id = provider.id().to_string();

        async_runtime::spawn(async move {
            let mut turn_span = TurnSpan::start(
                &session_id,
                &turn_id,
                &assistant_message_id,
                &provider_id,
                &model,
            );

            let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
            let accounting_provider: Arc<dyn AIProvider> = Arc::new(AccountingProvider::new(
                Arc::clone(&provider),
                model.clone(),
                app_handle.clone(),
            ));
            let tool_ctx = ToolContext {
                workspace_root,
                request_context: request.context.clone(),
                session_id: session_id.clone(),
                assistant_message_id: assistant_message_id.clone(),
                conversation: Arc::clone(&conversation),
                event_bus: Arc::clone(&event_bus),
                tasks,
                ask_store,
                path_permission_store,
                registry: Some(tools.registry()),
                provider: Some(Arc::clone(&accounting_provider)),
                subagent_depth: 0,
                max_subagent_depth: 1,
                subagent_id: None,
                parent_activity_id: None,
                app_handle: app_handle.clone(),
                cancelled: Arc::clone(&cancelled),
            };

            let runner =
                AgentRunner::new(accounting_provider, tools).with_max_turn_tokens(max_turn_tokens);
            let agent_task = async_runtime::spawn({
                let request = request.clone();
                let tx = tx.clone();
                let cancelled = cancelled.clone();
                let soft_queue = Arc::clone(&soft_queue);
                async move {
                    runner
                        .run(request, tool_ctx, tx, cancelled, soft_queue)
                        .await
                }
            });
            drop(tx);

            let mut content = String::new();
            let mut reasoning = String::new();
            let mut streaming_started = false;
            let mut finish_reason = None;
            // Text committed by completed provider rounds (after tools:N).
            // stream_retry must not wipe these — only the failed attempt.
            let mut stable_content_len = 0usize;
            let mut stable_reasoning_len = 0usize;
            let mut stable_timeline_len = 0usize;

            while let Some(event) = rx.recv().await {
                if !epoch_still_active(&active_tasks, &assistant_message_id, epoch) {
                    break;
                }
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }

                match event {
                    StreamEvent::Start => {}
                    StreamEvent::Delta(delta) => {
                        if !streaming_started {
                            streaming_started = true;
                            conversation.update_message(
                                &session_id,
                                &assistant_message_id,
                                MessageStatus::Streaming,
                                None,
                                None,
                            );
                        }
                        turn_span.mark_first_token();
                        content.push_str(&delta);
                        if let Ok(mut guard) = content_ref.lock() {
                            guard.push_str(&delta);
                        }
                        journal.record_delta(
                            &session_id,
                            &turn_id,
                            &assistant_message_id,
                            &delta,
                            false,
                        );
                        conversation.append_work_timeline_text(
                            &session_id,
                            &assistant_message_id,
                            TimelineTextKind::Content,
                            &delta,
                        );
                        event_bus.emit(BusEvent::ChatDelta {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            delta,
                        });
                    }
                    StreamEvent::Reasoning(chunk) => {
                        turn_span.mark_first_token();
                        reasoning.push_str(&chunk);
                        if let Ok(mut guard) = reasoning_ref.lock() {
                            guard.push_str(&chunk);
                        }
                        journal.record_delta(
                            &session_id,
                            &turn_id,
                            &assistant_message_id,
                            &chunk,
                            true,
                        );
                        conversation.append_work_timeline_text(
                            &session_id,
                            &assistant_message_id,
                            TimelineTextKind::Reasoning,
                            &chunk,
                        );
                        event_bus.emit(BusEvent::ChatReasoning {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            content: chunk,
                        });
                    }
                    StreamEvent::Status { kind } => {
                        if kind.starts_with("stream_retry") {
                            content.truncate(stable_content_len);
                            reasoning.truncate(stable_reasoning_len);
                            if let Ok(mut guard) = content_ref.lock() {
                                guard.truncate(stable_content_len);
                            }
                            if let Ok(mut guard) = reasoning_ref.lock() {
                                guard.truncate(stable_reasoning_len);
                            }
                            conversation.truncate_work_timeline(
                                &session_id,
                                &assistant_message_id,
                                stable_timeline_len,
                            );
                            conversation.update_message(
                                &session_id,
                                &assistant_message_id,
                                MessageStatus::Streaming,
                                Some(content.clone()),
                                Some(non_empty_string(reasoning.clone())),
                            );
                        } else if kind == "soft_injected" {
                            turn_span.soft_inject(0);
                        } else if kind.starts_with("tools:") {
                            if let Ok(count) = kind.trim_start_matches("tools:").parse::<u32>() {
                                turn_span.add_tools(count);
                            }
                            // Round committed — later retries keep this prefix.
                            stable_content_len = content.len();
                            stable_reasoning_len = reasoning.len();
                            stable_timeline_len =
                                conversation.work_timeline_len(&session_id, &assistant_message_id);
                        }
                        event_bus.emit(BusEvent::ChatStatus {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            kind,
                        });
                    }
                    StreamEvent::UserContentPatch {
                        message_id,
                        content,
                    } => {
                        let status = conversation
                            .find_message(&message_id)
                            .map(|(_, msg)| msg.status)
                            .unwrap_or(MessageStatus::Done);
                        conversation.update_message(
                            &session_id,
                            &message_id,
                            status,
                            Some(content.clone()),
                            None,
                        );
                        event_bus.emit(BusEvent::ChatUserContent {
                            session_id: session_id.clone(),
                            message_id,
                            content,
                        });
                    }
                    StreamEvent::ToolCall(_) => {}
                    StreamEvent::Usage(usage) => {
                        let usage = usage.clone();
                        let persisted_usage = usage.clone();
                        let pool = usage_pool.clone();
                        let run_id = turn_id.clone();
                        let session_id = usage_session_id.clone();
                        let message_id = assistant_message_id.clone();
                        let model = usage_model.clone();
                        let persisted_model = model.clone();
                        let provider = usage_provider.clone();
                        let recorded_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|value| value.as_millis() as u64)
                            .unwrap_or_default();
                        async_runtime::spawn(async move {
                            if let Err(error) = crate::core::chat::db::record_token_usage(
                                &pool,
                                &run_id,
                                &session_id,
                                Some(&message_id),
                                &persisted_model,
                                Some(&provider),
                                &persisted_usage,
                                recorded_at as i64,
                            )
                            .await
                            {
                                tracing::warn!(%error, "failed to persist token usage");
                            }
                        });
                        event_bus.emit(BusEvent::TokenUsage {
                            session_id: Some(usage_session_id.clone()),
                            message_id: Some(assistant_message_id.clone()),
                            model: model.clone(),
                            usage,
                        });
                    }
                    StreamEvent::TurnComplete {
                        content: turn_content,
                        reasoning: turn_reasoning,
                        tool_calls: _,
                        finish_reason: turn_finish,
                    } => {
                        if !turn_content.is_empty() {
                            let is_stop_notice = matches!(
                                turn_finish.as_deref(),
                                Some("tool_failure_breaker" | "max_steps" | "user_denied")
                            );
                            if is_stop_notice {
                                // Keep prior streamed narration; surface the stop
                                // summary so the UI can show why the turn ended.
                                let notice = turn_content.trim();
                                if content.trim().is_empty() {
                                    content = notice.to_string();
                                } else if !content.contains(notice) {
                                    content = format!("{}\n\n{}", content.trim_end(), notice);
                                }
                                conversation.append_work_timeline_text(
                                    &session_id,
                                    &assistant_message_id,
                                    TimelineTextKind::Content,
                                    &format!("\n\n{notice}"),
                                );
                            } else {
                                // Multi-turn agent loops stream deltas across many
                                // provider rounds, then emit TurnComplete with only
                                // the *last* round's text. Never shrink the
                                // accumulated transcript — that dropped earlier
                                // reasoning/narration and left the UI incomplete.
                                if content.is_empty() || turn_content.len() >= content.len() {
                                    content = turn_content;
                                }
                            }
                            if let Ok(mut guard) = content_ref.lock() {
                                *guard = content.clone();
                            }
                        }
                        if let Some(value) = turn_reasoning {
                            if reasoning.is_empty() || value.len() >= reasoning.len() {
                                reasoning = value;
                            }
                            if let Ok(mut guard) = reasoning_ref.lock() {
                                *guard = reasoning.clone();
                            }
                        }
                        finish_reason = turn_finish;
                    }
                    StreamEvent::Finish => break,
                    StreamEvent::Error(message) => {
                        journal.flush_message(&assistant_message_id);
                        turn_span.finish_err(&message);
                        let _ = crate::core::checkpoint::shared_checkpoint_store()
                            .finish_turn(&session_id);
                        if epoch_still_active(&active_tasks, &assistant_message_id, epoch) {
                            finish_with_error(
                                &event_bus,
                                &conversation,
                                &session_id,
                                &assistant_message_id,
                                content,
                                reasoning,
                                message,
                            );
                        }
                        active_tasks
                            .lock()
                            .ok()
                            .and_then(|mut active| active.remove(&assistant_message_id));
                        let _ = agent_task.await;
                        return;
                    }
                }
            }

            journal.flush_message(&assistant_message_id);

            let should_finish = match active_tasks.lock() {
                Ok(mut active) => match active.get(&assistant_message_id) {
                    Some(task) if task.epoch == epoch => {
                        active.remove(&assistant_message_id);
                        true
                    }
                    _ => false,
                },
                Err(_) => false,
            };

            let result = if cancelled.load(Ordering::Relaxed) {
                Err(ProviderError::cancelled())
            } else {
                match agent_task.await {
                    Ok(result) => result,
                    Err(error) => Err(ProviderError::message(format!(
                        "agent task failed: {error}"
                    ))),
                }
            };

            if !should_finish {
                let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
                return;
            }

            // Checkpoints must be visible before the completion event refreshes the UI.
            let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
            match result {
                Ok(()) => {
                    turn_span.finish_ok(finish_reason.as_deref());
                    let assistant_content = content.clone();
                    finish_success(
                        &event_bus,
                        &conversation,
                        &session_id,
                        &assistant_message_id,
                        content,
                        reasoning,
                        finish_reason,
                    );
                    maybe_generate_session_title(
                        Arc::clone(&conversation),
                        Arc::clone(&event_bus),
                        Arc::clone(&provider),
                        session_id.clone(),
                        assistant_content,
                    );
                }
                Err(ProviderError::Cancelled) => {
                    conversation.update_message(
                        &session_id,
                        &assistant_message_id,
                        MessageStatus::Cancelled,
                        Some(content),
                        Some(non_empty_string(reasoning)),
                    );
                    turn_span.finish_err("cancelled");
                }
                Err(error) => {
                    let message = error.to_string();
                    turn_span.finish_err(&message);
                    finish_with_error(
                        &event_bus,
                        &conversation,
                        &session_id,
                        &assistant_message_id,
                        content,
                        reasoning,
                        message,
                    );
                }
            }
        });
    }

    pub fn cancel(
        &self,
        conversation: &ConversationManager,
        event_bus: &dyn EventBus,
        message_id: &str,
    ) -> Result<(), ChatError> {
        let active = self
            .active_tasks
            .lock()
            .map_err(|error| ChatError::Internal(error.to_string()))?
            .remove(message_id);

        let Some(task) = active else {
            if let Some((session_id, message)) = conversation.settle_interrupted_message(message_id)
            {
                let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
                event_bus.emit(BusEvent::ChatFinished {
                    session_id,
                    message_id: message_id.to_string(),
                    content: message.content,
                    reasoning: message.reasoning,
                    finish_reason: Some("cancelled".to_string()),
                });
                return Ok(());
            }
            return Err(ChatError::MessageNotFound);
        };

        // Bump global epoch so late events from this task are ignored if any race.
        self.epoch_counter.fetch_add(1, Ordering::Relaxed);
        task.cancelled.store(true, Ordering::Relaxed);

        let content = task
            .content
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default();
        let reasoning = task
            .reasoning
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default();

        conversation.journal().flush_message(message_id);

        let (session_id, message) = conversation.find_message(message_id)?;
        let updated = conversation.update_message(
            &session_id,
            message_id,
            MessageStatus::Cancelled,
            Some(if content.is_empty() {
                message.content
            } else {
                content
            }),
            Some(non_empty_string(reasoning)),
        );
        let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
        if let Some(message) = updated {
            event_bus.emit(BusEvent::ChatFinished {
                session_id: session_id.clone(),
                message_id: message_id.to_string(),
                content: message.content,
                reasoning: message.reasoning,
                finish_reason: Some("cancelled".to_string()),
            });
        }
        Ok(())
    }
}

fn epoch_still_active(
    active_tasks: &Arc<Mutex<HashMap<String, ActiveTask>>>,
    message_id: &str,
    epoch: u64,
) -> bool {
    active_tasks
        .lock()
        .ok()
        .and_then(|active| active.get(message_id).map(|task| task.epoch == epoch))
        .unwrap_or(false)
}

fn finish_success(
    event_bus: &Arc<dyn EventBus>,
    conversation: &ConversationManager,
    session_id: &str,
    message_id: &str,
    content: String,
    reasoning: String,
    finish_reason: Option<String>,
) {
    let current_user = conversation
        .messages(session_id)
        .into_iter()
        .rev()
        .find(|message| message.role == crate::core::runtime::Role::User)
        .map(|message| super::selection::visible_user_text(&message.content).to_string());

    // work_timeline is the interleaved source of truth across tool rounds.
    // Prefer it when richer than the flat accumulators (which used to be
    // overwritten by the last provider turn's TurnComplete).
    let (content, reasoning) =
        enrich_from_work_timeline(conversation, session_id, message_id, content, reasoning);
    let reasoning = non_empty_string(reasoning);
    conversation.update_message(
        session_id,
        message_id,
        MessageStatus::Done,
        Some(content.clone()),
        Some(reasoning.clone()),
    );

    event_bus.emit(BusEvent::ChatFinished {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        content: content.clone(),
        reasoning,
        finish_reason: finish_reason.clone().or(Some("stop".to_string())),
    });
    if let Some(user) = current_user {
        tauri::async_runtime::spawn_blocking(move || {
            crate::core::tools::memory::shared_memory_store().remember_exchange(user, content);
        });
    }
}

fn enrich_from_work_timeline(
    conversation: &ConversationManager,
    session_id: &str,
    message_id: &str,
    content: String,
    reasoning: String,
) -> (String, String) {
    let Some(message) = conversation
        .messages(session_id)
        .into_iter()
        .find(|message| message.id == message_id)
    else {
        return (content, reasoning);
    };
    let Some(timeline) = message.work_timeline.as_ref() else {
        return (content, reasoning);
    };

    use crate::core::runtime::WorkTimelineItem;
    let mut timeline_content = String::new();
    let mut timeline_reasoning = String::new();
    for item in timeline {
        match item {
            WorkTimelineItem::Content { content: text, .. } => timeline_content.push_str(text),
            WorkTimelineItem::Reasoning { content: text, .. } => timeline_reasoning.push_str(text),
            WorkTimelineItem::Tool { .. } => {}
        }
    }

    let content = if timeline_content.len() > content.len() {
        timeline_content
    } else {
        content
    };
    let reasoning = if timeline_reasoning.len() > reasoning.len() {
        timeline_reasoning
    } else {
        reasoning
    };
    (content, reasoning)
}

const TITLE_MAX_CHARS: usize = 24;

/// Generate a concise AI title for the first completed exchange of a session.
/// Runs in the background; failures are logged and never block the turn.
fn maybe_generate_session_title(
    conversation: Arc<ConversationManager>,
    event_bus: Arc<dyn EventBus>,
    provider: Arc<dyn AIProvider>,
    session_id: String,
    assistant_content: String,
) {
    let messages = conversation.messages(&session_id);
    let user_turn_count = messages
        .iter()
        .filter(|message| message.role == Role::User)
        .count();
    // Only the very first exchange gets an AI title; later turns keep it.
    if user_turn_count != 1 || conversation.session_title(&session_id).is_some() {
        return;
    }
    let Some(first_user) = messages
        .iter()
        .find(|message| message.role == Role::User)
        .map(|message| super::selection::visible_user_text(&message.content).to_string())
    else {
        return;
    };
    if first_user.trim().is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        match generate_session_title(provider, &first_user, &assistant_content).await {
            Ok(title) => {
                conversation.set_session_title(&session_id, title.clone());
                event_bus.emit(BusEvent::ChatSessionTitleUpdated { session_id, title });
            }
            Err(error) => eprintln!("failed to generate session title: {error}"),
        }
    });
}

async fn generate_session_title(
    provider: Arc<dyn AIProvider>,
    first_user: &str,
    assistant_content: &str,
) -> Result<String, String> {
    let mut material = format!("User: {}", truncate_chars(first_user, 600));
    let reply = assistant_content.trim();
    if !reply.is_empty() {
        material.push_str("\n\nAssistant: ");
        material.push_str(&truncate_chars(reply, 400));
    }

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(16);
    let request = ChatRequest {
        request_id: format!("title-{}", now_millis()),
        session_id: "title".to_string(),
        messages: vec![
            ChatMessage {
                id: "title-system".into(),
                session_id: "title".into(),
                role: Role::System,
                content: "You create a very short conversation title. Reply with ONLY the title: plain text, no quotes, no trailing punctuation, no explanation, no emoji. Keep it to 2-6 words (under 24 characters). If the conversation is not in English, reply in the same language as the user's message.".into(),
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 0,
                estimated_tokens: None,
            },
            ChatMessage {
                id: "title-user".into(),
                session_id: "title".into(),
                role: Role::User,
                content: material,
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 0,
                estimated_tokens: None,
            },
        ],
        context: RequestContext::default(),
        provider: Some(provider.id().to_string()),
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: Some(0.2),
        max_tokens: Some(64),
    };

    let provider_task =
        tauri::async_runtime::spawn(async move { provider.stream(request, tx).await });

    let mut content = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Delta(delta) => content.push_str(&delta),
            StreamEvent::TurnComplete {
                content: turn_content,
                ..
            } => {
                if !turn_content.is_empty() {
                    content = turn_content;
                }
            }
            StreamEvent::Error(message) => return Err(message),
            _ => {}
        }
    }
    provider_task
        .await
        .map_err(|error| format!("title task failed: {error}"))?
        .map_err(|error| error.to_string())?;

    let title = clean_title(&content);
    if title.is_empty() {
        return Err("empty title".into());
    }
    Ok(truncate_chars(&title, TITLE_MAX_CHARS))
}

fn clean_title(value: &str) -> String {
    let mut cleaned = value.trim().to_string();
    for prefix in ['"', '\'', '「', '『', '《', '“', '‘'] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            cleaned = rest.trim_start().to_string();
            break;
        }
    }
    for suffix in [
        '"', '\'', '」', '』', '》', '”', '’', '.', '。', '!', '！', '?', '？', ':',
    ] {
        if let Some(rest) = cleaned.strip_suffix(suffix) {
            cleaned = rest.trim_end().to_string();
            break;
        }
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn finish_with_error(
    event_bus: &Arc<dyn EventBus>,
    conversation: &ConversationManager,
    session_id: &str,
    message_id: &str,
    content: String,
    reasoning: String,
    error: String,
) {
    let reasoning = non_empty_string(reasoning);
    conversation.update_message(
        session_id,
        message_id,
        MessageStatus::Error,
        Some(error.clone()),
        Some(reasoning),
    );

    event_bus.emit(BusEvent::ChatError {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        message: error,
    });

    let _ = content;
}

fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn public_workspace_root() -> std::path::PathBuf {
    let root = env::temp_dir().join("peek-public");
    let _ = std::fs::create_dir_all(&root);
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_mismatch_is_inactive() {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        tasks.lock().unwrap().insert(
            "m1".into(),
            ActiveTask {
                session_id: "s1".into(),
                epoch: 2,
                cancelled: Arc::new(AtomicBool::new(false)),
                soft_queue: Arc::new(Mutex::new(VecDeque::new())),
                content: Arc::new(Mutex::new(String::new())),
                reasoning: Arc::new(Mutex::new(String::new())),
            },
        );
        assert!(!epoch_still_active(&tasks, "m1", 1));
        assert!(epoch_still_active(&tasks, "m1", 2));
    }
}
