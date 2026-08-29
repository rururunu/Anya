use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;

use super::helpers::display_name;
use super::types::{Workspace, WorkspaceId};

pub(super) async fn init_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root TEXT NOT NULL UNIQUE,
            description TEXT,
            source TEXT,
            created_at TEXT NOT NULL,
            last_used_at TEXT NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0,
            archived INTEGER NOT NULL DEFAULT 0,
            sort_order INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;

    let columns = sqlx::query("PRAGMA table_info(workspace)")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "source")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN source TEXT")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "pinned")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "sort_order")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "last_used_at")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN last_used_at TEXT")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE workspace SET last_used_at = created_at WHERE last_used_at IS NULL")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "archived")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN archived INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace_state (
            singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
            current_workspace_id TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;

    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE workspace_state
         SET current_workspace_id = (
             SELECT root FROM workspace WHERE id = workspace_state.current_workspace_id
         )
         WHERE current_workspace_id IS NOT NULL
           AND EXISTS (SELECT 1 FROM workspace WHERE id = workspace_state.current_workspace_id)",
    )
    .execute(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE workspace SET id = root WHERE id <> root")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) async fn load_state(pool: &SqlitePool) -> Result<(Vec<Workspace>, Option<Workspace>), String> {
    let rows = sqlx::query(
        "SELECT id, name, root, description, source, created_at, last_used_at, pinned, archived, sort_order
             FROM workspace
             ORDER BY pinned DESC, sort_order ASC, created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let workspaces = rows
        .into_iter()
        .map(workspace_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let current_id =
        sqlx::query("SELECT current_workspace_id FROM workspace_state WHERE singleton_id = 1")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?
            .and_then(|row| row.get::<Option<String>, _>("current_workspace_id"));
    let current = current_id.and_then(|id| {
        workspaces
            .iter()
            .find(|workspace| workspace.id == id && !workspace.archived)
            .cloned()
    });
    Ok((workspaces, current))
}

fn workspace_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Workspace, String> {
    let id = row.get::<String, _>("id");
    let root = PathBuf::from(row.get::<String, _>("root"));
    let stored_name = row.get::<String, _>("name");
    let created_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
        .map_err(|error| error.to_string())?
        .with_timezone(&Utc);
    let last_used_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("last_used_at"))
        .map_err(|error| error.to_string())?
        .with_timezone(&Utc);
    Ok(Workspace {
        id,
        name: display_name(&stored_name, &root),
        root,
        description: row
            .get::<Option<String>, _>("description")
            .and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
        source: row.get::<Option<String>, _>("source"),
        created_at,
        last_used_at,
        pinned: row.get::<bool, _>("pinned"),
        archived: row.get::<bool, _>("archived"),
        sort_order: row.get::<i64, _>("sort_order"),
    })
}

pub(super) async fn save_current_id(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: Option<WorkspaceId>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO workspace_state (singleton_id, current_workspace_id) VALUES (1, ?)
         ON CONFLICT(singleton_id) DO UPDATE SET current_workspace_id = excluded.current_workspace_id",
    )
    .bind(id)
    .execute(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn map_workspace_write_error(error: sqlx::Error) -> String {
    let message = error.to_string();
    if message.contains("UNIQUE constraint failed: workspace.root") {
        "A workspace with this root already exists".to_string()
    } else {
        message
    }
}
