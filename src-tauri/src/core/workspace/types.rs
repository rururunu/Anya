use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub type WorkspaceId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub root: PathBuf,
    pub description: Option<String>,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    pub sort_order: i64,
}

pub struct WorkspaceManager {
    pub(super) current: Arc<RwLock<Option<Workspace>>>,
    pub(super) workspaces: Arc<RwLock<Vec<Workspace>>>,
    pub(super) db_pool: SqlitePool,
}
