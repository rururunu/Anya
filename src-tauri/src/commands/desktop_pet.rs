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

/// 设置桌面宠物尺寸（small|medium|large）。
#[tauri::command]
pub fn set_desktop_pet_size(app: AppHandle, size: String) -> Result<String, String> {
    let parsed = crate::services::desktop_pet::PetSize::parse(&size)
        .ok_or_else(|| format!("invalid pet size: {size}"))?;
    crate::services::desktop_pet::set_pet_size(&app, parsed);
    Ok(parsed.as_str().to_string())
}

/// 获取桌面宠物尺寸。
#[tauri::command]
pub fn get_desktop_pet_size(app: AppHandle) -> Result<String, String> {
    Ok(crate::services::desktop_pet::get_pet_size(&app)
        .as_str()
        .to_string())
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
