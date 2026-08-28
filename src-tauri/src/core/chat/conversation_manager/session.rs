use std::collections::HashMap;

use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::Role;
use crate::models::chat::ChatSessionSummary;

use super::helpers::{block_on_compat, session_preview};
use super::ConversationManager;

impl ConversationManager {
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
                super::super::db::bind_session_workspace(&pool, &session_id, &workspace_id).await
            {
                eprintln!("Failed to bind chat session workspace: {error}");
            }
        });
    }

    pub async fn rebind_workspace(
        &self,
        session_id: &str,
        workspace_id: &str,
    ) -> Result<(), String> {
        let previous = self
            .session_workspaces
            .lock()
            .ok()
            .and_then(|workspaces| workspaces.get(session_id).cloned());
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces.insert(session_id.to_string(), workspace_id.to_string());
        }
        if let Err(error) =
            super::super::db::set_session_workspace(&self.db_pool, session_id, workspace_id).await
        {
            if let Ok(mut workspaces) = self.session_workspaces.lock() {
                match previous {
                    Some(workspace) => {
                        workspaces.insert(session_id.to_string(), workspace);
                    }
                    None => {
                        workspaces.remove(session_id);
                    }
                }
            }
            return Err(error);
        }
        Ok(())
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
            if let Err(error) = super::super::db::save_session_title(&pool, &session_id, &title).await
            {
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

    pub fn list_sessions(&self) -> Vec<ChatSessionSummary> {
        self.list_sessions_filtered(false)
    }

    pub fn list_archived_sessions(&self) -> Vec<ChatSessionSummary> {
        self.list_sessions_filtered(true)
    }

    fn list_sessions_filtered(&self, archived: bool) -> Vec<ChatSessionSummary> {
        let pool = self.db_pool.clone();
        let mut summaries = block_on_compat(async move {
            super::super::db::load_session_summaries(&pool)
                .await
                .unwrap_or_default()
        });

        let session_workspaces = self
            .session_workspaces
            .lock()
            .map(|workspaces| workspaces.clone())
            .unwrap_or_default();
        let session_archived = self
            .session_archived
            .lock()
            .map(|archived| archived.clone())
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
                    archived: session_archived.contains(session_id),
                };
                if let Some(index) = by_id.get(session_id).copied() {
                    summaries[index] = summary;
                } else {
                    by_id.insert(session_id.clone(), summaries.len());
                    summaries.push(summary);
                }
            }
        }

        for summary in summaries.iter_mut() {
            summary.archived = session_archived.contains(&summary.session_id);
            if let Some(workspace_id) = session_workspaces.get(&summary.session_id) {
                summary.workspace_id = Some(workspace_id.clone());
            }
        }

        summaries.retain(|summary| summary.archived == archived);
        summaries.sort_by_key(|summary| std::cmp::Reverse(summary.updated_at));
        summaries
    }

    pub fn set_session_archived(&self, session_id: &str, archived: bool) {
        if let Ok(mut set) = self.session_archived.lock() {
            if archived {
                set.insert(session_id.to_string());
            } else {
                set.remove(session_id);
            }
        }
        let pool = self.db_pool.clone();
        let sid = session_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = super::super::db::set_session_archived(&pool, &sid, archived).await {
                eprintln!("Failed to persist session archive state for {sid}: {error}");
            }
        });
    }

    pub fn set_sessions_archived_for_workspace(&self, workspace_id: &str, archived: bool) {
        let session_ids: Vec<String> = if archived {
            self.list_sessions()
        } else {
            self.list_archived_sessions()
        }
        .into_iter()
        .filter(|session| session.workspace_id.as_deref() == Some(workspace_id))
        .map(|session| session.session_id)
        .collect();
        if session_ids.is_empty() {
            return;
        }

        if let Ok(mut set) = self.session_archived.lock() {
            for session_id in &session_ids {
                if archived {
                    set.insert(session_id.clone());
                } else {
                    set.remove(session_id);
                }
            }
        }

        let pool = self.db_pool.clone();
        let workspace_id = workspace_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                super::super::db::set_sessions_archived_batch(&pool, &session_ids, archived).await
            {
                eprintln!(
                    "Failed to persist workspace archive state for {workspace_id}: {error}"
                );
            }
        });
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
        if let Ok(mut archived) = self.session_archived.lock() {
            archived.remove(session_id);
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
        if let Ok(mut archived) = self.session_archived.lock() {
            archived.clear();
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
}
