use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::chat::error::ChatError;
use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::{ChatMessage, MessageStatus, Role, ToolActivity, WorkTimelineItem};
use crate::models::chat::ChatSessionSummary;

/// Which run of text a streamed chunk belongs to, so it can be merged into the
/// trailing `WorkTimelineItem` of the same kind instead of starting a new one
/// for every delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineTextKind {
    Reasoning,
    Content,
}

fn block_on_compat<F>(future: F) -> F::Output
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

pub struct ConversationManager {
    sessions: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
    /// Sessions whose message list has been loaded from SQLite (or created fresh).
    loaded_sessions: Arc<Mutex<HashSet<String>>>,
    session_workspaces: Arc<Mutex<HashMap<String, String>>>,
    session_titles: Arc<Mutex<HashMap<String, String>>>,
    db_pool: sqlx::SqlitePool,
    journal: super::journal::SessionJournal,
}

impl ConversationManager {
    pub fn new(db_path: std::path::PathBuf) -> Self {
        let db_pool = block_on_compat({
            let db_path = db_path.clone();
            async move {
                super::db::init_db(&db_path)
                    .await
                    .expect("Failed to initialize SQLite database")
            }
        });

        let journal = super::journal::SessionJournal::new(db_pool.clone());

        // Startup only hydrates orphaned mid-turn rows. Full session history is
        // loaded on demand when that session is opened.
        let orphaned = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_orphaned_messages(&db_pool)
                    .await
                    .expect("Failed to load orphaned messages from SQLite")
            }
        });

        let mut sessions: HashMap<String, Vec<ChatMessage>> = HashMap::new();
        let mut loaded_sessions: HashSet<String> = HashSet::new();
        for msg in orphaned {
            loaded_sessions.insert(msg.session_id.clone());
            sessions
                .entry(msg.session_id.clone())
                .or_insert_with(Vec::new)
                .push(msg);
        }

        // Rebuild partial streaming content from journal before settling orphans.
        {
            let pool = db_pool.clone();
            let mut flat: Vec<ChatMessage> = sessions.values().flatten().cloned().collect();
            let flat = block_on_compat({
                let pool = pool.clone();
                async move {
                    let _ = super::journal::hydrate_orphaned_from_journal(&pool, &mut flat).await;
                    flat
                }
            });
            let by_id: HashMap<String, ChatMessage> =
                flat.into_iter().map(|m| (m.id.clone(), m)).collect();
            for messages in sessions.values_mut() {
                for message in messages.iter_mut() {
                    if let Some(hydrated) = by_id.get(&message.id) {
                        if matches!(
                            message.status,
                            MessageStatus::Pending | MessageStatus::Streaming
                        ) {
                            message.content = hydrated.content.clone();
                            message.reasoning = hydrated.reasoning.clone();
                        }
                    }
                }
            }
        }

        // Crash / force-quit can leave pending/streaming messages and running tools.
        // Nothing is still executing after process start, so finalize them now.
        let dirty = settle_orphaned_in_sessions(&mut sessions);
        let pool_for_settle = db_pool.clone();
        let journal_for_settle = journal.clone();
        if !dirty.is_empty() {
            tauri::async_runtime::spawn(async move {
                for message in dirty {
                    if let Err(e) = super::db::save_message(&pool_for_settle, &message).await {
                        eprintln!("Failed to settle interrupted message {}: {}", message.id, e);
                    } else {
                        journal_for_settle.discard_message(&message.id);
                    }
                }
            });
        }

        // Orphans only contain mid-turn rows. Reload each touched session fully
        // so reopen does not show a partial transcript.
        let sessions_needing_full_load: Vec<String> = loaded_sessions.iter().cloned().collect();
        for session_id in sessions_needing_full_load {
            let pool = db_pool.clone();
            let sid = session_id.clone();
            let full = block_on_compat(async move {
                super::db::load_messages_for_session(&pool, &sid)
                    .await
                    .unwrap_or_default()
            });
            // Prefer settled in-memory copies for ids we already finalized.
            let settled_by_id: HashMap<String, ChatMessage> = sessions
                .remove(&session_id)
                .unwrap_or_default()
                .into_iter()
                .map(|m| (m.id.clone(), m))
                .collect();
            let merged: Vec<ChatMessage> = full
                .into_iter()
                .map(|message| settled_by_id.get(&message.id).cloned().unwrap_or(message))
                .collect();
            sessions.insert(session_id, merged);
        }
        // Mid-turn crash used to leave checkpoints only in memory — backfill so
        // the last user message can still be rewound after restart.
        ensure_conversation_checkpoints_for_sessions(&sessions);

        let session_workspaces = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_session_workspaces(&db_pool)
                    .await
                    .expect("Failed to load chat session workspaces")
            }
        });
        let session_titles = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_session_titles(&db_pool)
                    .await
                    .expect("Failed to load chat session titles")
            }
        });

        Self {
            sessions: Arc::new(Mutex::new(sessions)),
            loaded_sessions: Arc::new(Mutex::new(loaded_sessions)),
            session_workspaces: Arc::new(Mutex::new(session_workspaces)),
            session_titles: Arc::new(Mutex::new(session_titles)),
            db_pool,
            journal,
        }
    }

    pub fn journal(&self) -> &super::journal::SessionJournal {
        &self.journal
    }

    pub fn db_pool(&self) -> sqlx::SqlitePool {
        self.db_pool.clone()
    }

    pub fn inner(&self) -> Arc<Mutex<HashMap<String, Vec<ChatMessage>>>> {
        Arc::clone(&self.sessions)
    }

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
            super::db::load_messages_for_session(&pool, &sid)
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
                    if let Err(e) = super::db::save_message(&pool, &message).await {
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
            if let Err(e) = super::db::save_message(&pool, &message).await {
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
            if let Err(e) = super::db::save_message(&pool, &summary).await {
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

    pub fn bind_workspace(&self, session_id: &str, workspace_id: &str) {
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces
                .entry(session_id.to_string())
                .or_insert_with(|| workspace_id.to_string());
        }
        let pool = self.db_pool.clone();
        let session_id = session_id.to_string();
        let workspace_id = workspace_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                super::db::bind_session_workspace(&pool, &session_id, &workspace_id).await
            {
                eprintln!("Failed to bind chat session workspace: {error}");
            }
        });
    }

    pub fn workspace_for_session(&self, session_id: &str) -> Option<String> {
        self.session_workspaces
            .lock()
            .ok()
            .and_then(|workspaces| workspaces.get(session_id).cloned())
    }

    pub fn set_session_title(&self, session_id: &str, title: String) {
        let title = title.trim().to_string();
        if title.is_empty() {
            return;
        }
        if let Ok(mut titles) = self.session_titles.lock() {
            titles.insert(session_id.to_string(), title.clone());
        }
        let pool = self.db_pool.clone();
        let session_id = session_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = super::db::save_session_title(&pool, &session_id, &title).await {
                eprintln!("Failed to save chat session title: {error}");
            }
        });
    }

    pub fn session_title(&self, session_id: &str) -> Option<String> {
        self.session_titles
            .lock()
            .ok()
            .and_then(|titles| titles.get(session_id).cloned())
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

    pub fn list_sessions(&self) -> Vec<ChatSessionSummary> {
        let pool = self.db_pool.clone();
        let mut summaries = block_on_compat(async move {
            super::db::load_session_summaries(&pool)
                .await
                .unwrap_or_default()
        });

        let session_workspaces = self
            .session_workspaces
            .lock()
            .map(|workspaces| workspaces.clone())
            .unwrap_or_default();

        // Overlay in-memory loaded sessions so an active stream's preview /
        // counts stay accurate before the terminal SQLite flush.
        if let Ok(sessions) = self.sessions.lock() {
            let mut by_id: HashMap<String, usize> = summaries
                .iter()
                .enumerate()
                .map(|(index, summary)| (summary.session_id.clone(), index))
                .collect();

            for (session_id, messages) in sessions.iter() {
                if messages.is_empty() {
                    continue;
                }
                let preview = self
                    .session_title(session_id)
                    .unwrap_or_else(|| session_preview(messages));
                let updated_at = messages
                    .iter()
                    .map(|message| message.timestamp)
                    .max()
                    .unwrap_or(0);
                let turn_count = messages
                    .iter()
                    .filter(|message| message.role == Role::User)
                    .count();
                let estimated_tokens = messages
                    .iter()
                    .map(|message| {
                        message
                            .estimated_tokens
                            .unwrap_or_else(|| estimate_message_tokens(message))
                    })
                    .sum();
                let summary = ChatSessionSummary {
                    session_id: session_id.clone(),
                    workspace_id: session_workspaces.get(session_id).cloned().or_else(|| {
                        summaries
                            .iter()
                            .find(|item| item.session_id == *session_id)
                            .and_then(|item| item.workspace_id.clone())
                    }),
                    preview,
                    message_count: messages.len(),
                    turn_count,
                    estimated_tokens,
                    updated_at,
                };
                if let Some(index) = by_id.get(session_id).copied() {
                    summaries[index] = summary;
                } else {
                    by_id.insert(session_id.clone(), summaries.len());
                    summaries.push(summary);
                }
            }
        }

        summaries.sort_by_key(|summary| std::cmp::Reverse(summary.updated_at));
        summaries
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
                if let Err(e) = super::db::save_message(&pool, &msg_to_save).await {
                    eprintln!("Failed to save updated message to SQLite: {}", e);
                } else {
                    journal.discard_message(&msg_to_save.id);
                }
            });
        }

        Some(updated)
    }

    /// Append a streamed chunk to the message's chronological work timeline,
    /// merging into the trailing entry when it is the same kind so reasoning
    /// and reply text each collapse into one run instead of one item per
    /// delta. This only updates the in-memory copy — like the raw
    /// `content`/`reasoning` accumulators, the timeline is flushed to SQLite
    /// by the terminal `update_message` call once the turn settles, and
    /// recovered from the journal on crash in the meantime.
    pub fn append_work_timeline_text(
        &self,
        session_id: &str,
        message_id: &str,
        kind: TimelineTextKind,
        chunk: &str,
    ) {
        if chunk.is_empty() {
            return;
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(message) = sessions
            .get_mut(session_id)
            .and_then(|messages| messages.iter_mut().find(|item| item.id == message_id))
        else {
            return;
        };
        let timeline = message.work_timeline.get_or_insert_with(Vec::new);
        let next_index = timeline.len();
        if let Some(last) = timeline.last_mut() {
            let merged = match (last, kind) {
                (WorkTimelineItem::Reasoning { content, .. }, TimelineTextKind::Reasoning) => {
                    content.push_str(chunk);
                    true
                }
                (WorkTimelineItem::Content { content, .. }, TimelineTextKind::Content) => {
                    content.push_str(chunk);
                    true
                }
                _ => false,
            };
            if merged {
                return;
            }
        }
        let item = match kind {
            TimelineTextKind::Reasoning => WorkTimelineItem::Reasoning {
                id: format!("{message_id}-reasoning-{next_index}"),
                content: chunk.to_string(),
            },
            TimelineTextKind::Content => WorkTimelineItem::Content {
                id: format!("{message_id}-content-{next_index}"),
                content: chunk.to_string(),
            },
        };
        timeline.push(item);
    }

    pub fn work_timeline_len(&self, session_id: &str, message_id: &str) -> usize {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(session_id)?
                    .iter()
                    .find(|item| item.id == message_id)?
                    .work_timeline
                    .as_ref()
                    .map(|timeline| timeline.len())
            })
            .unwrap_or(0)
    }

    /// Keep timeline items produced before `keep_len` (completed prior rounds)
    /// and drop anything streamed during a failed/retried provider attempt.
    pub fn truncate_work_timeline(&self, session_id: &str, message_id: &str, keep_len: usize) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(message) = sessions
                .get_mut(session_id)
                .and_then(|messages| messages.iter_mut().find(|item| item.id == message_id))
            {
                if let Some(timeline) = message.work_timeline.as_mut() {
                    if keep_len == 0 {
                        message.work_timeline = None;
                    } else if timeline.len() > keep_len {
                        timeline.truncate(keep_len);
                    }
                }
            }
        }
    }

    pub fn upsert_tool_activity(
        &self,
        session_id: &str,
        message_id: &str,
        activity: ToolActivity,
    ) -> Option<ChatMessage> {
        self.ensure_session_loaded(session_id);
        let should_persist = activity.status != "running";
        let mut sessions = self.sessions.lock().ok()?;
        let message = sessions
            .get_mut(session_id)?
            .iter_mut()
            .find(|item| item.id == message_id)?;
        let activities = message.tool_activities.get_or_insert_with(Vec::new);
        if let Some(existing) = activities.iter_mut().find(|item| item.id == activity.id) {
            *existing = activity;
        } else {
            // Anchor the tool card at the point it actually started, right
            // after whatever narration preceded it, instead of grouping all
            // tool activity separately from the text that led up to it.
            let timeline_entry = WorkTimelineItem::Tool {
                id: format!("{}-tool-{}", message_id, activity.id),
                tool_activity_id: activity.id.clone(),
            };
            activities.push(activity);
            message
                .work_timeline
                .get_or_insert_with(Vec::new)
                .push(timeline_entry);
        }
        if !should_persist {
            message.estimated_tokens = None;
            return Some(message.clone());
        }
        refresh_message_token_cache(message);
        let updated = message.clone();
        let pool = self.db_pool.clone();
        let msg_to_save = updated.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::db::save_message_tool_fields(&pool, &msg_to_save).await {
                eprintln!("Failed to save tool activity to SQLite: {}", e);
            }
        });
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

    pub fn delete_session(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(session_id);
        }
        if let Ok(mut loaded) = self.loaded_sessions.lock() {
            loaded.remove(session_id);
        }
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces.remove(session_id);
        }

        let pool = self.db_pool.clone();
        let sid = session_id.to_string();
        self.journal.discard_session(session_id);
        tauri::async_runtime::spawn(async move {
            let result = async {
                let mut transaction = pool.begin().await?;
                sqlx::query("DELETE FROM chat_journal_events WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_messages WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_sessions WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                transaction.commit().await?;
                // No-op for legacy databases; new databases use incremental
                // auto-vacuum so deletion can return a bounded batch.
                sqlx::query("PRAGMA incremental_vacuum(1024)")
                    .execute(&pool)
                    .await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("Failed to delete session {sid} from SQLite: {error}");
            }
        });
    }

    pub fn clear_all_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.clear();
        }
        if let Ok(mut loaded) = self.loaded_sessions.lock() {
            loaded.clear();
        }
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces.clear();
        }

        let pool = self.db_pool.clone();
        self.journal.discard_all();
        tauri::async_runtime::spawn(async move {
            let result = async {
                let mut transaction = pool.begin().await?;
                sqlx::query("DELETE FROM chat_journal_events")
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_messages")
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_sessions")
                    .execute(&mut *transaction)
                    .await?;
                transaction.commit().await?;
                sqlx::query("PRAGMA incremental_vacuum(1024)")
                    .execute(&pool)
                    .await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("Failed to clear chat history in SQLite: {error}");
            }
        });
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
                if let Err(e) = super::db::save_message(&pool, &msg_to_save).await {
                    eprintln!("Failed to save settled message {}: {}", msg_to_save.id, e);
                }
            });
            return Some((session_id.clone(), updated));
        }
        None
    }
}

