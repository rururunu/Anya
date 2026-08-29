//! Workspace registry backed by SQLite.

mod db;
mod helpers;
mod manager;
mod types;

pub use types::{Workspace, WorkspaceManager};

#[cfg(test)]
mod tests {
    use super::helpers::workspace_name;
    use super::db::{init_schema, load_state};
    use super::types::WorkspaceManager;
    use sqlx::{Row, SqlitePool};
    use std::sync::{Arc, RwLock};

    async fn manager() -> WorkspaceManager {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_schema(&pool).await.unwrap();
        WorkspaceManager {
            current: Arc::new(RwLock::new(None)),
            workspaces: Arc::new(RwLock::new(Vec::new())),
            db_pool: pool,
        }
    }

    #[tokio::test]
    async fn create_switch_and_delete_keep_current_state_consistent() {
        let manager = manager().await;
        let first = manager.create(std::env::temp_dir()).await.unwrap();
        assert_eq!(manager.current().unwrap().id, first.id);

        let second = manager
            .create(std::env::current_dir().unwrap())
            .await
            .unwrap();
        assert_eq!(manager.list().len(), 2);
        assert_eq!(manager.current().unwrap().id, first.id);

        assert_eq!(second.id, second.root.display().to_string());
        assert_eq!(
            second.name,
            second.root.file_name().unwrap().to_string_lossy()
        );

        manager.switch(second.id.clone()).await.unwrap();
        assert_eq!(manager.current().unwrap().id, second.id);
        let (_, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(persisted_current.unwrap().id, second.id);

        manager.clear_current().await.unwrap();
        assert!(manager.current().is_none());
        let (_, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert!(persisted_current.is_none());

        manager.switch(second.id.clone()).await.unwrap();

        manager.delete(second.id).await.unwrap();
        assert!(manager.current().is_none());
        assert_eq!(manager.list(), vec![first]);
    }

    #[tokio::test]
    async fn migrates_legacy_uuid_ids_to_root_paths() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_schema(&pool).await.unwrap();
        let root = std::env::temp_dir().join("legacy-project");
        let root_string = root.display().to_string();
        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, created_at, last_used_at)
             VALUES ('legacy-uuid', 'Custom Name', ?, 'Old note', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .bind(&root_string)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workspace_state (singleton_id, current_workspace_id)
             VALUES (1, 'legacy-uuid')",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_schema(&pool).await.unwrap();
        let (workspaces, current) = load_state(&pool).await.unwrap();

        assert_eq!(workspaces[0].id, root_string);
        assert_eq!(workspaces[0].name, "Custom Name");
        assert_eq!(workspaces[0].description.as_deref(), Some("Old note"));
        assert!(workspaces[0].source.is_none());
        assert_eq!(current.unwrap().id, workspaces[0].id);
    }

    #[tokio::test]
    async fn adds_source_column_to_existing_workspace_database() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE workspace (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_schema(&pool).await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(workspace)")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "source"));
        assert!(columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "last_used_at"));
    }

    #[tokio::test]
    async fn remembers_ide_workspace_source_without_switching_current_workspace() {
        let manager = manager().await;
        let current = manager.create(std::env::temp_dir()).await.unwrap();
        let ide_root = std::env::current_dir().unwrap();

        let (remembered, changed) = manager
            .remember_from_ide(ide_root.clone(), "visual studio code")
            .await
            .unwrap();

        assert!(changed);
        assert_eq!(remembered.root, ide_root);
        assert_eq!(remembered.source.as_deref(), Some("vscode"));
        assert_eq!(manager.current().unwrap().id, current.id);

        let (persisted, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(
            persisted
                .iter()
                .find(|workspace| workspace.id == remembered.id)
                .and_then(|workspace| workspace.source.as_deref()),
            Some("vscode")
        );
        assert_eq!(persisted_current.unwrap().id, current.id);

        let (_, changed_again) = manager.remember_from_ide(ide_root, "vscode").await.unwrap();
        assert!(!changed_again);
    }

    #[tokio::test]
    async fn workspace_order_changes_only_through_manual_controls() {
        let manager = manager().await;
        let first = manager.create(std::env::temp_dir()).await.unwrap();
        let second = manager
            .create(std::env::current_dir().unwrap())
            .await
            .unwrap();

        manager
            .reorder(&[first.id.clone(), second.id.clone()])
            .await
            .unwrap();
        let manual_order = manager
            .list()
            .into_iter()
            .map(|workspace| workspace.id)
            .collect::<Vec<_>>();
        manager.switch(second.id.clone()).await.unwrap();
        manager.switch(first.id.clone()).await.unwrap();
        assert_eq!(
            manager
                .list()
                .into_iter()
                .map(|workspace| workspace.id)
                .collect::<Vec<_>>(),
            manual_order
        );

        manager.switch(second.id.clone()).await.unwrap();
        manager.set_pinned(&second.id, true).await.unwrap();
        assert_eq!(manager.list()[0].id, second.id);
        assert!(manager.current().unwrap().pinned);
        let (persisted, _) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(persisted[0].id, second.id);
        assert!(persisted[0].pinned);
    }

    #[tokio::test]
    async fn update_persists_custom_name_and_description() {
        let manager = manager().await;
        let workspace = manager.create(std::env::temp_dir()).await.unwrap();

        let updated = manager
            .update(
                &workspace.id,
                "  My Project  ".into(),
                Some("  Notes for this repo  ".into()),
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "My Project");
        assert_eq!(updated.description.as_deref(), Some("Notes for this repo"));
        assert_eq!(manager.current().unwrap().name, "My Project");

        let (persisted, current) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(persisted[0].name, "My Project");
        assert_eq!(
            persisted[0].description.as_deref(),
            Some("Notes for this repo")
        );
        assert_eq!(current.unwrap().name, "My Project");

        let cleared = manager
            .update(&workspace.id, "   ".into(), Some("   ".into()))
            .await
            .unwrap();
        assert_eq!(cleared.name, workspace_name(&workspace.root));
        assert!(cleared.description.is_none());
    }
}
