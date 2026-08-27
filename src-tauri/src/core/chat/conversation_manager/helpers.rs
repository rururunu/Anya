use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::chat::error::ChatError;
use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::{ChatMessage, MessageStatus, Role};

pub(super) fn block_on_compat<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::spawn(move || tauri::async_runtime::block_on(future))
            .join()
            .expect("async database init thread panicked"),
        Err(_) => tauri::async_runtime::block_on(future),
    }
}

pub(super) fn settle_orphaned_in_sessions(
    sessions: &mut HashMap<String, Vec<ChatMessage>>,
) -> Vec<ChatMessage> {
    let mut dirty = Vec::new();
    for messages in sessions.values_mut() {
        for message in messages.iter_mut() {
            if settle_message_in_place(message) {
                dirty.push(message.clone());
            }
        }
    }
    dirty
}

/// After a crash, `finish_turn` may never have run. Create conversation-only
/// checkpoints for any user message that is missing one so rewind stays available.
pub(super) fn ensure_conversation_checkpoints_for_sessions(
    sessions: &HashMap<String, Vec<ChatMessage>>,
) {
    let store = crate::core::checkpoint::shared_checkpoint_store();
    for (session_id, messages) in sessions {
        let existing = store.list(session_id).unwrap_or_default();
        let have: std::collections::HashSet<String> = existing
            .into_iter()
            .filter_map(|checkpoint| checkpoint.user_message_id)
            .collect();
        let mut turn = 0usize;
        for message in messages {
            if message.role != Role::User {
                continue;
            }
            turn += 1;
            if have.contains(&message.id) {
                continue;
            }
            let _ = store.ensure_conversation_checkpoint(
                session_id,
                turn,
                &message.content,
                &message.id,
                None,
            );
        }
    }
}

pub(super) fn settle_message_in_place(message: &mut ChatMessage) -> bool {
    let mut changed = false;
    if matches!(
        message.status,
        MessageStatus::Pending | MessageStatus::Streaming
    ) {
        message.status = MessageStatus::Cancelled;
        changed = true;
    }
    if let Some(activities) = message.tool_activities.as_mut() {
        for activity in activities.iter_mut() {
            if activity.status == "running" {
                activity.status = "error".into();
                activity.success = false;
                if activity
                    .result
                    .as_ref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    activity.result = Some("interrupted".into());
                }
                changed = true;
            }
        }
    }
    if changed {
        refresh_message_token_cache(message);
    }
    changed
}

pub(super) fn session_preview(messages: &[ChatMessage]) -> String {
    for message in messages {
        if matches!(message.role, Role::User) {
            let trimmed = super::super::selection::visible_user_text(&message.content);
            if !trimmed.is_empty() {
                return truncate_preview(&trimmed);
            }
        }
    }
    for message in messages {
        if matches!(message.role, Role::Assistant) {
            let trimmed = message.content.trim();
            if !trimmed.is_empty() {
                return truncate_preview(trimmed);
            }
        }
    }
    "（空会话）".into()
}

fn truncate_preview(value: &str) -> String {
    const MAX: usize = 72;
    let normalized = value.replace('\n', " ").trim().to_string();
    if normalized.chars().count() <= MAX {
        return normalized;
    }
    let truncated: String = normalized.chars().take(MAX).collect();
    format!("{truncated}…")
}

pub fn create_message(
    session_id: &str,
    role: Role,
    content: String,
    status: MessageStatus,
) -> ChatMessage {
    ChatMessage {
        id: format!("msg-{}", uuid::Uuid::new_v4()),
        session_id: session_id.to_string(),
        role,
        content,
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status,
        timestamp: now_millis(),
        estimated_tokens: None,
    }
}

pub(super) fn refresh_message_token_cache(message: &mut ChatMessage) {
    if matches!(
        message.status,
        MessageStatus::Pending | MessageStatus::Streaming
    ) {
        message.estimated_tokens = None;
    } else {
        message.estimated_tokens = Some(estimate_message_tokens(message));
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn lock_error<T: std::fmt::Display>(error: T) -> ChatError {
    ChatError::Internal(error.to_string())
}
