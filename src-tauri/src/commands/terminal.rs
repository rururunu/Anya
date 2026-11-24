//! Workbench IPC for the local interactive terminal.
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::plugins::PtyHub;

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    let events = app.clone();
    app.manage(PtyHub::new(std::sync::Arc::new(move |event| {
        if let Some(window) = events.get_webview_window("workbench") {
            let _ = window.emit("terminal-output", event);
        }
    })));
}

pub fn shutdown(app: &AppHandle) {
    if let Some(hub) = app.try_state::<PtyHub>() {
        hub.kill_all();
    }
}

#[tauri::command]
pub async fn workbench_terminal(
    hub: State<'_, PtyHub>,
    operation: String,
    session: Option<String>,
    data: Option<String>,
    cwd: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<Value, String> {
    let size = (cols.unwrap_or(80), rows.unwrap_or(24));
    let id = session.as_deref().unwrap_or_default();
    let result = match operation.as_str() {
        "open" => hub
            .open_native(size.0, size.1, cwd.as_deref())
            .map(|id| json!(id)),
        "write" => hub
            .write(id, data.as_deref().unwrap_or_default())
            .map(|_| Value::Null),
        "resize" => hub.resize(id, size.0, size.1).map(|_| Value::Null),
        "close" => hub.kill(id).map(|_| Value::Null),
        _ => return Err("Unknown terminal operation".into()),
    };
    result.map_err(|err| err.to_string())
}
