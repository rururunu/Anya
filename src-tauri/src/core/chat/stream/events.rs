use std::sync::Arc;

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::telemetry::TurnSpan;
use crate::core::event::EventBus;
use crate::core::runtime::{MessageStatus, Role, WorkTimelineItem};

pub(crate) fn finish_success(
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
        .find(|message| message.role == Role::User)
        .map(|message| super::super::selection::visible_user_text(&message.content).to_string());

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

    event_bus.emit(crate::core::event::BusEvent::ChatFinished {
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

pub(crate) fn enrich_from_work_timeline(
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

pub(crate) fn finish_with_error(
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

    event_bus.emit(crate::core::event::BusEvent::ChatError {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        message: error,
    });

    let _ = content;
}

pub(crate) fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(crate) enum StreamEventOutcome {
    Continue,
    Break,
    Abort {
        content: String,
        reasoning: String,
        error: String,
    },
}

pub(crate) struct StreamEventCtx<'a> {
    pub conversation: &'a ConversationManager,
    pub event_bus: &'a Arc<dyn EventBus>,
    pub journal: &'a crate::core::chat::journal::SessionJournal,
    pub session_id: &'a str,
    pub assistant_message_id: &'a str,
    pub turn_id: &'a str,
    pub turn_span: &'a mut TurnSpan,
    pub content: &'a mut String,
    pub reasoning: &'a mut String,
    pub content_ref: &'a Arc<std::sync::Mutex<String>>,
    pub reasoning_ref: &'a Arc<std::sync::Mutex<String>>,
    pub streaming_started: &'a mut bool,
    pub finish_reason: &'a mut Option<String>,
    pub stable_content_len: &'a mut usize,
    pub stable_reasoning_len: &'a mut usize,
    pub stable_timeline_len: &'a mut usize,
    pub usage_pool: &'a sqlx::SqlitePool,
    pub usage_session_id: &'a str,
    pub usage_model: &'a str,
    pub usage_provider: &'a str,
}

pub(crate) fn handle_stream_event(
    event: crate::core::runtime::StreamEvent,
    ctx: &mut StreamEventCtx<'_>,
) -> StreamEventOutcome {
    use crate::core::chat::conversation_manager::TimelineTextKind;
    use crate::core::event::BusEvent;
    use crate::core::runtime::StreamEvent;

    match event {
        StreamEvent::Start => StreamEventOutcome::Continue,
        StreamEvent::Delta(delta) => {
            if !*ctx.streaming_started {
                *ctx.streaming_started = true;
                ctx.conversation.update_message(
                    ctx.session_id,
                    ctx.assistant_message_id,
                    MessageStatus::Streaming,
                    None,
                    None,
                );
            }
            ctx.turn_span.mark_first_token();
            ctx.content.push_str(&delta);
            if let Ok(mut guard) = ctx.content_ref.lock() {
                guard.push_str(&delta);
            }
            ctx.journal.record_delta(
                ctx.session_id,
                ctx.turn_id,
                ctx.assistant_message_id,
                &delta,
                false,
            );
            ctx.conversation.append_work_timeline_text(
                ctx.session_id,
                ctx.assistant_message_id,
                TimelineTextKind::Content,
                &delta,
            );
            ctx.event_bus.emit(BusEvent::ChatDelta {
                session_id: ctx.session_id.to_string(),
                message_id: ctx.assistant_message_id.to_string(),
                delta,
            });
            StreamEventOutcome::Continue
        }
        StreamEvent::Reasoning(chunk) => {
            ctx.turn_span.mark_first_token();
            ctx.reasoning.push_str(&chunk);
            if let Ok(mut guard) = ctx.reasoning_ref.lock() {
                guard.push_str(&chunk);
            }
            ctx.journal.record_delta(
                ctx.session_id,
                ctx.turn_id,
                ctx.assistant_message_id,
                &chunk,
                true,
            );
            ctx.conversation.append_work_timeline_text(
                ctx.session_id,
                ctx.assistant_message_id,
                TimelineTextKind::Reasoning,
                &chunk,
            );
            ctx.event_bus.emit(BusEvent::ChatReasoning {
                session_id: ctx.session_id.to_string(),
                message_id: ctx.assistant_message_id.to_string(),
                content: chunk,
            });
            StreamEventOutcome::Continue
        }
        StreamEvent::Status { kind } => {
            if kind.starts_with("stream_retry") {
                ctx.content.truncate(*ctx.stable_content_len);
                ctx.reasoning.truncate(*ctx.stable_reasoning_len);
                if let Ok(mut guard) = ctx.content_ref.lock() {
                    guard.truncate(*ctx.stable_content_len);
                }
                if let Ok(mut guard) = ctx.reasoning_ref.lock() {
                    guard.truncate(*ctx.stable_reasoning_len);
                }
                ctx.conversation.truncate_work_timeline(
                    ctx.session_id,
                    ctx.assistant_message_id,
                    *ctx.stable_timeline_len,
                );
                ctx.conversation.update_message(
                    ctx.session_id,
                    ctx.assistant_message_id,
                    MessageStatus::Streaming,
                    Some(ctx.content.clone()),
                    Some(non_empty_string(ctx.reasoning.clone())),
                );
            } else if kind == "soft_injected" {
                ctx.turn_span.soft_inject(0);
            } else if kind.starts_with("tools:") {
                if let Ok(count) = kind.trim_start_matches("tools:").parse::<u32>() {
                    ctx.turn_span.add_tools(count);
                }
                // Round committed — later retries keep this prefix.
                *ctx.stable_content_len = ctx.content.len();
                *ctx.stable_reasoning_len = ctx.reasoning.len();
                *ctx.stable_timeline_len =
                    ctx.conversation
                        .work_timeline_len(ctx.session_id, ctx.assistant_message_id);
            }
            ctx.event_bus.emit(BusEvent::ChatStatus {
                session_id: ctx.session_id.to_string(),
                message_id: ctx.assistant_message_id.to_string(),
                kind,
            });
            StreamEventOutcome::Continue
        }
        StreamEvent::UserContentPatch { message_id, content } => {
            let status = ctx
                .conversation
                .find_message(&message_id)
                .map(|(_, msg)| msg.status)
                .unwrap_or(MessageStatus::Done);
            ctx.conversation.update_message(
                ctx.session_id,
                &message_id,
                status,
                Some(content.clone()),
                None,
            );
            ctx.event_bus.emit(BusEvent::ChatUserContent {
                session_id: ctx.session_id.to_string(),
                message_id,
                content,
            });
            StreamEventOutcome::Continue
        }
        StreamEvent::ToolCall(_) => StreamEventOutcome::Continue,
        StreamEvent::Usage(usage) => {
            let usage = usage.clone();
            let persisted_usage = usage.clone();
            let pool = ctx.usage_pool.clone();
            let run_id = ctx.turn_id.to_string();
            let session_id = ctx.usage_session_id.to_string();
            let message_id = ctx.assistant_message_id.to_string();
            let model = ctx.usage_model.to_string();
            let persisted_model = model.clone();
            let provider = ctx.usage_provider.to_string();
            let recorded_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis() as u64)
                .unwrap_or_default();
            tauri::async_runtime::spawn(async move {
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
            ctx.event_bus.emit(BusEvent::TokenUsage {
                session_id: Some(ctx.usage_session_id.to_string()),
                message_id: Some(ctx.assistant_message_id.to_string()),
                model: ctx.usage_model.to_string(),
                usage,
            });
            StreamEventOutcome::Continue
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
                    if ctx.content.trim().is_empty() {
                        *ctx.content = notice.to_string();
                    } else if !ctx.content.contains(notice) {
                        *ctx.content = format!("{}\n\n{}", ctx.content.trim_end(), notice);
                    }
                    ctx.conversation.append_work_timeline_text(
                        ctx.session_id,
                        ctx.assistant_message_id,
                        TimelineTextKind::Content,
                        &format!("\n\n{notice}"),
                    );
                } else {
                    // Multi-turn agent loops stream deltas across many
                    // provider rounds, then emit TurnComplete with only
                    // the *last* round's text. Never shrink the
                    // accumulated transcript — that dropped earlier
                    // reasoning/narration and left the UI incomplete.
                    if ctx.content.is_empty() || turn_content.len() >= ctx.content.len() {
                        *ctx.content = turn_content;
                    }
                }
                if let Ok(mut guard) = ctx.content_ref.lock() {
                    *guard = ctx.content.clone();
                }
            }
            if let Some(value) = turn_reasoning {
                if ctx.reasoning.is_empty() || value.len() >= ctx.reasoning.len() {
                    *ctx.reasoning = value;
                }
                if let Ok(mut guard) = ctx.reasoning_ref.lock() {
                    *guard = ctx.reasoning.clone();
                }
            }
            *ctx.finish_reason = turn_finish;
            StreamEventOutcome::Continue
        }
        StreamEvent::Finish => StreamEventOutcome::Break,
        StreamEvent::Error(message) => StreamEventOutcome::Abort {
            content: ctx.content.clone(),
            reasoning: ctx.reasoning.clone(),
            error: message,
        },
    }
}
