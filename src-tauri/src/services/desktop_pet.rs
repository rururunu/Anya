//! 桌面宠物（Desktop Pet）窗口服务
//!
//! 管理桌面宠物窗口的显示、隐藏、置顶与状态切换，并记住用户上次
//! 的开启状态，以便下次启动时自动恢复。

use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub const DESKTOP_PET_LABEL: &str = "desktop-pet";
const DESKTOP_PET_STATE_FILE: &str = "desktop-pet.json";

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(DESKTOP_PET_STATE_FILE))
}

/// 查询桌面宠物当前是否可见。
pub fn is_desktop_pet_visible(app: &AppHandle) -> bool {
    app.get_webview_window(DESKTOP_PET_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(false))
}

/// 读取上次记录的宠物开启偏好。
fn pet_was_enabled(app: &AppHandle) -> bool {
    let Some(path) = state_path(app) else {
        return false;
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| value.get("desktopPetEnabled").and_then(|v| v.as_bool()))
        .unwrap_or(false)
}

/// 应用启动时按上次记录的开关恢复宠物显示。
pub fn restore_desktop_pet_on_startup(app: &AppHandle) {
    if pet_was_enabled(app) {
        show_desktop_pet(app);
    }
}

/// 持久化宠物开启偏好，供下次启动恢复。
fn remember_pet_enabled(app: &AppHandle, enabled: bool) {
    let Some(path) = state_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let raw = serde_json::to_string(&serde_json::json!({ "desktopPetEnabled": enabled }))
        .unwrap_or_default();
    let _ = fs::write(path, raw);
}

/// 确保桌面宠物窗口属性处于正确的置顶和无边框状态。
fn configure_desktop_pet_window(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    crate::services::overlay_native::reapply_toolwindow_style(window);
    let _ = window.set_shadow(false);
}

/// 显示桌面宠物窗口。
pub fn show_desktop_pet(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(DESKTOP_PET_LABEL) else {
        tracing::warn!("desktop pet window is missing from application config");
        return false;
    };

    configure_desktop_pet_window(&window);

    if let Ok(settings) = crate::services::settings_store::get_settings(app) {
        crate::services::webview_theme::apply_webview_theme(app, &settings);
    }

    let _ = window.show();
    let _ = app.emit("desktop-pet-visibility-changed", true);
    true
}

/// 隐藏桌面宠物窗口。
pub fn hide_desktop_pet(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(DESKTOP_PET_LABEL) else {
        return false;
    };

    let _ = window.hide();
    let _ = app.emit("desktop-pet-visibility-changed", false);
    false
}

/// 切换桌面宠物可见状态。如果指定了 `visible` 则以其为准，否则取反，
/// 并记住结果供下次启动恢复。
pub fn toggle_desktop_pet(app: &AppHandle, visible: Option<bool>) -> bool {
    let currently_visible = is_desktop_pet_visible(app);
    let target = visible.unwrap_or(!currently_visible);

    remember_pet_enabled(app, target);

    if target {
        show_desktop_pet(app);
    } else {
        hide_desktop_pet(app);
    }
    target
}
