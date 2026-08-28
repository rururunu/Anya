use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app_state::AppState;
use crate::core::workspace::Workspace;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use walkdir::{DirEntry, WalkDir};

fn resolve_existing_path(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".to_string());
    }
    let raw = PathBuf::from(trimmed);
    if raw.exists() {
        return Ok(raw);
    }
    if let Some(workspace) = state.core.workspaces().current() {
        let joined = Path::new(&workspace.root).join(&raw);
        if joined.exists() {
            return Ok(joined);
        }
    }
    Err("Path no longer exists".to_string())
}

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

fn find_workspace(state: &State<'_, AppState>, id: &str) -> Result<Workspace, String> {
    state
        .core
        .workspaces()
        .list()
        .into_iter()
        .chain(state.core.workspaces().list_archived())
        .find(|workspace| workspace.id == id)
        .ok_or_else(|| "Workspace not found".to_string())
}

/// Explorer and terminals reject `\\?\` verbatim paths and often mis-parse
/// `/e,/root,` plus forward slashes (common for IDE-reported folders).
fn normalize_shell_path(path: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim_path(resolved)
}

fn strip_verbatim_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else if cfg!(windows) && text.contains('/') {
        PathBuf::from(text.replace('/', "\\"))
    } else {
        path
    }
}

fn require_workspace_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.is_dir() {
        return Err("Workspace folder no longer exists".to_string());
    }
    Ok(normalize_shell_path(path))
}

fn open_in_file_manager(app: &AppHandle, folder: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe")
            .raw_arg(format!(r#""{}""#, folder.display()))
            .spawn()
            .map_err(|error| format!("Failed to open in File Explorer: {error}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        app.opener()
            .open_path(folder.to_string_lossy(), None::<&str>)
            .map_err(|error| error.to_string())
    }
}

fn open_in_terminal(folder: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if Command::new("wt.exe").arg("-d").arg(folder).spawn().is_ok() {
            return Ok(());
        }
        let dir = folder.to_string_lossy().into_owned();
        Command::new("cmd.exe")
            .args(["/c", "start", "Anya", "/D", &dir, "cmd.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("Failed to open in terminal: {error}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal"])
            .arg(folder)
            .spawn()
            .map_err(|error| format!("Failed to open in terminal: {error}"))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let candidates = [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "xterm",
        ];
        for program in candidates {
            if Command::new(program).current_dir(folder).spawn().is_ok() {
                return Ok(());
            }
        }
        Err("Failed to open in terminal".to_string())
    }
}

#[tauri::command]
pub fn open_workspace_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let workspace = find_workspace(&state, &id)?;
    let folder = require_workspace_dir(&workspace.root)?;
    open_in_file_manager(&app, &folder)
}

#[tauri::command]
pub fn open_workspace_in_terminal(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let workspace = find_workspace(&state, &id)?;
    let folder = require_workspace_dir(&workspace.root)?;
    open_in_terminal(&folder)
}

#[tauri::command]
pub fn open_in_default_app(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let target = resolve_existing_path(&state, &path)?;
    app.opener()
        .open_path(target.to_string_lossy(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[cfg_attr(target_os = "windows", allow(unused_variables))]
pub fn reveal_in_explorer(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let target = resolve_existing_path(&state, &path)?;

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
pub async fn update_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let manager = state.core.workspaces();
    let workspace = manager.update(&id, name, description).await?;
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(workspace)
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
    state
        .core
        .chat()
        .set_sessions_archived_for_workspace(&id, archived);
    app.emit("workspaces-changed", manager.current())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::strip_verbatim_path;
    use std::path::PathBuf;

    #[test]
    fn strips_windows_verbatim_drive_prefix() {
        let path = strip_verbatim_path(PathBuf::from(r"\\?\C:\Users\foo\project"));
        assert_eq!(path, PathBuf::from(r"C:\Users\foo\project"));
    }

    #[test]
    fn strips_windows_verbatim_unc_prefix() {
        let path = strip_verbatim_path(PathBuf::from(r"\\?\UNC\server\share\repo"));
        assert_eq!(path, PathBuf::from(r"\\server\share\repo"));
    }

    #[test]
    fn converts_forward_slashes_on_windows() {
        let path = strip_verbatim_path(PathBuf::from("C:/Users/foo/project"));
        if cfg!(windows) {
            assert_eq!(path, PathBuf::from(r"C:\Users\foo\project"));
        } else {
            assert_eq!(path, PathBuf::from("C:/Users/foo/project"));
        }
    }
}
