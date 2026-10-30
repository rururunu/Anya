//! Fires a bot's due tasks by sending a turn into that bot's conversation.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::core::chat::session_origin::RequestOrigin;
use crate::core::chat::SendPreferences;
use crate::models::chat::ChatSendOverrides;
use crate::models::settings::ChatMode;
use crate::services::settings_store::{apply_chat_request_settings, get_settings};

use super::model::{now_ms, Bot, ScheduledTask};
use super::store;

struct Inflight {
    started_ms: i64,
}

fn inflight() -> &'static Mutex<HashMap<String, Inflight>> {
    static MAP: OnceLock<Mutex<HashMap<String, Inflight>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pending() -> &'static Mutex<HashMap<String, String>> {
    static MAP: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn start_scheduler(app: AppHandle) {
    super::bind_app(app.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            if let Err(error) = tick(&app).await {
                tracing::warn!(%error, "bot schedule tick failed");
            }
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    });
}

/// Attach the assistant reply to the task that started this turn, if any.
pub fn note_run_finished(session_id: &str, content: &str, ok: bool) {
    let task_id = pending()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(session_id);
    let Some(task_id) = task_id else {
        return;
    };
    inflight()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&task_id);
    let _ = store::complete_task(&task_id, content, ok);
}

pub async fn run_now(app: &AppHandle, id: &str) -> Result<(), String> {
    let (task, bot) = store::task_with_bot(id)?;
    if !mark_running(&task.id) {
        return Err("this task is already running".into());
    }
    if let Err(error) = dispatch(app, &task, &bot).await {
        clear_running(&task.id);
        return Err(error);
    }
    Ok(())
}

async fn tick(app: &AppHandle) -> Result<(), String> {
    for (task, bot) in store::due_tasks(now_ms())? {
        if !mark_running(&task.id) {
            continue;
        }
        if let Err(error) = dispatch(app, &task, &bot).await {
            clear_running(&task.id);
            let _ = store::postpone_task(&task.id, now_ms().saturating_add(120_000), &error);
        }
    }
    Ok(())
}

async fn dispatch(app: &AppHandle, task: &ScheduledTask, bot: &Bot) -> Result<(), String> {
    let settings = get_settings(app)?;
    let preferences = SendPreferences::from(&settings);
    let settings_for_cfg = settings.clone();
    tauri::async_runtime::spawn_blocking(move || apply_chat_request_settings(&settings_for_cfg))
        .await
        .map_err(|error| format!("configure runtimes failed: {error}"))?;
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "app state unavailable".to_string())?;
    let message = format!(
        "定时任务到点了：「{}」\n请完成下面的任务，并把结果告诉我。\n\n{}",
        task.title, task.instruction
    );
    let mut overrides = ChatSendOverrides::default();
    overrides.chat_mode = Some(ChatMode::Agent);
    state
        .core
        .chat()
        .send(
            Some(bot.session_id.clone()),
            message,
            preferences,
            None,
            false,
            overrides,
            RequestOrigin::Desktop,
        )
        .await
        .map_err(|error| error.to_string())?;
    pending()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(bot.session_id.clone(), task.id.clone());
    Ok(())
}

fn mark_running(id: &str) -> bool {
    let now = now_ms();
    let mut guard = inflight().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(run) = guard.get(id) {
        if now.saturating_sub(run.started_ms) < 30 * 60 * 1000 {
            return false;
        }
    }
    guard.insert(id.to_string(), Inflight { started_ms: now });
    true
}

fn clear_running(id: &str) {
    inflight()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(id);
}
