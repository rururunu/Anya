use std::path::PathBuf;
use std::process::Command;

use crate::app_state::AppState;
use crate::core::workspace::Workspace;
use tauri::{AppHandle, Emitter, State};
#[cfg(not(target_os = "windows"))]
use tauri_plugin_opener::OpenerExt;
use walkdir::{DirEntry, WalkDir};

const MAX_WORKSPACE_FILES: usize = 5_000;

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git" | ".svn" | ".hg" | "node_modules" | ".next" | ".nuxt"
    )
}

#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> Vec<Workspace> {
    state.core.workspaces().list()
}

#[tauri::command]
pub fn get_current_workspace(state: State<'_, AppState>) -> Option<Workspace> {
    state.core.workspaces().current()
}

#[tauri::command]
pub async fn list_workspace_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let workspace = state
        .core
        .workspaces()
        .current()
        .ok_or_else(|| "No active workspace".to_string())?;
    let root = workspace.root;

    tauri::async_runtime::spawn_blocking(move || {
        let mut files = WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(should_descend)
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&root)
                    .ok()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
            })
            .take(MAX_WORKSPACE_FILES)
            .collect::<Vec<_>>();
        files.sort_unstable_by_key(|path| path.to_lowercase());
        files
    })
    .await
    .map_err(|error| format!("workspace file scan failed: {error}"))
}

#[tauri::command]
pub async fn create_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
) -> Result<Workspace, String> {
    let manager = state.core.workspaces();
    let workspace = manager.create(PathBuf::from(root)).await?;
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(workspace)
}

#[tauri::command]
pub async fn switch_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Workspace, String> {
    let workspace = state.core.workspaces().switch(id).await?;
    app.emit("workspaces-changed", Some(workspace.clone()))
        .map_err(|error| error.to_string())?;
    Ok(workspace)
}

#[tauri::command]
pub async fn clear_current_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.core.workspaces().clear_current().await?;
    app.emit("workspaces-changed", Option::<Workspace>::None)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let manager = state.core.workspaces();
    manager.delete(id).await?;
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_workspace_folder(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let workspace = state
        .core
        .workspaces()
        .list()
        .into_iter()
        .chain(state.core.workspaces().list_archived())
        .find(|workspace| workspace.id == id)
        .ok_or_else(|| "Workspace not found".to_string())?;
    if !workspace.root.is_dir() {
        return Err("Workspace folder no longer exists".to_string());
    }

    Command::new("explorer.exe")
        .arg(format!("/e,/root,{}", workspace.root.display()))
        .spawn()
        .map_err(|error| format!("Failed to open workspace folder: {error}"))?;
    Ok(())
}

#[tauri::command]
#[cfg_attr(target_os = "windows", allow(unused_variables))]
pub fn reveal_in_explorer(app: AppHandle, path: String) -> Result<(), String> {
    let target = PathBuf::from(path.trim());
    if !target.exists() {
        return Err("Path no longer exists".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe")
            .raw_arg(format!(r#"/select,"{}""#, target.display()))
            .spawn()
            .map_err(|error| format!("Failed to reveal in Explorer: {error}"))?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let folder = if target.is_file() {
            target.parent().unwrap_or(target.as_path()).to_path_buf()
        } else {
            target
        };
        app.opener()
            .open_path(folder.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
pub async fn set_workspace_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    let manager = state.core.workspaces();
    manager.set_pinned(&id, pinned).await?;
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_workspaces(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let manager = state.core.workspaces();
    manager.reorder(&ids).await?;
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_archived_workspaces(state: State<'_, AppState>) -> Vec<Workspace> {
    state.core.workspaces().list_archived()
}

#[tauri::command]
pub async fn set_workspace_archived(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<(), String> {
    let manager = state.core.workspaces();
    manager.set_archived(&id, archived).await?;
    let sessions = if archived {
        state.core.chat().list_sessions()
    } else {
        state.core.chat().list_archived_sessions()
    };
    for session in sessions {
        if session.workspace_id.as_deref() == Some(id.as_str()) {
            state
                .core
                .chat()
                .set_session_archived(&session.session_id, archived);
        }
    }
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(())
}
