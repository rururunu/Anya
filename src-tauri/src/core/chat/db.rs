use crate::core::runtime::stream::ToolCallPayload;
use crate::core::runtime::{ChatMessage, MessageStatus, Role, ToolActivity};
use crate::core::token::TokenUsage;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub async fn init_db(db_path: &Path) -> Result<SqlitePool, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let is_new_database = std::fs::metadata(db_path)
        .map(|metadata| metadata.len() == 0)
        .unwrap_or(true);
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(|e| e.to_string())?;

    // This must be selected before the first table is created. It lets new
    // databases return free pages gradually without a blocking full VACUUM.
    if is_new_database {
        sqlx::query("PRAGMA auto_vacuum = INCREMENTAL")
            .execute(&pool)
            .await
            .map_err(|error| error.to_string())?;
    }

    // Create messages table if not exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            reasoning TEXT,
            tool_activities TEXT,
            tool_calls TEXT,
            tool_call_id TEXT,
            name TEXT,
            status TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            estimated_tokens INTEGER
        );",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let message_columns = sqlx::query("PRAGMA table_info(chat_messages)")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "tool_activities")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN tool_activities TEXT")
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "estimated_tokens")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN estimated_tokens INTEGER")
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "work_timeline")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN work_timeline TEXT")
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Create index on session_id for faster history lookup
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_ts
         ON chat_messages(session_id, timestamp);",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_status
         ON chat_messages(status);",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    init_chat_session_schema(&pool).await?;
    crate::core::chat::journal::init_journal_schema(&pool).await?;
    crate::core::chat::journal::compact_recovery_journal(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS token_usage_records (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            model TEXT NOT NULL,
            provider TEXT,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            system_tokens INTEGER NOT NULL,
            context_tokens INTEGER NOT NULL,
            tool_call_tokens INTEGER NOT NULL,
            tool_result_tokens INTEGER NOT NULL,
            memory_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            accuracy TEXT NOT NULL,
            source TEXT,
            recorded_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_token_usage_recorded_at ON token_usage_records(recorded_at)",
        "CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage_records(model)",
    ] {
        sqlx::query(statement).execute(&pool).await.map_err(|e| e.to_string())?;
    }

    Ok(pool)
}

