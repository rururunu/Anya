use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::async_runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::error::ChatError;
use crate::core::chat::telemetry::TurnSpan;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatRequest, MessageStatus, StreamEvent};
use crate::core::token::AccountingProvider;
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem, ToolContext};
use crate::runtime::ToolManager;

use super::events::{
    finish_success, finish_with_error, handle_stream_event, non_empty_string, StreamEventCtx,
    StreamEventOutcome,
};
use super::StreamManager;

pub(crate) struct ActiveTask {
    pub session_id: String,
    pub epoch: u64,
    pub cancelled: Arc<AtomicBool>,
    pub soft_queue: Arc<Mutex<VecDeque<String>>>,
    pub content: Arc<Mutex<String>>,
    pub reasoning: Arc<Mutex<String>>,
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
    /// Spawns an agent stream task and registers it for lifecycle tracking.
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

                let mut ctx = StreamEventCtx {
                    conversation: &conversation,
                    event_bus: &event_bus,
                    journal: &journal,
                    session_id: &session_id,
                    assistant_message_id: &assistant_message_id,
                    turn_id: &turn_id,
                    turn_span: &mut turn_span,
                    content: &mut content,
                    reasoning: &mut reasoning,
                    content_ref: &content_ref,
                    reasoning_ref: &reasoning_ref,
                    streaming_started: &mut streaming_started,
                    finish_reason: &mut finish_reason,
                    stable_content_len: &mut stable_content_len,
                    stable_reasoning_len: &mut stable_reasoning_len,
                    stable_timeline_len: &mut stable_timeline_len,
                    usage_pool: &usage_pool,
                    usage_session_id: &usage_session_id,
                    usage_model: &usage_model,
                    usage_provider: &usage_provider,
                };

                match handle_stream_event(event, &mut ctx) {
                    StreamEventOutcome::Continue => {}
                    StreamEventOutcome::Break => break,
                    StreamEventOutcome::Abort {
                        content,
                        reasoning,
                        error,
                    } => {
                        journal.flush_message(&assistant_message_id);
                        turn_span.finish_err(&error);
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
                                error,
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
                    finish_success(
                        &event_bus,
                        &conversation,
                        &session_id,
                        &assistant_message_id,
                        content,
                        reasoning,
                        finish_reason,
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

    /// Cancels a streaming assistant message and emits the appropriate finish event.
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

pub(super) fn epoch_still_active(
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

fn public_workspace_root() -> std::path::PathBuf {
    let root = env::temp_dir().join("peek-public");
    let _ = std::fs::create_dir_all(&root);
    root
}
