mod messages;
pub(crate) mod schema;
mod sessions;
mod tool_activity;

pub use messages::*;
pub use sessions::*;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

/// Open or create the chat SQLite database and run schema migrations.
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

    if is_new_database {
        sqlx::query("PRAGMA auto_vacuum = INCREMENTAL")
            .execute(&pool)
            .await
            .map_err(|error| error.to_string())?;
    }

    schema::init_all(&pool).await?;

    Ok(pool)
}
