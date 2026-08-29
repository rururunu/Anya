use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

/// Run all chat database schema migrations and create indexes.
pub(crate) async fn init_all(pool: &SqlitePool) -> Result<(), String> {
    init_messages_schema(pool).await?;
    init_chat_session_schema(pool).await?;
    crate::core::chat::journal::init_journal_schema(pool).await?;
    crate::core::chat::journal::compact_recovery_journal(pool).await?;
    init_token_usage_schema(pool).await?;
    Ok(())
}

async fn init_messages_schema(pool: &SqlitePool) -> Result<(), String> {
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
            estimated_tokens INTEGER,
            completed_at INTEGER
        );",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let message_columns = sqlx::query("PRAGMA table_info(chat_messages)")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "tool_activities")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN tool_activities TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "estimated_tokens")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN estimated_tokens INTEGER")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "work_timeline")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN work_timeline TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "completed_at")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN completed_at INTEGER")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_ts
         ON chat_messages(session_id, timestamp);",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_status
         ON chat_messages(status);",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Migrate chat_sessions table and legacy workspace_path column.
pub(crate) async fn init_chat_session_schema(pool: &SqlitePool) -> Result<(), String> {
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
    if !column_names.iter().any(|name| name == "archived") {
        sqlx::query("ALTER TABLE chat_sessions ADD COLUMN archived INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !column_names.iter().any(|name| name == "title_source") {
        sqlx::query("ALTER TABLE chat_sessions ADD COLUMN title_source TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn init_token_usage_schema(pool: &SqlitePool) -> Result<(), String> {
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
            recorded_at INTEGER NOT NULL,
            cache_read_tokens INTEGER,
            message_id TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    ensure_token_usage_columns(pool).await?;
    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_token_usage_recorded_at ON token_usage_records(recorded_at)",
        "CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage_records(model)",
        "CREATE INDEX IF NOT EXISTS idx_token_usage_session ON token_usage_records(session_id, recorded_at)",
        "CREATE INDEX IF NOT EXISTS idx_token_usage_message ON token_usage_records(session_id, message_id)",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn ensure_token_usage_columns(pool: &SqlitePool) -> Result<(), String> {
    let columns = sqlx::query("PRAGMA table_info(token_usage_records)")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let names = columns
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<HashSet<_>>();
    if !names.contains("cache_read_tokens") {
        sqlx::query("ALTER TABLE token_usage_records ADD COLUMN cache_read_tokens INTEGER")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !names.contains("message_id") {
        sqlx::query("ALTER TABLE token_usage_records ADD COLUMN message_id TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
