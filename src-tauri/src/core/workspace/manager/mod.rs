//! Workspace registry CRUD: create, switch, and last-used timestamps.

use chrono::Utc;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::db::{init_schema, load_state, map_workspace_write_error, save_current_id};
use super::helpers::{normalize_ide_source, validate_root, workspace_name};
use super::types::{Workspace, WorkspaceId, WorkspaceManager};

mod organize;

impl WorkspaceManager {
    /// Opens or creates the workspace database at the given path.
    pub fn new(db_path: PathBuf) -> Self {
        let db_pool = tauri::async_runtime::block_on(async {
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create workspace database directory");
            }
            let options = SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("Failed to connect to workspace database");
            init_schema(&pool)
                .await
                .expect("Failed to initialize workspace database");
            pool
        });

        let (workspaces, current) = tauri::async_runtime::block_on(load_state(&db_pool))
            .expect("Failed to load workspaces");

        Self {
            current: Arc::new(RwLock::new(current)),
            workspaces: Arc::new(RwLock::new(workspaces)),
            db_pool,
        }
    }

    /// Returns the currently active workspace, if any.
    pub fn current(&self) -> Option<Workspace> {
        self.current.read().ok().and_then(|value| value.clone())
    }

    /// Lists all non-archived workspaces in display order.
    pub fn list(&self) -> Vec<Workspace> {
        self.sorted_workspaces()
            .into_iter()
            .filter(|workspace| !workspace.archived)
            .collect()
    }

    /// Lists all archived workspaces in display order.
    pub fn list_archived(&self) -> Vec<Workspace> {
        self.sorted_workspaces()
            .into_iter()
            .filter(|workspace| workspace.archived)
            .collect()
    }

    pub(super) fn sorted_workspaces(&self) -> Vec<Workspace> {
        let mut workspaces = self
            .workspaces
            .read()
            .map(|items| items.clone())
            .unwrap_or_default();
        workspaces.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| left.sort_order.cmp(&right.sort_order))
                .then_with(|| right.created_at.cmp(&left.created_at))
        });
        workspaces
    }

    /// Registers a new workspace for the given root directory.
    pub async fn create(&self, root: PathBuf) -> Result<Workspace, String> {
        validate_root(&root)?;

        let id = root.to_string_lossy().to_string();
        if let Some(existing) = self
            .sorted_workspaces()
            .into_iter()
            .find(|workspace| workspace.id == id)
        {
            if existing.archived {
                self.set_archived(&existing.id, false).await?;
                return self
                    .sorted_workspaces()
                    .into_iter()
                    .find(|workspace| workspace.id == id)
                    .ok_or_else(|| "Workspace not found".to_string());
            }
            return Ok(existing);
        }

        let now = Utc::now();
        let sort_order = self
            .list()
            .iter()
            .filter(|workspace| !workspace.pinned)
            .map(|workspace| workspace.sort_order)
            .min()
            .unwrap_or(1)
            - 1;
        let workspace = Workspace {
            id,
            name: workspace_name(&root),
            root,
            description: None,
            source: None,
            created_at: now,
            last_used_at: now,
            pinned: false,
            archived: false,
            sort_order,
        };
        let should_select = self.current().is_none();
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;

        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, created_at, last_used_at, pinned, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(workspace.root.to_string_lossy().to_string())
        .bind(&workspace.description)
        .bind(workspace.created_at.to_rfc3339())
        .bind(workspace.last_used_at.to_rfc3339())
        .bind(workspace.pinned)
        .bind(workspace.sort_order)
        .execute(&mut *transaction)
        .await
        .map_err(map_workspace_write_error)?;

        if should_select {
            save_current_id(&mut transaction, Some(workspace.id.clone())).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .push(workspace.clone());
        if should_select {
            *self
                .current
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())? = Some(workspace.clone());
        }

        Ok(workspace)
    }

    /// Creates or updates a workspace discovered from an external IDE.
    pub async fn remember_from_ide(
        &self,
        root: PathBuf,
        ide: &str,
    ) -> Result<(Workspace, bool), String> {
        validate_root(&root)?;
        let source = normalize_ide_source(ide)
            .ok_or_else(|| "IDE workspace source must not be empty".to_string())?;
        let id = root.to_string_lossy().to_string();

        if let Some(mut existing) = self.list().into_iter().find(|workspace| workspace.id == id) {
            if existing.source.as_deref() == Some(source.as_str()) {
                return Ok((existing, false));
            }

            sqlx::query("UPDATE workspace SET source = ? WHERE id = ?")
                .bind(&source)
                .bind(&id)
                .execute(&self.db_pool)
                .await
                .map_err(|error| error.to_string())?;

            existing.source = Some(source);
            let mut workspaces = self
                .workspaces
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())?;
            if let Some(workspace) = workspaces.iter_mut().find(|workspace| workspace.id == id) {
                *workspace = existing.clone();
            }
            drop(workspaces);
            if self.current().is_some_and(|workspace| workspace.id == id) {
                *self
                    .current
                    .write()
                    .map_err(|_| "Workspace lock is poisoned".to_string())? =
                    Some(existing.clone());
            }
            return Ok((existing, true));
        }

        let now = Utc::now();
        let sort_order = self
            .list()
            .iter()
            .filter(|workspace| !workspace.pinned)
            .map(|workspace| workspace.sort_order)
            .min()
            .unwrap_or(1)
            - 1;
        let workspace = Workspace {
            id,
            name: workspace_name(&root),
            root,
            description: None,
            source: Some(source),
            created_at: now,
            last_used_at: now,
            pinned: false,
            archived: false,
            sort_order,
        };
        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, source, created_at, last_used_at, pinned, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(workspace.root.to_string_lossy().to_string())
        .bind(&workspace.description)
        .bind(&workspace.source)
        .bind(workspace.created_at.to_rfc3339())
        .bind(workspace.last_used_at.to_rfc3339())
        .bind(workspace.pinned)
        .bind(workspace.sort_order)
        .execute(&self.db_pool)
        .await
        .map_err(map_workspace_write_error)?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .push(workspace.clone());
        Ok((workspace, true))
    }

    /// Makes the given workspace the active one and updates last-used time.
    pub async fn switch(&self, id: WorkspaceId) -> Result<Workspace, String> {
        let mut workspace = self
            .list()
            .into_iter()
            .find(|workspace| workspace.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;
        let now = Utc::now();

        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE workspace SET last_used_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(&id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        save_current_id(&mut transaction, Some(id)).await?;
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        workspace.last_used_at = now;
        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?;
        if let Some(stored) = workspaces.iter_mut().find(|item| item.id == workspace.id) {
            *stored = workspace.clone();
        }
        drop(workspaces);
        *self
            .current
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())? = Some(workspace.clone());

        Ok(workspace)
    }

    /// Updates the last-used timestamp for a workspace.
    pub async fn touch(&self, id: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE workspace SET last_used_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|error| error.to_string())?;

        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?;
        if let Some(workspace) = workspaces.iter_mut().find(|workspace| workspace.id == id) {
            workspace.last_used_at = now;
        }
        drop(workspaces);
        if let Ok(mut current) = self.current.write() {
            if let Some(workspace) = current.as_mut().filter(|workspace| workspace.id == id) {
                workspace.last_used_at = now;
            }
        }
        Ok(())
    }
}
