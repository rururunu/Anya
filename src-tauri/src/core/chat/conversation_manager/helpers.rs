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

pub(super) fn is_trivial_user_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_lowercase();
    let stripped: String = lower
        .chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(
                    *c,
                    '.' | ',' | '!' | '?' | '。' | '，' | '！' | '？' | '~' | '、' | '-' | '_'
                )
        })
        .collect();
    matches!(
        stripped.as_str(),
        "你好"
            | "您好"
            | "hi"
            | "hello"
            | "hey"
            | "在吗"
            | "在不在"
            | "好的"
            | "好的谢谢"
            | "好的好的"
            | "谢谢"
            | "收到"
            | "ok"
            | "okay"
            | "k"
            | "行"
            | "可以"
            | "对"
            | "对的"
            | "是的"
            | "嗯"
            | "嗯嗯"
            | "yes"
            | "no"
            | "thanks"
            | "thankyou"
            | "继续"
            | "接着说"
            | "下一步"
            | "continue"
            | "next"
            | "goon"
    )
}

pub(super) fn clean_assistant_summary(content: &str, max_chars: usize) -> String {
    let mut cleaned = String::new();
    let mut in_code_fence = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            if in_code_fence {
                cleaned.push_str(" [代码] ");
            }
            continue;
        }
        if !in_code_fence && !trimmed.is_empty() {
            if !cleaned.is_empty() {
                cleaned.push(' ');
            }
            cleaned.push_str(trimmed);
        }
    }
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    crate::core::chat::limits::truncate_chars(&collapsed, max_chars)
}

pub(super) fn is_plugin_agent_session_id(session_id: &str) -> bool {
    session_id.starts_with("plugin:")
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn trivial_user_text_detects_common_fillers() {
        assert!(is_trivial_user_text("你好"));
        assert!(is_trivial_user_text("好的谢谢！"));
        assert!(is_trivial_user_text("ok"));
        assert!(is_trivial_user_text("继续"));
        assert!(!is_trivial_user_text("帮我写一个网页爬虫"));
        assert!(!is_trivial_user_text("现在用ai重新生成标题他的效果很差劲"));
    }

    #[test]
    fn plugin_agent_session_ids_use_the_plugin_prefix() {
        assert!(is_plugin_agent_session_id("plugin:group:room"));
        assert!(!is_plugin_agent_session_id("session-1"));
    }
}
