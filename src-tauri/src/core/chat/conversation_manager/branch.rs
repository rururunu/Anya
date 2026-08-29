use std::collections::{HashMap, HashSet};

use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::{ChatMessage, Role};
use crate::models::chat::ChatSessionSummary;

use super::helpers::{
    block_on_compat, ensure_conversation_checkpoints_for_sessions, session_preview,
    settle_message_in_place,
};
use super::ConversationManager;

impl ConversationManager {
    /// Clone an existing conversation into a new session in the same workspace.
    /// When `until_message_id` is set, later messages are omitted.
    pub fn branch_session(
        &self,
        source_session_id: &str,
        until_message_id: Option<&str>,
    ) -> Result<ChatSessionSummary, String> {
        let source_id = source_session_id.trim();
        if source_id.is_empty() {
            return Err("Session id is required".into());
        }

        self.ensure_session_loaded(source_id);
        let mut source_messages = self.messages(source_id);
        if let Some(until_id) = until_message_id.map(str::trim).filter(|id| !id.is_empty()) {
            let Some(end) = source_messages
                .iter()
                .position(|message| message.id == until_id)
            else {
                return Err("找不到要分支的消息".into());
            };
            source_messages.truncate(end + 1);
        }
        if source_messages.is_empty() {
            return Err("当前对话没有可分支的记录".into());
        }

        let new_id = format!("session-{}", uuid::Uuid::new_v4());
        let cloned: Vec<ChatMessage> = source_messages
            .into_iter()
            .map(|message| clone_message_for_branch(message, &new_id))
            .collect();

        let workspace_id = self.workspace_for_session(source_id);
        let title = {
            let base = self
                .session_title(source_id)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| session_preview(&cloned));
            next_branch_title(&base, &self.taken_session_titles())
        };

        {
            let pool = self.db_pool.clone();
            let to_save = cloned.clone();
            let sid = new_id.clone();
            let title_save = title.clone();
            let ws = workspace_id.clone();
            block_on_compat(async move {
                for message in &to_save {
                    super::super::db::save_message(&pool, message).await?;
                }
                super::super::db::save_session_title(
                    &pool,
                    &sid,
                    &title_save,
                    super::super::session_title::SessionTitleSource::Auto,
                )
                .await?;
                if let Some(workspace_id) = ws.as_deref() {
                    super::super::db::bind_session_workspace(&pool, &sid, workspace_id).await?;
                }
                Ok::<(), String>(())
            })?;
        }

        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(new_id.clone(), cloned.clone());
        }
        if let Ok(mut loaded) = self.loaded_sessions.lock() {
            loaded.insert(new_id.clone());
        }
        if let Ok(mut titles) = self.session_titles.lock() {
            titles.insert(new_id.clone(), title.clone());
        }
        if let Ok(mut sources) = self.session_title_sources.lock() {
            sources.insert(
                new_id.clone(),
                super::super::session_title::SessionTitleSource::Auto,
            );
        }
        if let Some(workspace_id) = workspace_id.as_ref() {
            if let Ok(mut workspaces) = self.session_workspaces.lock() {
                workspaces.insert(new_id.clone(), workspace_id.clone());
            }
        }

        let mut checkpoint_map = HashMap::new();
        checkpoint_map.insert(new_id.clone(), cloned.clone());
        ensure_conversation_checkpoints_for_sessions(&checkpoint_map);

        let updated_at = cloned
            .iter()
            .map(|message| message.timestamp)
            .max()
            .unwrap_or(0);
        let turn_count = cloned
            .iter()
            .filter(|message| message.role == Role::User)
            .count();
        let estimated_tokens = cloned
            .iter()
            .map(|message| {
                message
                    .estimated_tokens
                    .unwrap_or_else(|| estimate_message_tokens(message))
            })
            .sum();

        Ok(ChatSessionSummary {
            session_id: new_id,
            workspace_id,
            preview: title,
            message_count: cloned.len(),
            turn_count,
            estimated_tokens,
            updated_at,
            archived: false,
        })
    }

    fn taken_session_titles(&self) -> HashSet<String> {
        let mut taken = HashSet::new();
        if let Ok(titles) = self.session_titles.lock() {
            for title in titles.values() {
                let trimmed = title.trim();
                if !trimmed.is_empty() {
                    taken.insert(trimmed.to_string());
                }
            }
        }
        let pool = self.db_pool.clone();
        let summaries = block_on_compat(async move {
            super::super::db::load_session_summaries(&pool)
                .await
                .unwrap_or_default()
        });
        for summary in summaries {
            let trimmed = summary.preview.trim();
            if !trimmed.is_empty() {
                taken.insert(trimmed.to_string());
            }
        }
        taken
    }
}

fn clone_message_for_branch(mut message: ChatMessage, session_id: &str) -> ChatMessage {
    settle_message_in_place(&mut message);
    message.id = format!("msg-{}", uuid::Uuid::new_v4());
    message.session_id = session_id.to_string();
    message
}

pub(super) fn branch_title_stem(title: &str) -> &str {
    let trimmed = title.trim();
    if let Some(pos) = trimmed.rfind(" (") {
        let rest = &trimmed[pos + 2..];
        if let Some(inner) = rest.strip_suffix(')') {
            if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
                return trimmed[..pos].trim_end();
            }
        }
    }
    if let Some((byte_idx, open)) = trimmed.char_indices().rfind(|(_, ch)| *ch == '（') {
        let rest = &trimmed[byte_idx + open.len_utf8()..];
        if let Some(inner) = rest.strip_suffix('）') {
            if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
                return trimmed[..byte_idx].trim_end();
            }
        }
    }
    trimmed
}

pub(super) fn next_branch_title(base: &str, taken: &HashSet<String>) -> String {
    let stem = branch_title_stem(base);
    let stem = if stem.is_empty() { "新对话" } else { stem };
    let mut n = 1u32;
    loop {
        let candidate = format!("{stem} ({n})");
        if !taken.contains(&candidate) {
            return candidate;
        }
        n = n.saturating_add(1);
        if n == u32::MAX {
            return candidate;
        }
    }
}
