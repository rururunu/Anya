use crate::core::chat::session_title::SessionTitleSource;
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Persisted session title row.
pub struct SessionTitleRecord {
    pub title: String,
    pub source: Option<SessionTitleSource>,
}

/// Load session IDs marked as archived.
pub async fn load_session_archived(pool: &SqlitePool) -> Result<HashSet<String>, String> {
    let rows = sqlx::query("SELECT session_id FROM chat_sessions WHERE archived = 1")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(rows.into_iter().map(|row| row.get("session_id")).collect())
}

/// Set archived flag for a single session.
pub async fn set_session_archived(
    pool: &SqlitePool,
    session_id: &str,
    archived: bool,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT INTO chat_sessions (session_id, archived, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
             archived = excluded.archived,
             updated_at = excluded.updated_at",
    )
    .bind(session_id)
    .bind(archived)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Set archived flag for multiple sessions in one transaction.
pub async fn set_sessions_archived_batch(
    pool: &SqlitePool,
    session_ids: &[String],
    archived: bool,
) -> Result<(), String> {
    if session_ids.is_empty() {
        return Ok(());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    for session_id in session_ids {
        sqlx::query(
            "INSERT INTO chat_sessions (session_id, archived, created_at, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(session_id) DO UPDATE SET
                 archived = excluded.archived,
                 updated_at = excluded.updated_at",
        )
        .bind(session_id)
        .bind(archived)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Load session-to-workspace bindings for all sessions with a workspace.
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

/// Bind a workspace to a session without overwriting an existing binding.
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
        "INSERT INTO chat_sessions
         (session_id, workspace_id, created_at, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
             workspace_id = COALESCE(chat_sessions.workspace_id, excluded.workspace_id),
             updated_at = excluded.updated_at",
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

/// Set or overwrite the workspace binding for a session.
pub async fn set_session_workspace(
    pool: &SqlitePool,
    session_id: &str,
    workspace_id: &str,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT INTO chat_sessions
         (session_id, workspace_id, created_at, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
             workspace_id = excluded.workspace_id,
             updated_at = excluded.updated_at",
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

/// Load non-empty session titles keyed by session ID.
pub async fn load_session_titles(
    pool: &SqlitePool,
) -> Result<HashMap<String, SessionTitleRecord>, String> {
    let rows = sqlx::query(
        "SELECT session_id, title, title_source FROM chat_sessions
         WHERE title IS NOT NULL AND trim(title) != ''",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let session_id: String = row.get("session_id");
            let title: String = row.get("title");
            let source_raw: Option<String> = row.get("title_source");
            let source = source_raw.as_deref().and_then(SessionTitleSource::parse);
            (session_id, SessionTitleRecord { title, source })
        })
        .collect())
}

/// Persist a session title and its source.
pub async fn save_session_title(
    pool: &SqlitePool,
    session_id: &str,
    title: &str,
    source: SessionTitleSource,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT INTO chat_sessions (session_id, title, title_source, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
             title = excluded.title,
             title_source = excluded.title_source,
             updated_at = excluded.updated_at",
    )
    .bind(session_id)
    .bind(title)
    .bind(source.as_str())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Load aggregated session summaries for the workbench session list.
pub async fn load_session_summaries(
    pool: &SqlitePool,
) -> Result<Vec<crate::models::chat::ChatSessionSummary>, String> {
    let rows = sqlx::query(
        "SELECT
            m.session_id AS session_id,
            s.workspace_id AS workspace_id,
            s.title AS title,
            COALESCE(s.archived, 0) AS archived,
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
                        .map(crate::core::chat::selection::visible_user_text)
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
                archived: row.get::<i64, _>("archived") != 0,
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
mod session_workspace_tests {
    use super::*;
    use crate::core::chat::db::schema::init_chat_session_schema;
    use crate::core::chat::session_title::SessionTitleSource;

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
    async fn set_session_workspace_overwrites_existing_binding() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_chat_session_schema(&pool).await.unwrap();
        bind_session_workspace(&pool, "s1", "ws-a").await.unwrap();
        set_session_workspace(&pool, "s1", "ws-b").await.unwrap();
        let workspaces = load_session_workspaces(&pool).await.unwrap();
        assert_eq!(workspaces["s1"], "ws-b");
    }

    #[tokio::test]
    async fn bind_fills_workspace_when_session_row_already_exists() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_chat_session_schema(&pool).await.unwrap();
        save_session_title(&pool, "s1", "草稿标题", SessionTitleSource::Fallback)
            .await
            .unwrap();
        bind_session_workspace(&pool, "s1", r"D:\Code\VueAdmin")
            .await
            .unwrap();
        let workspaces = load_session_workspaces(&pool).await.unwrap();
        assert_eq!(workspaces["s1"], r"D:\Code\VueAdmin");
    }

    #[tokio::test]
    async fn migrates_title_column_and_persists_titles() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
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
        save_session_title(&pool, "s1", "修复工具循环熔断", SessionTitleSource::Auto)
            .await
            .unwrap();
        save_session_title(&pool, "s1", "更新标题", SessionTitleSource::Auto)
            .await
            .unwrap();
        save_session_title(&pool, "s2", "  ", SessionTitleSource::Auto)
            .await
            .unwrap();

        let titles = load_session_titles(&pool).await.unwrap();
        assert_eq!(titles["s1"].title, "更新标题");
        assert_eq!(titles["s1"].source, Some(SessionTitleSource::Auto));
        assert!(!titles.contains_key("s2"));
    }
}
