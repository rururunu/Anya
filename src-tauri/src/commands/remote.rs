use tauri::AppHandle;
use tauri::Emitter;

use crate::core::remote::{
    broadcast_compose_event, clear_staged, create_pairing_session, gateway_status,
    get_session_compose, insert_staged, list_paired_devices, list_staged, pop_staged_front,
    push_staged, remove_staged, revoke_device, set_session_compose, start_gateway, stop_gateway,
    take_staged_at, GatewayStatus, PairedDevice, PairingSessionInfo, SessionCompose, TunnelPrefs,
};

#[tauri::command]
pub fn remote_gateway_status(app: AppHandle) -> GatewayStatus {
    gateway_status(&app)
}

#[tauri::command]
pub fn remote_gateway_start(app: AppHandle, port: Option<u16>) -> Result<GatewayStatus, String> {
    start_gateway(app.clone(), port)?;
    // Bind is async — brief wait so status reflects running when possible.
    std::thread::sleep(std::time::Duration::from_millis(120));
    Ok(gateway_status(&app))
}

#[tauri::command]
pub fn remote_gateway_stop(app: AppHandle) -> GatewayStatus {
    stop_gateway(&app);
    gateway_status(&app)
}

#[tauri::command]
pub fn remote_create_pairing(app: AppHandle) -> Result<PairingSessionInfo, String> {
    create_pairing_session(&app)
}

#[tauri::command]
pub fn remote_list_devices(app: AppHandle) -> Vec<PairedDevice> {
    list_paired_devices(&app)
}

#[tauri::command]
pub fn remote_revoke_device(app: AppHandle, device_id: String) -> Result<GatewayStatus, String> {
    if !revoke_device(&app, &device_id) {
        return Err("device not found".into());
    }
    Ok(gateway_status(&app))
}

#[tauri::command]
pub fn remote_get_tunnel_prefs(app: AppHandle) -> TunnelPrefs {
    crate::core::remote::remote_state(&app).tunnel_prefs()
}

#[tauri::command]
pub fn remote_set_tunnel_prefs(app: AppHandle, prefs: TunnelPrefs) -> Result<(), String> {
    crate::core::remote::remote_state(&app).set_tunnel_prefs(prefs)
}

/// Desktop → Companion: mirror Pinia session compose into the gateway store and fan out.
#[tauri::command]
pub fn remote_sync_session_compose(
    session_id: String,
    compose: SessionCompose,
) -> Result<SessionCompose, String> {
    if session_id.trim().is_empty() {
        return Err("sessionId required".into());
    }
    let stored = set_session_compose(&session_id, compose);
    broadcast_compose_event(&session_id, &stored);
    Ok(stored)
}

#[tauri::command]
pub fn remote_get_session_compose(session_id: String) -> SessionCompose {
    get_session_compose(&session_id)
}

fn emit_staged(app: &AppHandle, session_id: &str, messages: &[String]) {
    let _ = app.emit(
        "remote-staged-changed",
        serde_json::json!({ "sessionId": session_id, "messages": messages }),
    );
}

#[tauri::command]
pub fn remote_list_staged(session_id: String) -> Vec<String> {
    list_staged(&session_id)
}

#[tauri::command]
pub fn remote_push_staged(app: AppHandle, session_id: String, message: String) -> Vec<String> {
    let messages = push_staged(&session_id, &message);
    emit_staged(&app, &session_id, &messages);
    messages
}

#[tauri::command]
pub fn remote_remove_staged(app: AppHandle, session_id: String, index: usize) -> Vec<String> {
    let messages = remove_staged(&session_id, index);
    emit_staged(&app, &session_id, &messages);
    messages
}

#[tauri::command]
pub fn remote_clear_staged(app: AppHandle, session_id: String) {
    clear_staged(&session_id);
    emit_staged(&app, &session_id, &[]);
}

#[tauri::command]
pub fn remote_insert_staged(
    app: AppHandle,
    session_id: String,
    index: usize,
    message: String,
) -> Vec<String> {
    let messages = insert_staged(&session_id, index, &message);
    emit_staged(&app, &session_id, &messages);
    messages
}

/// Pop the front staged message for post-turn flush (atomic vs Companion).
#[tauri::command]
pub fn remote_pop_staged(app: AppHandle, session_id: String) -> Option<String> {
    let content = pop_staged_front(&session_id)?;
    emit_staged(&app, &session_id, &list_staged(&session_id));
    Some(content)
}

/// Take one staged message for soft-inject guide.
#[tauri::command]
pub fn remote_take_staged(app: AppHandle, session_id: String, index: usize) -> Option<String> {
    let content = take_staged_at(&session_id, index)?;
    emit_staged(&app, &session_id, &list_staged(&session_id));
    Some(content)
}
