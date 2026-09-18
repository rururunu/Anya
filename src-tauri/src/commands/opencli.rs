//! OpenCLI plugin setup IPC (status + one-click npm install).

use crate::core::plugins::opencli::{
    install_cli, probe_status, OpenCliInstallResult, OpenCliSetupStatus,
};

#[tauri::command]
pub fn get_opencli_setup_status(include_doctor: Option<bool>) -> OpenCliSetupStatus {
    probe_status(include_doctor.unwrap_or(false))
}

#[tauri::command]
pub async fn install_opencli_cli() -> Result<OpenCliInstallResult, String> {
    tauri::async_runtime::spawn_blocking(install_cli)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_opencli_doctor() -> OpenCliSetupStatus {
    tauri::async_runtime::spawn_blocking(|| probe_status(true))
        .await
        .unwrap_or_else(|_| probe_status(false))
}
