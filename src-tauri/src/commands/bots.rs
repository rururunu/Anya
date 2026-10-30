//! IPC for creating bots and reading the tasks that belong to them.

use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::core::bots::{self, Bot, ScheduledTask};

#[tauri::command]
pub fn list_bots() -> Result<Vec<Bot>, String> {
    bots::list_bots()
}

#[tauri::command]
pub fn create_bot(name: String, avatar: String, session_id: String) -> Result<Bot, String> {
    bots::create_bot(name, avatar, session_id)
}

#[tauri::command]
pub fn update_bot(id: String, name: String, avatar: String) -> Result<Bot, String> {
    bots::update_bot(&id, name, avatar)
}

#[tauri::command]
pub fn delete_bot(id: String) -> Result<(), String> {
    bots::delete_bot(&id)
}

#[tauri::command]
pub fn list_bot_schedules() -> Result<Vec<ScheduledTask>, String> {
    bots::list_tasks()
}

#[tauri::command]
pub fn set_bot_schedule_enabled(id: String, enabled: bool) -> Result<ScheduledTask, String> {
    bots::set_task_enabled(&id, enabled)
}

#[tauri::command]
pub fn delete_bot_schedule(id: String) -> Result<(), String> {
    bots::delete_task(&id)
}

#[tauri::command]
pub async fn run_bot_schedule_now(
    app: AppHandle,
    _state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    bots::run_now(&app, &id).await
}
