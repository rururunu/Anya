use std::collections::{HashMap, HashSet};

use crate::core::chat::error::ChatError;
use crate::core::runtime::{ChatMessage, MessageStatus};

use super::helpers::{
    block_on_compat, ensure_conversation_checkpoints_for_sessions, lock_error,
    refresh_message_token_cache, settle_message_in_place,
};
use super::ConversationManager;

impl ConversationManager {
    /// Load one session's messages from SQLite the first time it is opened.
    pub fn ensure_session_loaded(&self, session_id: &str) {
        if session_id.is_empty() {
            return;
        }
        if let Ok(loaded) = self.loaded_sessions.lock() {
            if loaded.contains(session_id) {
                return;
            }
        }

        let pool = self.db_pool.clone();
        let sid = session_id.to_string();
        let mut messages = block_on_compat(async move {
            super::super::db::load_messages_for_session(&pool, &sid)
                .await
                .unwrap_or_default()
        });

        // Settle any mid-turn leftovers discovered when the session is opened.
        let dirty = {
            let mut dirty = Vec::new();
            for message in messages.iter_mut() {
                if settle_message_in_place(message) {
                    dirty.push(message.clone());
                }
            }
            dirty
        };
        if !dirty.is_empty() {
            let pool = self.db_pool.clone();
            let journal = self.journal.clone();
            tauri::async_runtime::spawn(async move {
                for message in dirty {
                    if let Err(e) = super::super::db::save_message(&pool, &message).await {
                        eprintln!("Failed to settle interrupted message {}: {}", message.id, e);
                    } else {
                        journal.discard_message(&message.id);
                    }
                }
            });
        }

        let mut checkpoint_map = HashMap::new();
        checkpoint_map.insert(session_id.to_string(), messages.clone());
        ensure_conversation_checkpoints_for_sessions(&checkpoint_map);

        if let (Ok(mut sessions), Ok(mut loaded)) =
            (self.sessions.lock(), self.loaded_sessions.lock())
        {
            if loaded.contains(session_id) {
                return;
            }
            // Keep any messages appended while the DB load was in flight.
            match sessions.get_mut(session_id) {
                Some(existing) if !existing.is_empty() => {
                    let existing_ids: HashSet<String> =
                        existing.iter().map(|m| m.id.clone()).collect();
                    for message in messages {
                        if !existing_ids.contains(&message.id) {
                            existing.push(message);
                        }
                    }
                    existing.sort_by_key(|message| message.timestamp);
                }
                _ => {
                    sessions.insert(session_id.to_string(), messages);
                }
            }
            loaded.insert(session_id.to_string());
        }
    }

    pub fn append(&self, session_id: &str, mut message: ChatMessage) {
        self.ensure_session_loaded(session_id);
        refresh_message_token_cache(&mut message);
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions
                .entry(session_id.to_string())
                .or_default()
                .push(message.clone());
        }
        if let Ok(mut loaded) = self.loaded_sessions.lock() {
            loaded.insert(session_id.to_string());
        }

