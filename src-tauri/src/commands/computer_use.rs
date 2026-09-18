//! Computer Use HUD + learned playbook IPC.

use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::core::plugins::{
    delete_computer_playbook, get_computer_playbook, list_computer_playbooks, ComputerPlaybook,
};

/// Hide the Computer Use glow/banner and cancel any in-flight chat streams.
#[tauri::command]
pub fn dismiss_computer_use_hud(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    crate::services::computer_use_hud::force_dismiss();
    state.core.chat().cancel_all_active();
    let _ = app;
    Ok(())
}

/// List learned Computer Use playbooks for the plugin settings page.
#[tauri::command]
pub fn list_computer_use_playbooks() -> Result<Vec<ComputerPlaybook>, String> {
    list_computer_playbooks().map_err(|e| e.to_string())
}

/// Load one playbook by id.
#[tauri::command]
pub fn get_computer_use_playbook(id: String) -> Result<ComputerPlaybook, String> {
    get_computer_playbook(&id).map_err(|e| e.to_string())
}

/// Delete a learned playbook.
#[tauri::command]
pub fn delete_computer_use_playbook(id: String) -> Result<(), String> {
    delete_computer_playbook(&id).map_err(|e| e.to_string())
}
