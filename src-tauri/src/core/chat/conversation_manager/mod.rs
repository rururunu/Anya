mod activity;
mod branch;
mod helpers;
mod messages;
mod session;

pub use activity::TimelineTextKind;
pub use helpers::create_message;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::core::runtime::ChatMessage;

use crate::core::chat::session_title::SessionTitleSource;

use helpers::{
    block_on_compat, ensure_conversation_checkpoints_for_sessions, settle_orphaned_in_sessions,
};

pub struct ConversationManager {
    sessions: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
    /// Sessions whose message list has been loaded from SQLite (or created fresh).
    loaded_sessions: Arc<Mutex<HashSet<String>>>,
    session_workspaces: Arc<Mutex<HashMap<String, String>>>,
    session_titles: Arc<Mutex<HashMap<String, String>>>,
    session_title_sources: Arc<Mutex<HashMap<String, SessionTitleSource>>>,
    session_archived: Arc<Mutex<HashSet<String>>>,
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
                            crate::core::runtime::MessageStatus::Pending
                                | crate::core::runtime::MessageStatus::Streaming
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
        let session_title_records = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_session_titles(&db_pool)
                    .await
                    .expect("Failed to load chat session titles")
            }
        });
        let mut session_titles = HashMap::new();
        let mut session_title_sources = HashMap::new();
        for (session_id, record) in session_title_records {
            session_titles.insert(session_id.clone(), record.title);
            if let Some(source) = record.source {
                session_title_sources.insert(session_id, source);
            }
        }
        let session_archived = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_session_archived(&db_pool)
                    .await
                    .expect("Failed to load archived chat sessions")
            }
        });

        Self {
            sessions: Arc::new(Mutex::new(sessions)),
            loaded_sessions: Arc::new(Mutex::new(loaded_sessions)),
            session_workspaces: Arc::new(Mutex::new(session_workspaces)),
            session_titles: Arc::new(Mutex::new(session_titles)),
            session_title_sources: Arc::new(Mutex::new(session_title_sources)),
            session_archived: Arc::new(Mutex::new(session_archived)),
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

    #[test]
    fn truncate_work_timeline_drops_retried_tool_activities() {
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
        let stable_len = manager.work_timeline_len(session_id, &message_id);
        manager.upsert_tool_activity(
            session_id,
            &message_id,
            ToolActivity {
                id: "activity-retry".into(),
                subagent_id: None,
                parent_activity_id: None,
                tool_name: "run_parallel_subagents".into(),
                title: "Subagents (3)".into(),
                kind: "other".into(),
                detail: None,
                arguments: None,
                result: None,
                preview: None,
                success: true,
                status: "running".into(),
            },
        );
        manager.upsert_tool_activity(
            session_id,
            &message_id,
            ToolActivity {
                id: "activity-child".into(),
                subagent_id: Some("child-1".into()),
                parent_activity_id: Some("activity-retry".into()),
                tool_name: "web_search".into(),
                title: "Web search".into(),
                kind: "other".into(),
                detail: None,
                arguments: None,
                result: None,
                preview: None,
                success: true,
                status: "running".into(),
            },
        );
        assert_eq!(
            manager.messages(session_id)[0]
                .tool_activities
                .as_ref()
                .map(|items| items.len()),
            Some(2)
        );

        manager.truncate_work_timeline(session_id, &message_id, stable_len);
        let kept = &manager.messages(session_id)[0];
        assert_eq!(
            kept.work_timeline.as_ref().map(|items| items.len()),
            Some(stable_len)
        );
        assert!(kept.tool_activities.is_none());

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

#[cfg(test)]
mod branch_tests {
    use super::branch::{branch_title_stem, next_branch_title};
    use super::{create_message, ConversationManager};
    use crate::core::chat::db;
    use crate::core::chat::session_title::SessionTitleSource;
    use crate::core::runtime::{MessageStatus, Role};
    use std::collections::HashSet;

    fn cleanup_db(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
    }

    #[test]
    fn branch_title_strips_numeric_suffix() {
        assert_eq!(branch_title_stem("Foo"), "Foo");
        assert_eq!(branch_title_stem("Foo (1)"), "Foo");
        assert_eq!(branch_title_stem("Foo (12)"), "Foo");
        assert_eq!(branch_title_stem("Foo（2）"), "Foo");
        assert_eq!(branch_title_stem("Report (draft)"), "Report (draft)");
        assert_eq!(branch_title_stem("  "), "");
    }

    #[test]
    fn next_branch_title_skips_taken() {
        let empty = HashSet::new();
        assert_eq!(next_branch_title("Foo", &empty), "Foo (1)");

        let taken = HashSet::from(["Foo (1)".into()]);
        assert_eq!(next_branch_title("Foo", &taken), "Foo (2)");
        assert_eq!(next_branch_title("Foo (1)", &taken), "Foo (2)");
    }

    #[tokio::test]
    async fn branch_session_clones_history_and_workspace() {
        let db_path = std::env::temp_dir().join(format!(
            "anya-branch-conversation-{}.db",
            uuid::Uuid::new_v4()
        ));
        let manager = ConversationManager::new(db_path.clone());
        let source_id = "session-source";
        let user = create_message(source_id, Role::User, "hello".into(), MessageStatus::Done);
        let assistant = create_message(
            source_id,
            Role::Assistant,
            "world".into(),
            MessageStatus::Done,
        );

        manager
            .sessions
            .lock()
            .unwrap()
            .insert(source_id.into(), vec![user.clone(), assistant.clone()]);
        manager
            .loaded_sessions
            .lock()
            .unwrap()
            .insert(source_id.into());
        for message in [&user, &assistant] {
            db::save_message(&manager.db_pool, message).await.unwrap();
        }
        manager.set_session_title(source_id, "Narrow Relativity".into(), SessionTitleSource::Auto);
        db::save_session_title(&manager.db_pool, source_id, "Narrow Relativity", SessionTitleSource::Auto)
            .await
            .unwrap();
        db::bind_session_workspace(&manager.db_pool, source_id, "ws-1")
            .await
            .unwrap();
        manager
            .session_workspaces
            .lock()
            .unwrap()
            .insert(source_id.into(), "ws-1".into());

        let branched = manager.branch_session(source_id, None).unwrap();
        assert_ne!(branched.session_id, source_id);
        assert_eq!(branched.workspace_id.as_deref(), Some("ws-1"));
        assert_eq!(branched.preview, "Narrow Relativity (1)");
        assert_eq!(branched.message_count, 2);
        assert_eq!(branched.turn_count, 1);

        let cloned = manager.messages(&branched.session_id);
        assert_eq!(cloned.len(), 2);
        assert_ne!(cloned[0].id, user.id);
        assert_eq!(cloned[0].session_id, branched.session_id);
        assert_eq!(cloned[0].content, "hello");
        assert_eq!(cloned[1].content, "world");

        let source = manager.messages(source_id);
        assert_eq!(source.len(), 2);
        assert_eq!(source[0].id, user.id);

        let second = manager.branch_session(source_id, None).unwrap();
        assert_eq!(second.preview, "Narrow Relativity (2)");

        drop(manager);
        let reloaded = ConversationManager::new(db_path.clone());
        let reloaded_messages = reloaded.messages(&branched.session_id);
        assert_eq!(reloaded_messages.len(), 2);
        assert_eq!(
            reloaded.session_title(&branched.session_id).as_deref(),
            Some("Narrow Relativity (1)")
        );
        assert_eq!(
            reloaded
                .workspace_for_session(&branched.session_id)
                .as_deref(),
            Some("ws-1")
        );
        drop(reloaded);
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn branch_session_rejects_empty_history() {
        let db_path =
            std::env::temp_dir().join(format!("anya-branch-empty-{}.db", uuid::Uuid::new_v4()));
        let manager = ConversationManager::new(db_path.clone());
        let error = manager.branch_session("session-empty", None).unwrap_err();
        assert!(error.contains("没有可分支"));
        drop(manager);
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn branch_session_stops_at_message() {
        let db_path =
            std::env::temp_dir().join(format!("anya-branch-until-{}.db", uuid::Uuid::new_v4()));
        let manager = ConversationManager::new(db_path.clone());
        let source_id = "session-source";
        let first_user = create_message(source_id, Role::User, "one".into(), MessageStatus::Done);
        let first_assistant =
            create_message(source_id, Role::Assistant, "a1".into(), MessageStatus::Done);
        let later_user = create_message(source_id, Role::User, "two".into(), MessageStatus::Done);
        let later_assistant =
            create_message(source_id, Role::Assistant, "a2".into(), MessageStatus::Done);

        manager.sessions.lock().unwrap().insert(
            source_id.into(),
            vec![
                first_user.clone(),
                first_assistant.clone(),
                later_user.clone(),
                later_assistant.clone(),
            ],
        );
        manager
            .loaded_sessions
            .lock()
            .unwrap()
            .insert(source_id.into());
        for message in [&first_user, &first_assistant, &later_user, &later_assistant] {
            db::save_message(&manager.db_pool, message).await.unwrap();
        }

        let branched = manager
            .branch_session(source_id, Some(&first_assistant.id))
            .unwrap();
        let cloned = manager.messages(&branched.session_id);
        assert_eq!(cloned.len(), 2);
        assert_eq!(cloned[0].content, "one");
        assert_eq!(cloned[1].content, "a1");
        assert_eq!(manager.messages(source_id).len(), 4);

        drop(manager);
        cleanup_db(&db_path);
    }
}