        // Save to database asynchronously
        let pool = self.db_pool.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::super::db::save_message(&pool, &message).await {
                eprintln!("Failed to save message to SQLite: {}", e);
            }
        });
    }

    /// Insert a compaction summary before `before_id` (or at the end).
    /// No-op when a row with the same id already exists.
    pub fn insert_compaction_summary(
        &self,
        session_id: &str,
        mut summary: ChatMessage,
        before_id: Option<&str>,
    ) {
        self.ensure_session_loaded(session_id);
        if let Ok(sessions) = self.sessions.lock() {
            if sessions
                .get(session_id)
                .is_some_and(|messages| messages.iter().any(|message| message.id == summary.id))
            {
                return;
            }
        }

        refresh_message_token_cache(&mut summary);

        let insert_at = {
            let sessions = self.sessions.lock().ok();
            let messages = sessions.as_ref().and_then(|map| map.get(session_id));
            let len = messages.map(|list| list.len()).unwrap_or(0);
            let idx = before_id
                .and_then(|id| {
                    messages.and_then(|list| list.iter().position(|message| message.id == id))
                })
                .unwrap_or(len);
            if idx > 0 {
                if let Some(list) = messages {
                    let prev = list[idx - 1].timestamp;
                    let next = list
                        .get(idx)
                        .map(|message| message.timestamp)
                        .unwrap_or(prev.saturating_add(2));
                    summary.timestamp = prev.saturating_add(1).min(next);
                }
            } else if let Some(next) = messages.and_then(|list| list.first()) {
                summary.timestamp = next.timestamp.saturating_sub(1);
            }
            idx
        };

        if let Ok(mut sessions) = self.sessions.lock() {
            let entry = sessions.entry(session_id.to_string()).or_default();
            let idx = insert_at.min(entry.len());
            entry.insert(idx, summary.clone());
        }
        if let Ok(mut loaded) = self.loaded_sessions.lock() {
            loaded.insert(session_id.to_string());
        }

        let pool = self.db_pool.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::super::db::save_message(&pool, &summary).await {
                eprintln!("Failed to save compaction summary to SQLite: {}", e);
            }
        });
    }

    pub fn messages(&self, session_id: &str) -> Vec<ChatMessage> {
        self.ensure_session_loaded(session_id);
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).cloned())
            .unwrap_or_default()
    }

    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        Ok(self.messages(session_id))
    }

    pub fn find_message(&self, message_id: &str) -> Result<(String, ChatMessage), ChatError> {
        {
            let sessions = self.sessions.lock().map_err(lock_error)?;
            for (session_id, messages) in sessions.iter() {
                if let Some(message) = messages.iter().find(|item| item.id == message_id) {
                    return Ok((session_id.clone(), message.clone()));
                }
            }
        }

        // Message may live in an unloaded session (e.g. cancel after restart).
        let pool = self.db_pool.clone();
        let mid = message_id.to_string();
        let session_id = block_on_compat(async move {
            sqlx::query_scalar::<_, String>("SELECT session_id FROM chat_messages WHERE id = ?")
                .bind(mid)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
        });
        let Some(session_id) = session_id else {
            return Err(ChatError::MessageNotFound);
        };
        self.ensure_session_loaded(&session_id);
        let sessions = self.sessions.lock().map_err(lock_error)?;
        sessions
            .get(&session_id)
            .and_then(|messages| {
                messages
                    .iter()
                    .find(|item| item.id == message_id)
                    .map(|message| (session_id.clone(), message.clone()))
            })
            .ok_or(ChatError::MessageNotFound)
    }

    pub fn update_message(
        &self,
        session_id: &str,
        message_id: &str,
        status: MessageStatus,
        content: Option<String>,
        reasoning: Option<Option<String>>,
    ) -> Option<ChatMessage> {
        self.ensure_session_loaded(session_id);
        let mut sessions = self.sessions.lock().ok()?;
        let messages = sessions.get_mut(session_id)?;
        let message = messages.iter_mut().find(|item| item.id == message_id)?;

        let mut updated = message.clone().with_status(status);
        if let Some(content) = content {
            updated = updated.with_content(content);
        }
        if let Some(reasoning) = reasoning {
            updated.reasoning = reasoning;
        }
        refresh_message_token_cache(&mut updated);
        *message = updated.clone();

        // Streaming/pending rows stay in memory + journal; only terminal (or
        // already-settled) updates rewrite the full SQLite row.
        let should_persist = !matches!(
            &updated.status,
            MessageStatus::Pending | MessageStatus::Streaming
        );
        if should_persist {
            let pool = self.db_pool.clone();
            let journal = self.journal.clone();
            let msg_to_save = updated.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = super::super::db::save_message(&pool, &msg_to_save).await {
                    eprintln!("Failed to save updated message to SQLite: {}", e);
                } else {
                    journal.discard_message(&msg_to_save.id);
                }
            });
        }

        Some(updated)
    }

    /// Remove `user_message_id` and every message after it in the session.
    pub async fn truncate_from_message(
        &self,
        session_id: &str,
        user_message_id: &str,
    ) -> Result<(), ChatError> {
        self.ensure_session_loaded(session_id);
        let removed_ids = {
            let mut sessions = self.sessions.lock().map_err(lock_error)?;
            let messages = sessions
                .get_mut(session_id)
                .ok_or(ChatError::MessageNotFound)?;
            let Some(index) = messages.iter().position(|m| m.id == user_message_id) else {
                return Err(ChatError::MessageNotFound);
            };
            let removed: Vec<String> = messages[index..].iter().map(|m| m.id.clone()).collect();
            messages.truncate(index);
            removed
        };

        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        for id in &removed_ids {
            sqlx::query("DELETE FROM chat_journal_events WHERE message_id = ?;")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ChatError::Internal(error.to_string()))?;
            sqlx::query("DELETE FROM chat_messages WHERE id = ?;")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ChatError::Internal(error.to_string()))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        for id in removed_ids {
            self.journal.discard_message(&id);
        }
        Ok(())
    }

    /// Finalize a message that is no longer backed by an active stream task
    /// (e.g. app was killed mid-run, then user hits pause on the restored chat).
    pub fn settle_interrupted_message(&self, message_id: &str) -> Option<(String, ChatMessage)> {
        let mut sessions = self.sessions.lock().ok()?;
        for (session_id, messages) in sessions.iter_mut() {
            let Some(message) = messages.iter_mut().find(|item| item.id == message_id) else {
                continue;
            };
            if !settle_message_in_place(message) {
                return None;
            }
            let updated = message.clone();
            let pool = self.db_pool.clone();
            let msg_to_save = updated.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = super::super::db::save_message(&pool, &msg_to_save).await {
                    eprintln!("Failed to save settled message {}: {}", msg_to_save.id, e);
                }
            });
            return Some((session_id.clone(), updated));
        }
        None
    }
}