async fn init_chat_session_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            session_id TEXT PRIMARY KEY,
            workspace_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let columns = sqlx::query("PRAGMA table_info(chat_sessions)")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let column_names = columns
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    if !column_names.iter().any(|name| name == "workspace_id") {
        sqlx::query("ALTER TABLE chat_sessions ADD COLUMN workspace_id TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if column_names.iter().any(|name| name == "workspace_path") {
        sqlx::query(
            "UPDATE chat_sessions SET workspace_id = workspace_path
             WHERE workspace_id IS NULL AND workspace_path IS NOT NULL",
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }
    if !column_names.iter().any(|name| name == "title") {
        sqlx::query("ALTER TABLE chat_sessions ADD COLUMN title TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn load_session_workspaces(pool: &SqlitePool) -> Result<HashMap<String, String>, String> {
    let rows = sqlx::query(
        "SELECT session_id, workspace_id FROM chat_sessions WHERE workspace_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("session_id"), row.get("workspace_id")))
        .collect())
}

pub async fn bind_session_workspace(
    pool: &SqlitePool,
    session_id: &str,
    workspace_id: &str,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT OR IGNORE INTO chat_sessions
         (session_id, workspace_id, created_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(workspace_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub async fn load_session_titles(pool: &SqlitePool) -> Result<HashMap<String, String>, String> {
    let rows = sqlx::query(
        "SELECT session_id, title FROM chat_sessions
         WHERE title IS NOT NULL AND trim(title) != ''",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("session_id"), row.get("title")))
        .collect())
}

pub async fn save_session_title(
    pool: &SqlitePool,
    session_id: &str,
    title: &str,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT INTO chat_sessions (session_id, title, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
             title = excluded.title,
             updated_at = excluded.updated_at",
    )
    .bind(session_id)
    .bind(title)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod session_workspace_tests {
    use super::*;

    #[tokio::test]
    async fn migrates_legacy_workspace_path_and_preserves_first_binding() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_sessions (
                session_id TEXT PRIMARY KEY,
                workspace_path TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO chat_sessions VALUES ('legacy', 'D:\\Code\\Peek', 1, 1)")
            .execute(&pool)
            .await
            .unwrap();

        init_chat_session_schema(&pool).await.unwrap();
        bind_session_workspace(&pool, "new", "D:\\Code\\VueAdmin")
            .await
            .unwrap();
        bind_session_workspace(&pool, "new", "D:\\Code\\Other")
            .await
            .unwrap();
        let workspaces = load_session_workspaces(&pool).await.unwrap();

        assert_eq!(workspaces["legacy"], "D:\\Code\\Peek");
        assert_eq!(workspaces["new"], "D:\\Code\\VueAdmin");
    }

    #[tokio::test]
    async fn migrates_title_column_and_persists_titles() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Legacy schema without a title column must be migrated by init.
        sqlx::query(
            "CREATE TABLE chat_sessions (
                session_id TEXT PRIMARY KEY,
                workspace_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_chat_session_schema(&pool).await.unwrap();
        save_session_title(&pool, "s1", "修复工具循环熔断")
            .await
            .unwrap();
        save_session_title(&pool, "s1", "更新标题").await.unwrap();
        save_session_title(&pool, "s2", "  ").await.unwrap();

        let titles = load_session_titles(&pool).await.unwrap();
        assert_eq!(titles["s1"], "更新标题");
        assert!(!titles.contains_key("s2"));
    }
}

const MESSAGE_SELECT_COLUMNS: &str = "id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp, estimated_tokens, work_timeline";

pub async fn save_message(pool: &SqlitePool, msg: &ChatMessage) -> Result<(), String> {
    let role_str = match msg.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    let status_str = match msg.status {
        MessageStatus::Pending => "pending",
        MessageStatus::Streaming => "streaming",
        MessageStatus::Done => "done",
        MessageStatus::Error => "error",
        MessageStatus::Cancelled => "cancelled",
    };

    let tool_calls_json = serialize_tool_calls(msg.tool_calls.as_ref());
    let tool_activities_json = serialize_tool_activities(msg.tool_activities.as_ref());
    let work_timeline_json = serialize_work_timeline(msg.work_timeline.as_ref());
    let timestamp_val = msg.timestamp as i64;
    let estimated_tokens = if matches!(
        msg.status,
        MessageStatus::Pending | MessageStatus::Streaming
    ) {
        None
    } else {
        Some(
            msg.estimated_tokens
                .unwrap_or_else(|| crate::core::chat::limits::estimate_message_tokens(msg))
                as i64,
        )
    };

    sqlx::query(
        "INSERT OR REPLACE INTO chat_messages (
            id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp, estimated_tokens, work_timeline
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"
    )
    .bind(&msg.id)
    .bind(&msg.session_id)
    .bind(role_str)
    .bind(&msg.content)
    .bind(&msg.reasoning)
    .bind(tool_activities_json)
    .bind(tool_calls_json)
    .bind(&msg.tool_call_id)
    .bind(&msg.name)
    .bind(status_str)
    .bind(timestamp_val)
    .bind(estimated_tokens)
    .bind(work_timeline_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Update only the large tool/timeline columns so mid-turn tool completions
/// do not rewrite content/reasoning on every activity.
pub async fn save_message_tool_fields(pool: &SqlitePool, msg: &ChatMessage) -> Result<(), String> {
    let tool_activities_json = serialize_tool_activities(msg.tool_activities.as_ref());
    let work_timeline_json = serialize_work_timeline(msg.work_timeline.as_ref());
    let estimated_tokens = msg.estimated_tokens.map(|tokens| tokens as i64);

    sqlx::query(
        "UPDATE chat_messages
         SET tool_activities = ?, work_timeline = ?, estimated_tokens = ?
         WHERE id = ?",
    )
    .bind(tool_activities_json)
    .bind(work_timeline_json)
    .bind(estimated_tokens)
    .bind(&msg.id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn serialize_tool_activities(activities: Option<&Vec<ToolActivity>>) -> Option<String> {
    let activities = activities?;
    let capped: Vec<ToolActivity> = activities
        .iter()
        .map(|activity| {
            let mut activity = activity.clone();
            if let Some(result) = activity.result.as_mut() {
                *result = crate::core::chat::limits::truncate_tool_output(
                    result,
                    crate::core::chat::limits::STORED_TOOL_RESULT_MAX_CHARS,
                );
            }
            if let Some(preview) = activity.preview.as_mut() {
                if let Some(old) = preview.old_text.as_mut() {
                    *old = crate::core::chat::limits::truncate_chars(
                        old,
                        crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                    );
                }
                if let Some(new) = preview.new_text.as_mut() {
                    *new = crate::core::chat::limits::truncate_chars(
                        new,
                        crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                    );
                }
                preview.unified_diff = crate::core::chat::limits::truncate_chars(
                    &preview.unified_diff,
                    crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                );
            }
            activity
        })
        .collect();
    serde_json::to_string(&capped).ok()
}

fn serialize_work_timeline(
    timeline: Option<&Vec<crate::core::runtime::WorkTimelineItem>>,
) -> Option<String> {
    use crate::core::runtime::WorkTimelineItem;
    let timeline = timeline?;
    let capped: Vec<WorkTimelineItem> = timeline
        .iter()
        .map(|item| match item {
            WorkTimelineItem::Reasoning { id, content } => WorkTimelineItem::Reasoning {
                id: id.clone(),
                content: crate::core::chat::limits::truncate_chars(
                    content,
                    crate::core::chat::limits::STORED_TIMELINE_ITEM_MAX_CHARS,
                ),
            },
            WorkTimelineItem::Content { id, content } => WorkTimelineItem::Content {
                id: id.clone(),
                content: crate::core::chat::limits::truncate_chars(
                    content,
                    crate::core::chat::limits::STORED_TIMELINE_ITEM_MAX_CHARS,
                ),
            },
            other => other.clone(),
        })
        .collect();
    serde_json::to_string(&capped).ok()
}

fn serialize_tool_calls(calls: Option<&Vec<ToolCallPayload>>) -> Option<String> {
    let calls = calls?;
    let capped: Vec<ToolCallPayload> = calls
        .iter()
        .map(|call| {
            let mut call = call.clone();
            call.arguments = crate::core::chat::limits::truncate_chars(
                &call.arguments,
                crate::core::chat::limits::STORED_TOOL_CALL_ARGS_MAX_CHARS,
            );
            call
        })
        .collect();
    serde_json::to_string(&capped).ok()
}

fn parse_message_row(row: &sqlx::sqlite::SqliteRow) -> ChatMessage {
    let id: String = row.get("id");
    let session_id: String = row.get("session_id");

    let role_str: String = row.get("role");
    let role = match role_str.as_str() {
        "system" => Role::System,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    };

    let content: String = row.get("content");
    let reasoning: Option<String> = row.get("reasoning");
    let tool_activities_str: Option<String> = row.get("tool_activities");
    let tool_activities: Option<Vec<ToolActivity>> =
        tool_activities_str.and_then(|value| serde_json::from_str(&value).ok());

    let tool_calls_str: Option<String> = row.get("tool_calls");
    let tool_calls: Option<Vec<ToolCallPayload>> =
        tool_calls_str.and_then(|s| serde_json::from_str(&s).ok());

    let work_timeline_str: Option<String> = row.get("work_timeline");
    let work_timeline: Option<Vec<crate::core::runtime::WorkTimelineItem>> =
        work_timeline_str.and_then(|value| serde_json::from_str(&value).ok());

    let tool_call_id: Option<String> = row.get("tool_call_id");
    let name: Option<String> = row.get("name");

    let status_str: String = row.get("status");
    let status = match status_str.as_str() {
        "pending" => MessageStatus::Pending,
        "streaming" => MessageStatus::Streaming,
        "done" => MessageStatus::Done,
        "error" => MessageStatus::Error,
        "cancelled" => MessageStatus::Cancelled,
        _ => MessageStatus::Done,
    };

    let timestamp_val: i64 = row.get("timestamp");
    let timestamp = timestamp_val as u64;
    let cached_tokens = row.get::<Option<i64>, _>("estimated_tokens");

    ChatMessage {
        id,
        session_id,
        role,
        content,
        reasoning,
        tool_activities,
        tool_calls,
        tool_call_id,
        name,
        status,
        timestamp,
        estimated_tokens: cached_tokens.map(|tokens| tokens.max(0) as usize),
        work_timeline,
    }
}

async fn finalize_loaded_messages(
    pool: &SqlitePool,
    mut messages: Vec<ChatMessage>,
) -> Result<Vec<ChatMessage>, String> {
    let mut token_backfill = Vec::new();
    for message in &mut messages {
        if message.estimated_tokens.is_none()
            && !matches!(
                message.status,
                MessageStatus::Pending | MessageStatus::Streaming
            )
        {
            let estimated_tokens = crate::core::chat::limits::estimate_message_tokens(message);
            message.estimated_tokens = Some(estimated_tokens);
            token_backfill.push((message.id.clone(), estimated_tokens));
        }
    }

    if !token_backfill.is_empty() {
        let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
        for (id, estimated_tokens) in token_backfill {
            sqlx::query("UPDATE chat_messages SET estimated_tokens = ? WHERE id = ?")
                .bind(estimated_tokens as i64)
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(messages)
}

#[cfg(test)]
pub async fn load_all_messages(pool: &SqlitePool) -> Result<Vec<ChatMessage>, String> {
    let query = format!(
        "SELECT {MESSAGE_SELECT_COLUMNS}
         FROM chat_messages
         ORDER BY timestamp ASC;"
    );
    let rows = sqlx::query(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let messages = rows.iter().map(parse_message_row).collect();
    finalize_loaded_messages(pool, messages).await
}

pub async fn load_messages_for_session(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<ChatMessage>, String> {
    let query = format!(
        "SELECT {MESSAGE_SELECT_COLUMNS}
         FROM chat_messages
         WHERE session_id = ?
         ORDER BY timestamp ASC;"
    );
    let rows = sqlx::query(&query)
        .bind(session_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let messages = rows.iter().map(parse_message_row).collect();
    finalize_loaded_messages(pool, messages).await
}

/// Messages left mid-turn after a crash — loaded at startup for hydrate/settle
/// without pulling every session into memory.
pub async fn load_orphaned_messages(pool: &SqlitePool) -> Result<Vec<ChatMessage>, String> {
    let query = format!(
        "SELECT {MESSAGE_SELECT_COLUMNS}
         FROM chat_messages
         WHERE status IN ('pending', 'streaming')
         ORDER BY timestamp ASC;"
    );
    let rows = sqlx::query(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let messages = rows.iter().map(parse_message_row).collect();
    finalize_loaded_messages(pool, messages).await
}

pub async fn load_session_summaries(
    pool: &SqlitePool,
) -> Result<Vec<crate::models::chat::ChatSessionSummary>, String> {
    let rows = sqlx::query(
        "SELECT
            m.session_id AS session_id,
            s.workspace_id AS workspace_id,
            s.title AS title,
            COUNT(*) AS message_count,
            SUM(CASE WHEN m.role = 'user' THEN 1 ELSE 0 END) AS turn_count,
            SUM(COALESCE(m.estimated_tokens, 0)) AS estimated_tokens,
            MAX(m.timestamp) AS updated_at,
            (
                SELECT u.content
                FROM chat_messages u
                WHERE u.session_id = m.session_id AND u.role = 'user'
                ORDER BY u.timestamp ASC
                LIMIT 1
            ) AS preview_content,
            (
                SELECT a.content
                FROM chat_messages a
                WHERE a.session_id = m.session_id AND a.role = 'assistant'
                ORDER BY a.timestamp ASC
                LIMIT 1
            ) AS assistant_preview
         FROM chat_messages m
         LEFT JOIN chat_sessions s ON s.session_id = m.session_id
         GROUP BY m.session_id
         ORDER BY updated_at DESC;",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let session_id: String = row.get("session_id");
            let title: Option<String> = row.get("title");
            let preview_content: Option<String> = row.get("preview_content");
            let assistant_preview: Option<String> = row.get("assistant_preview");
            let preview = title
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
                .or_else(|| {
                    preview_content
                        .as_deref()
                        .map(super::selection::visible_user_text)
                        .filter(|value| !value.is_empty())
                        .map(|value| truncate_session_preview(&value))
                })
                .or_else(|| {
                    assistant_preview
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(truncate_session_preview)
                })
                .unwrap_or_else(|| "（空会话）".into());

            crate::models::chat::ChatSessionSummary {
                session_id,
                workspace_id: row.get("workspace_id"),
                preview,
                message_count: row.get::<i64, _>("message_count").max(0) as usize,
                turn_count: row.get::<i64, _>("turn_count").max(0) as usize,
                estimated_tokens: row.get::<i64, _>("estimated_tokens").max(0) as usize,
                updated_at: row.get::<i64, _>("updated_at").max(0) as u64,
            }
        })
        .collect())
}

fn truncate_session_preview(value: &str) -> String {
    const MAX: usize = 72;
    let normalized = value.replace('\n', " ").trim().to_string();
    if normalized.chars().count() <= MAX {
        return normalized;
    }
    let truncated: String = normalized.chars().take(MAX).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod message_persistence_tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn round_trips_tool_activities() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
                content TEXT NOT NULL, reasoning TEXT, tool_activities TEXT,
                tool_calls TEXT, tool_call_id TEXT, name TEXT,
                status TEXT NOT NULL, timestamp INTEGER NOT NULL,
                estimated_tokens INTEGER, work_timeline TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut message = ChatMessage {
            id: "assistant-1".into(),
            session_id: "session-1".into(),
            role: Role::Assistant,
            content: "done".into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: Some(vec![ToolActivity {
                id: "activity-1".into(),
                subagent_id: Some("child-1".into()),
                parent_activity_id: Some("parent-activity".into()),
                tool_name: "replace_in_file".into(),
                title: "Modify src/main.ts".into(),
                kind: "edit".into(),
                detail: None,
                arguments: Some(json!({ "path": "src/main.ts" })),
                result: Some("replaced".into()),
                preview: Some(crate::core::tools::preview::ToolPreview {
                    path: "src/main.ts".into(),
                    affected_paths: vec!["src/main.ts".into()],
                    kind: crate::core::tools::preview::ChangeKind::Modify,
                    old_text: Some("old".into()),
                    new_text: Some("new".into()),
                    unified_diff: "--- a/src/main.ts\n+++ b/src/main.ts\n-old\n+new\n".into(),
                }),
                success: true,
                status: "done".into(),
            }]),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        };
        message.estimated_tokens =
            Some(crate::core::chat::limits::estimate_message_tokens(&message));

        save_message(&pool, &message).await.unwrap();
        let loaded = load_all_messages(&pool).await.unwrap();

        assert_eq!(loaded, vec![message]);
    }

    #[tokio::test]
    async fn backfills_missing_token_estimate() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
                content TEXT NOT NULL, reasoning TEXT, tool_activities TEXT,
                tool_calls TEXT, tool_call_id TEXT, name TEXT,
                status TEXT NOT NULL, timestamp INTEGER NOT NULL,
                estimated_tokens INTEGER, work_timeline TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO chat_messages
             (id, session_id, role, content, status, timestamp, estimated_tokens)
             VALUES ('legacy-1', 'session-1', 'user', '12345678', 'done', 1, NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let loaded = load_all_messages(&pool).await.unwrap();
        assert_eq!(loaded[0].estimated_tokens, Some(6));

        let cached: Option<i64> =
            sqlx::query_scalar("SELECT estimated_tokens FROM chat_messages WHERE id = 'legacy-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cached, Some(6));
    }

    #[tokio::test]
    async fn truncates_oversized_tool_results_on_save() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
                content TEXT NOT NULL, reasoning TEXT, tool_activities TEXT,
                tool_calls TEXT, tool_call_id TEXT, name TEXT,
                status TEXT NOT NULL, timestamp INTEGER NOT NULL,
                estimated_tokens INTEGER, work_timeline TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let huge = "x".repeat(crate::core::chat::limits::STORED_TOOL_RESULT_MAX_CHARS + 500);
        let message = ChatMessage {
            id: "assistant-1".into(),
            session_id: "session-1".into(),
            role: Role::Assistant,
            content: "done".into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: Some(vec![ToolActivity {
                id: "activity-1".into(),
                subagent_id: None,
                parent_activity_id: None,
                tool_name: "read_file".into(),
                title: "Read".into(),
                kind: "read".into(),
                detail: None,
                arguments: None,
                result: Some(huge),
                preview: None,
                success: true,
                status: "done".into(),
            }]),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: Some(1),
        };

        save_message(&pool, &message).await.unwrap();
        let loaded = load_messages_for_session(&pool, "session-1").await.unwrap();
        let result = loaded[0].tool_activities.as_ref().unwrap()[0]
            .result
            .as_ref()
            .unwrap();
        assert!(
            result.chars().count() <= crate::core::chat::limits::STORED_TOOL_RESULT_MAX_CHARS + 40
        );
        assert!(result.contains("truncated"));
    }

    #[tokio::test]
    async fn loads_messages_for_one_session_only() {
        let path =
            std::env::temp_dir().join(format!("anya-session-load-{}.db", uuid::Uuid::new_v4()));
        let pool = init_db(&path).await.unwrap();
        for (id, session) in [("a1", "s1"), ("a2", "s2")] {
            let message = ChatMessage {
                id: id.into(),
                session_id: session.into(),
                role: Role::User,
                content: format!("hello {session}"),
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 1,
                estimated_tokens: Some(2),
            };
            save_message(&pool, &message).await.unwrap();
        }

        let s1 = load_messages_for_session(&pool, "s1").await.unwrap();
        assert_eq!(s1.len(), 1);
        assert_eq!(s1[0].id, "a1");

        let summaries = load_session_summaries(&pool).await.unwrap();
        assert_eq!(summaries.len(), 2);

        pool.close().await;
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
    }
}

#[cfg(test)]
mod token_usage_persistence_tests {
    use super::*;
    use crate::core::token::TokenUsage;

    #[tokio::test]
    async fn stores_and_filters_usage_records() {
        let path = std::env::temp_dir().join(format!("anya-token-{}.db", uuid::Uuid::new_v4()));
        let pool = init_db(&path).await.unwrap();
        record_token_usage(
            &pool,
            "run-1",
            "session-1",
            "deepseek-v4-pro",
            Some("deepseek"),
            &TokenUsage::exact(120, 30, "deepseek-v4"),
            1_000,
        )
        .await
        .unwrap();

        assert_eq!(
            load_token_usage_records(&pool, 999, 1_001)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(load_token_usage_records(&pool, 1_001, 2_000)
            .await
            .unwrap()
            .is_empty());
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}

pub async fn record_token_usage(
    pool: &SqlitePool,
    run_id: &str,
    session_id: &str,
    model: &str,
    provider: Option<&str>,
    usage: &TokenUsage,
    recorded_at: i64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO token_usage_records (
            id, run_id, session_id, model, provider, input_tokens, output_tokens,
            system_tokens, context_tokens, tool_call_tokens, tool_result_tokens,
            memory_tokens, total_tokens, accuracy, source, recorded_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(session_id)
    .bind(model)
    .bind(provider)
    .bind(usage.input_tokens as i64)
    .bind(usage.output_tokens as i64)
    .bind(usage.system_tokens as i64)
    .bind(usage.context_tokens as i64)
    .bind(usage.tool_call_tokens as i64)
    .bind(usage.tool_result_tokens as i64)
    .bind(usage.memory_tokens as i64)
    .bind(usage.total_tokens as i64)
    .bind(match usage.accuracy {
        crate::core::token::TokenAccuracy::Exact => "exact",
        crate::core::token::TokenAccuracy::Mixed => "mixed",
        crate::core::token::TokenAccuracy::Estimated => "estimated",
    })
    .bind(&usage.source)
    .bind(recorded_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn load_token_usage_records(
    pool: &SqlitePool,
    from: i64,
    to: i64,
) -> Result<Vec<sqlx::sqlite::SqliteRow>, String> {
    sqlx::query(
        "SELECT model, provider, input_tokens, output_tokens, system_tokens,
                context_tokens, tool_call_tokens, tool_result_tokens, memory_tokens,
                total_tokens, accuracy, recorded_at
         FROM token_usage_records WHERE recorded_at >= ? AND recorded_at < ?
         ORDER BY recorded_at ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}
