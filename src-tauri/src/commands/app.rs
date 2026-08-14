use tauri::AppHandle;

use crate::models::app_info::AppInfo;

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: app.package_info().name.clone(),
        version: app.package_info().version.to_string(),
        identifier: app.config().identifier.clone(),
    })
}

#[tauri::command]
pub fn webview_gpu_disabled() -> bool {
    crate::services::settings_store::webview_gpu_disabled()
}

#[tauri::command]
pub fn relaunch_app(app: AppHandle) {
    crate::services::app_lifecycle::mark_allow_exit();
    app.restart();
}