fn settle_orphaned_in_sessions(
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
fn ensure_conversation_checkpoints_for_sessions(sessions: &HashMap<String, Vec<ChatMessage>>) {
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

fn settle_message_in_place(message: &mut ChatMessage) -> bool {
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

fn session_preview(messages: &[ChatMessage]) -> String {
    for message in messages {
        if matches!(message.role, Role::User) {
            let trimmed = super::selection::visible_user_text(&message.content);
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

fn refresh_message_token_cache(message: &mut ChatMessage) {
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

fn lock_error<T: std::fmt::Display>(error: T) -> ChatError {
    ChatError::Internal(error.to_string())
}

#[cfg(test)]
mod rewind_tests {
    use super::{create_message, ConversationManager};
    use crate::core::chat::db;
    use crate::core::runtime::{MessageStatus, Role};

    #[tokio::test]
    async fn truncate_is_persisted_before_returning() {
        let db_path = std::env::temp_dir().join(format!(
            "anya-rewind-conversation-{}.db",
            uuid::Uuid::new_v4()
        ));
        let manager = ConversationManager::new(db_path.clone());
        let session_id = "session";
        let first = create_message(session_id, Role::User, "keep".into(), MessageStatus::Done);
        let rewind_from =
            create_message(session_id, Role::User, "rewind".into(), MessageStatus::Done);
        let assistant = create_message(
            session_id,
            Role::Assistant,
            "answer".into(),
            MessageStatus::Done,
        );

        manager.sessions.lock().unwrap().insert(
            session_id.into(),
            vec![first.clone(), rewind_from.clone(), assistant.clone()],
        );
        for message in [&first, &rewind_from, &assistant] {
            db::save_message(&manager.db_pool, message).await.unwrap();
        }
        for message in [&rewind_from, &assistant] {
            sqlx::query(
                "INSERT INTO chat_journal_events
                 (session_id, turn_id, message_id, kind, payload_json, created_at)
                 VALUES (?, 'turn', ?, 'delta', '{}', 0)",
            )
            .bind(session_id)
            .bind(&message.id)
            .execute(&manager.db_pool)
            .await
            .unwrap();
        }

        manager
            .truncate_from_message(session_id, &rewind_from.id)
            .await
            .unwrap();
        assert_eq!(manager.messages(session_id), vec![first.clone()]);
        let journal_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_journal_events WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(&manager.db_pool)
                .await
                .unwrap();
        assert_eq!(journal_count, 0);
        drop(manager);

        let reloaded = ConversationManager::new(db_path.clone());
        let reloaded_messages = reloaded.messages(session_id);
        assert_eq!(reloaded_messages.len(), 1);
        assert_eq!(reloaded_messages[0].id, first.id);
        assert_eq!(reloaded_messages[0].content, first.content);
        assert!(reloaded_messages[0].estimated_tokens.is_some());
        drop(reloaded);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    }
}

#[cfg(test)]
mod work_timeline_tests {
    use super::{create_message, ConversationManager, TimelineTextKind};
    use crate::core::runtime::{MessageStatus, Role, ToolActivity, WorkTimelineItem};

    fn temp_manager() -> (ConversationManager, std::path::PathBuf) {
        let db_path =
            std::env::temp_dir().join(format!("anya-work-timeline-{}.db", uuid::Uuid::new_v4()));
        (ConversationManager::new(db_path.clone()), db_path)
    }

    fn cleanup(db_path: std::path::PathBuf) {
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    }

    /// Reasoning text, then a tool call, then reply text must land in that
    /// exact order in the timeline — this is the ordering the UI renders
    /// directly, so a regression here reproduces the "all thinking, then all
    /// tools" bug the timeline was built to fix.
    #[test]
    fn interleaves_reasoning_tool_and_content_in_call_order() {
        let (manager, db_path) = temp_manager();
        let session_id = "session";
        let message = create_message(
            session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Streaming,
        );
        let message_id = message.id.clone();
        manager
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.into(), vec![message]);

        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Reasoning,
            "let me check the file",
        );
        manager.upsert_tool_activity(
            session_id,
            &message_id,
            ToolActivity {
                id: "activity-1".into(),
                subagent_id: None,
                parent_activity_id: None,
                tool_name: "read_file".into(),
                title: "Read file".into(),
                kind: "read".into(),
                detail: None,
                arguments: None,
                result: None,
                preview: None,
                success: true,
                status: "running".into(),
            },
        );
        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Content,
            "the file looks fine",
        );

        let messages = manager.messages(session_id);
        let timeline = messages[0].work_timeline.clone().expect("timeline");
        assert_eq!(timeline.len(), 3);
        assert!(
            matches!(&timeline[0], WorkTimelineItem::Reasoning { content, .. } if content == "let me check the file")
        );
        assert!(
            matches!(&timeline[1], WorkTimelineItem::Tool { tool_activity_id, .. } if tool_activity_id == "activity-1")
        );
        assert!(
            matches!(&timeline[2], WorkTimelineItem::Content { content, .. } if content == "the file looks fine")
        );

        cleanup(db_path);
    }

    /// Consecutive deltas of the same kind must merge into one run instead of
    /// fragmenting into one timeline item per chunk.
    #[test]
    fn merges_consecutive_deltas_of_the_same_kind() {
        let (manager, db_path) = temp_manager();
        let session_id = "session";
        let message = create_message(
            session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Streaming,
        );
        let message_id = message.id.clone();
        manager
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.into(), vec![message]);

        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Reasoning,
            "step one, ",
        );
        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Reasoning,
            "step two",
        );

        let messages = manager.messages(session_id);
        let timeline = messages[0].work_timeline.clone().expect("timeline");
        assert_eq!(timeline.len(), 1);
        assert!(
            matches!(&timeline[0], WorkTimelineItem::Reasoning { content, .. } if content == "step one, step two")
        );

        cleanup(db_path);
    }

    /// A tool activity that transitions running -> done must update the same
    /// timeline entry rather than appending a duplicate.
    #[test]
    fn tool_activity_updates_do_not_duplicate_timeline_entries() {
        let (manager, db_path) = temp_manager();
        let session_id = "session";
        let message = create_message(
            session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Streaming,
        );
        let message_id = message.id.clone();
        manager
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.into(), vec![message]);

        let running = ToolActivity {
            id: "activity-1".into(),
            subagent_id: None,
            parent_activity_id: None,
            tool_name: "read_file".into(),
            title: "Read file".into(),
            kind: "read".into(),
            detail: None,
            arguments: None,
            result: None,
            preview: None,
            success: true,
            status: "running".into(),
        };
        let mut done = running.clone();
        done.status = "done".into();

        manager.upsert_tool_activity(session_id, &message_id, running);
        manager.upsert_tool_activity(session_id, &message_id, done);

        let messages = manager.messages(session_id);
        let timeline = messages[0].work_timeline.clone().expect("timeline");
        assert_eq!(timeline.len(), 1);

        cleanup(db_path);
    }

    #[test]
    fn truncate_work_timeline_keeps_stable_prefix() {
        let (manager, db_path) = temp_manager();
        let session_id = "session";
        let message = create_message(
            session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Streaming,
        );
        let message_id = message.id.clone();
        manager
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.into(), vec![message]);

        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Reasoning,
            "stable thought",
        );
        manager.upsert_tool_activity(
            session_id,
            &message_id,
            ToolActivity {
                id: "activity-1".into(),
                subagent_id: None,
                parent_activity_id: None,
                tool_name: "read_file".into(),
                title: "Read file".into(),
                kind: "read".into(),
                detail: None,
                arguments: None,
                result: None,
                preview: None,
                success: true,
                status: "running".into(),
            },
        );
        let stable_len = manager.work_timeline_len(session_id, &message_id);
        manager.append_work_timeline_text(
            session_id,
            &message_id,
            TimelineTextKind::Reasoning,
            "retry partial",
        );
        assert_eq!(
            manager.work_timeline_len(session_id, &message_id),
            stable_len + 1
        );

        manager.truncate_work_timeline(session_id, &message_id, stable_len);
        let timeline = manager.messages(session_id)[0]
            .work_timeline
            .clone()
            .expect("timeline");
        assert_eq!(timeline.len(), stable_len);
        assert!(matches!(
            &timeline[0],
            WorkTimelineItem::Reasoning { content, .. } if content == "stable thought"
        ));

        cleanup(db_path);
    }
}

