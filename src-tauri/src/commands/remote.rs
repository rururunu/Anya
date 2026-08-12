use tauri::AppHandle;

use crate::core::remote::{
    broadcast_compose_event, create_pairing_session, gateway_status, get_session_compose,
    list_paired_devices, revoke_device, set_session_compose, start_gateway, stop_gateway,
    GatewayStatus, PairedDevice, PairingSessionInfo, SessionCompose, TunnelPrefs,
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
