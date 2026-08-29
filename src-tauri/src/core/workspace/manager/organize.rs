//! Pin, reorder, archive, update, and delete workspace registry entries.

use super::super::db::save_current_id;
use super::super::helpers::display_name;
use super::super::types::{Workspace, WorkspaceId, WorkspaceManager};

impl WorkspaceManager {
    /// Pins or unpins a workspace in the sidebar list.
    pub async fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), String> {
        let sort_order = self
            .list()
            .iter()
            .filter(|workspace| workspace.pinned == pinned && workspace.id != id)
            .map(|workspace| workspace.sort_order)
            .min()
            .unwrap_or(1)
            - 1;
        sqlx::query("UPDATE workspace SET pinned = ?, sort_order = ? WHERE id = ?")
            .bind(pinned)
            .bind(sort_order)
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|error| error.to_string())?;

        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?;
        let workspace = workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;
        workspace.pinned = pinned;
        workspace.sort_order = sort_order;
        let updated_workspace = workspace.clone();
        drop(workspaces);
        if let Ok(mut current) = self.current.write() {
            if current.as_ref().is_some_and(|workspace| workspace.id == id) {
                *current = Some(updated_workspace);
            }
        }
        Ok(())
    }

    /// Reorders workspaces according to the provided id sequence.
    pub async fn reorder(&self, ids: &[String]) -> Result<(), String> {
        let known_ids = self
            .list()
            .into_iter()
            .map(|workspace| workspace.id)
            .collect::<std::collections::HashSet<_>>();
        let requested_ids = ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        if ids.len() != known_ids.len() || requested_ids != known_ids {
            return Err("Workspace order must include every workspace exactly once".to_string());
        }

        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        for (sort_order, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE workspace SET sort_order = ? WHERE id = ?")
                .bind(sort_order as i64)
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;

        let order = ids
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index as i64))
            .collect::<std::collections::HashMap<_, _>>();
        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?;
        for workspace in workspaces.iter_mut() {
            workspace.sort_order = order[workspace.id.as_str()];
        }
        let updated_current = self.current().and_then(|current| {
            workspaces
                .iter()
                .find(|workspace| workspace.id == current.id)
                .cloned()
        });
        drop(workspaces);
        if let Some(updated_current) = updated_current {
            *self
                .current
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())? = Some(updated_current);
        }
        Ok(())
    }

    /// Archives or restores a workspace without deleting it.
    pub async fn set_archived(&self, id: &str, archived: bool) -> Result<(), String> {
        sqlx::query("UPDATE workspace SET archived = ?, pinned = CASE WHEN ? THEN 0 ELSE pinned END WHERE id = ?")
            .bind(archived)
            .bind(archived)
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|error| error.to_string())?;

        {
            let mut workspaces = self
                .workspaces
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())?;
            let workspace = workspaces
                .iter_mut()
                .find(|workspace| workspace.id == id)
                .ok_or_else(|| "Workspace not found".to_string())?;
            workspace.archived = archived;
            if archived {
                workspace.pinned = false;
            }
        }

        let should_clear_current =
            archived && self.current().is_some_and(|current| current.id == id);
        if should_clear_current {
            self.clear_current().await?;
        } else if let Ok(mut current) = self.current.write() {
            if current.as_ref().is_some_and(|workspace| workspace.id == id) {
                if let Some(updated) = self
                    .sorted_workspaces()
                    .into_iter()
                    .find(|workspace| workspace.id == id)
                {
                    *current = Some(updated);
                }
            }
        }
        Ok(())
    }

    /// Updates editable workspace metadata such as name and description.
    pub async fn update(
        &self,
        id: &str,
        name: String,
        description: Option<String>,
    ) -> Result<Workspace, String> {
        let root = self
            .sorted_workspaces()
            .into_iter()
            .find(|workspace| workspace.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?
            .root;
        let name = display_name(&name, &root);
        let description = description.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

        sqlx::query("UPDATE workspace SET name = ?, description = ? WHERE id = ?")
            .bind(&name)
            .bind(&description)
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|error| error.to_string())?;

        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?;
        let workspace = workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;
        workspace.name = name;
        workspace.description = description;
        let updated = workspace.clone();
        drop(workspaces);
        if let Ok(mut current) = self.current.write() {
            if current.as_ref().is_some_and(|workspace| workspace.id == id) {
                *current = Some(updated.clone());
            }
        }
        Ok(updated)
    }

    /// Clears the active workspace selection without deleting any entry.
    pub async fn clear_current(&self) -> Result<(), String> {
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        save_current_id(&mut transaction, None).await?;
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        *self
            .current
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())? = None;
        Ok(())
    }

    /// Permanently removes a workspace from the registry.
    pub async fn delete(&self, id: WorkspaceId) -> Result<(), String> {
        if !self
            .sorted_workspaces()
            .iter()
            .any(|workspace| workspace.id == id)
        {
            return Err("Workspace not found".to_string());
        }

        let deleting_current = self.current().is_some_and(|workspace| workspace.id == id);
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM workspace WHERE id = ?")
            .bind(&id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        if deleting_current {
            save_current_id(&mut transaction, None).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .retain(|workspace| workspace.id != id);
        if deleting_current {
            *self
                .current
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())? = None;
        }
        Ok(())
    }
}
