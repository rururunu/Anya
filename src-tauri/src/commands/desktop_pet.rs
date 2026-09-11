//! 桌面宠物 IPC 命令

use tauri::AppHandle;

/// 切换或设置桌面宠物的可见状态。
#[tauri::command]
pub fn toggle_desktop_pet(app: AppHandle, visible: Option<bool>) -> Result<bool, String> {
    Ok(crate::services::desktop_pet::toggle_desktop_pet(
        &app, visible,
    ))
}

/// 获取桌面宠物当前的可见状态。
#[tauri::command]
pub fn get_desktop_pet_visible(app: AppHandle) -> Result<bool, String> {
    Ok(crate::services::desktop_pet::is_desktop_pet_visible(&app))
}

/// 显示并聚焦 Anya 主工作台。
#[tauri::command]
pub fn show_workbench(app: AppHandle) -> Result<(), String> {
    crate::services::window::show_workbench_window(&app);
    Ok(())
}

/// 触发呼出 Overlay 快捷提问栏。
#[tauri::command]
pub fn toggle_overlay_from_pet(app: AppHandle) -> Result<(), String> {
    crate::services::window::toggle_overlay(&app, None);
    Ok(())
}
