use super::tool_activity::{serialize_tool_activities, serialize_tool_calls, serialize_work_timeline};
use crate::core::runtime::stream::ToolCallPayload;
use crate::core::runtime::{ChatMessage, MessageStatus, Role, ToolActivity};
use crate::core::token::TokenUsage;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const MESSAGE_SELECT_COLUMNS: &str = "id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp, estimated_tokens, work_timeline";

/// Insert or replace a chat message row.
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
    let completed_at = if matches!(
        msg.status,
        MessageStatus::Done | MessageStatus::Error | MessageStatus::Cancelled
    ) {
        let existing = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT completed_at FROM chat_messages WHERE id = ?",
        )
        .bind(&msg.id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .flatten();
        Some(existing.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_millis() as i64)
                .unwrap_or(timestamp_val)
        }))
    } else {
        None
    };

    sqlx::query(
        "INSERT OR REPLACE INTO chat_messages (
            id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp, estimated_tokens, work_timeline, completed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"
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
    .bind(completed_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Update only tool-activity and timeline columns for an in-progress message.
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

/// Load all messages for a session ordered by timestamp.
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

/// Load pending/streaming messages left mid-turn after a crash.
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

/// Load assistant message completion timestamps keyed by message ID.
pub async fn load_message_completed_at(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<HashMap<String, u64>, String> {
    let rows = sqlx::query(
        "SELECT id, completed_at FROM chat_messages
         WHERE session_id = ? AND completed_at IS NOT NULL",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id = row.try_get::<String, _>("id").ok()?;
            let completed_at = row.try_get::<i64, _>("completed_at").ok()?;
            if completed_at <= 0 {
                return None;
            }
            Some((id, completed_at as u64))
        })
        .collect())
}

/// Record a token usage snapshot for a model run.
pub async fn record_token_usage(
    pool: &SqlitePool,
    run_id: &str,
    session_id: &str,
    message_id: Option<&str>,
    model: &str,
    provider: Option<&str>,
    usage: &TokenUsage,
    recorded_at: i64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO token_usage_records (
            id, run_id, session_id, message_id, model, provider, input_tokens, output_tokens,
            system_tokens, context_tokens, tool_call_tokens, tool_result_tokens,
            memory_tokens, total_tokens, accuracy, source, recorded_at, cache_read_tokens
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(session_id)
    .bind(message_id)
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
    .bind(usage.cache_read_tokens.map(|value| value as i64))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load token usage records within a time range.
pub async fn load_token_usage_records(
    pool: &SqlitePool,
    from: i64,
    to: i64,
) -> Result<Vec<sqlx::sqlite::SqliteRow>, String> {
    sqlx::query(
        "SELECT model, provider, input_tokens, output_tokens, system_tokens,
                context_tokens, tool_call_tokens, tool_result_tokens, memory_tokens,
                total_tokens, accuracy, recorded_at, cache_read_tokens
         FROM token_usage_records WHERE recorded_at >= ? AND recorded_at < ?
         ORDER BY recorded_at ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Load aggregated cache-read token usage for a session.
pub async fn load_session_cache_usage(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Option<crate::models::chat::SessionCacheUsage>, String> {
    let row = sqlx::query(
        "SELECT
            COALESCE(SUM(input_tokens), 0) AS input_tokens,
            COALESCE(SUM(cache_read_tokens), 0) AS cache_read_tokens,
            (
                SELECT model FROM token_usage_records
                WHERE session_id = ? AND cache_read_tokens IS NOT NULL
                ORDER BY recorded_at DESC LIMIT 1
            ) AS model
         FROM token_usage_records
         WHERE session_id = ? AND cache_read_tokens IS NOT NULL",
    )
    .bind(session_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.and_then(|row| {
        let input = row.get::<i64, _>("input_tokens").max(0) as usize;
        let cache_read = row.get::<i64, _>("cache_read_tokens").max(0) as usize;
        if input == 0 && cache_read == 0 {
            return None;
        }
        Some(crate::models::chat::SessionCacheUsage {
            input_tokens: input,
            cache_read_tokens: cache_read,
            model: row.try_get::<String, _>("model").ok(),
        })
    }))
}

/// Load per-message cache-read token usage for a session.
pub async fn load_message_cache_usages(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<crate::models::chat::MessageCacheUsage>, String> {
    let rows = sqlx::query(
        "SELECT message_id,
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(cache_read_tokens), 0) AS cache_read_tokens
         FROM token_usage_records
         WHERE session_id = ? AND message_id IS NOT NULL AND cache_read_tokens IS NOT NULL
         GROUP BY message_id",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let message_id = row.try_get::<String, _>("message_id").ok()?;
            if message_id.is_empty() {
                return None;
            }
            Some(crate::models::chat::MessageCacheUsage {
                message_id,
                input_tokens: row.get::<i64, _>("input_tokens").max(0) as usize,
                cache_read_tokens: row.get::<i64, _>("cache_read_tokens").max(0) as usize,
            })
        })
        .collect())
}

#[cfg(test)]
mod message_persistence_tests {
    use super::*;
    use crate::core::chat::db::init_db;
    use crate::core::chat::db::sessions::load_session_summaries;
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
                estimated_tokens INTEGER, work_timeline TEXT, completed_at INTEGER
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
                estimated_tokens INTEGER, work_timeline TEXT, completed_at INTEGER
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
                estimated_tokens INTEGER, work_timeline TEXT, completed_at INTEGER
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
    use crate::core::chat::db::init_db;
    use crate::core::token::TokenUsage;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn stores_and_filters_usage_records() {
        let path = std::env::temp_dir().join(format!("anya-token-{}.db", uuid::Uuid::new_v4()));
        let pool = init_db(&path).await.unwrap();
        record_token_usage(
            &pool,
            "run-1",
            "session-1",
            None,
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

    #[tokio::test]
    async fn loads_session_cache_usage_totals() {
        let path =
            std::env::temp_dir().join(format!("anya-token-cache-{}.db", uuid::Uuid::new_v4()));
        let pool = init_db(&path).await.unwrap();
        record_token_usage(
            &pool,
            "run-1",
            "session-1",
            Some("msg-1"),
            "deepseek-v4-pro",
            Some("deepseek"),
            &TokenUsage::exact_with_breakdown(20, 10, "deepseek-v4", Some(80), None),
            1_000,
        )
        .await
        .unwrap();
        record_token_usage(
            &pool,
            "run-2",
            "session-1",
            Some("msg-2"),
            "deepseek-v4-pro",
            Some("deepseek"),
            &TokenUsage::exact_with_breakdown(10, 8, "deepseek-v4", Some(90), None),
            2_000,
        )
        .await
        .unwrap();
        record_token_usage(
            &pool,
            "run-2b",
            "session-1",
            Some("msg-2"),
            "deepseek-v4-pro",
            Some("deepseek"),
            &TokenUsage::exact_with_breakdown(5, 4, "deepseek-v4", Some(10), None),
            2_500,
        )
        .await
        .unwrap();

        let session = load_session_cache_usage(&pool, "session-1")
            .await
            .unwrap()
            .expect("cache usage");
        assert_eq!(session.input_tokens, 35);
        assert_eq!(session.cache_read_tokens, 180);
        assert_eq!(session.model.as_deref(), Some("deepseek-v4-pro"));
        assert!(load_session_cache_usage(&pool, "missing")
            .await
            .unwrap()
            .is_none());

        let mut messages = load_message_cache_usages(&pool, "session-1").await.unwrap();
        messages.sort_by(|left, right| left.message_id.cmp(&right.message_id));
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id, "msg-1");
        assert_eq!(messages[0].input_tokens, 20);
        assert_eq!(messages[0].cache_read_tokens, 80);
        assert_eq!(messages[1].message_id, "msg-2");
        assert_eq!(messages[1].input_tokens, 15);
        assert_eq!(messages[1].cache_read_tokens, 100);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn migrates_legacy_token_usage_table_without_message_id() {
        let path =
            std::env::temp_dir().join(format!("anya-token-legacy-{}.db", uuid::Uuid::new_v4()));
        {
            let options = SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .unwrap();
            sqlx::query(
                "CREATE TABLE token_usage_records (
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
                    recorded_at INTEGER NOT NULL,
                    cache_read_tokens INTEGER
                )",
            )
            .execute(&pool)
            .await
            .unwrap();
            pool.close().await;
        }

        let pool = init_db(&path)
            .await
            .expect("legacy token_usage_records should gain message_id before indexes");
        let columns = sqlx::query("PRAGMA table_info(token_usage_records)")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "message_id"));
        pool.close().await;
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
    }
}