#[cfg(test)]
mod lazy_load_tests {
    use super::{create_message, ConversationManager};
    use crate::core::chat::db;
    use crate::core::runtime::{MessageStatus, Role};

    #[tokio::test]
    async fn startup_does_not_load_all_sessions_until_opened() {
        let db_path =
            std::env::temp_dir().join(format!("anya-lazy-load-{}.db", uuid::Uuid::new_v4()));
        let manager = ConversationManager::new(db_path.clone());
        let keep = create_message("keep", Role::User, "one".into(), MessageStatus::Done);
        let other = create_message("other", Role::User, "two".into(), MessageStatus::Done);
        db::save_message(&manager.db_pool, &keep).await.unwrap();
        db::save_message(&manager.db_pool, &other).await.unwrap();
        drop(manager);

        let reloaded = ConversationManager::new(db_path.clone());
        {
            let sessions = reloaded.sessions.lock().unwrap();
            assert!(sessions.is_empty(), "startup should not preload history");
        }
        let summaries = reloaded.list_sessions();
        assert_eq!(summaries.len(), 2);

        let loaded = reloaded.messages("keep");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "one");
        {
            let sessions = reloaded.sessions.lock().unwrap();
            assert!(sessions.contains_key("keep"));
            assert!(!sessions.contains_key("other"));
        }

        let other_loaded = reloaded.messages("other");
        assert_eq!(other_loaded.len(), 1);
        assert_eq!(other_loaded[0].content, "two");
        drop(reloaded);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    }
}
