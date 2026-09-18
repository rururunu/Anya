//! 桌面宠物（Desktop Pet）窗口服务
//!
//! 管理桌面宠物窗口的显示、隐藏、置顶与状态切换，并记住用户上次
//! 的开启状态，以便下次启动时自动恢复。插件可通过 `pet.*` host RPC
//! 改尺寸、皮肤与表情。

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub const DESKTOP_PET_LABEL: &str = "desktop-pet";
const DESKTOP_PET_STATE_FILE: &str = "desktop-pet.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PetSize {
    Small,
    Medium,
    Large,
}

impl PetSize {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            "large" => Some(Self::Large),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetAppearance {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetPersistedState {
    #[serde(default)]
    desktop_pet_enabled: bool,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    appearance: Option<PetAppearance>,
}

#[derive(Debug, Default)]
struct PetRuntimeState {
    size: PetSize,
    appearance: Option<PetAppearance>,
    expression: Option<String>,
}

impl Default for PetSize {
    fn default() -> Self {
        Self::Medium
    }
}

fn runtime_state() -> &'static Mutex<PetRuntimeState> {
    static STATE: OnceLock<Mutex<PetRuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(PetRuntimeState::default()))
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(DESKTOP_PET_STATE_FILE))
}

fn load_persisted(app: &AppHandle) -> PetPersistedState {
    let Some(path) = state_path(app) else {
        return PetPersistedState::default();
    };
    let Some(raw) = fs::read_to_string(path).ok() else {
        return PetPersistedState::default();
    };
    let mut state = serde_json::from_str::<PetPersistedState>(&raw).unwrap_or_default();
    if state.appearance.is_none() {
        state.appearance = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|value| value.get("skin").and_then(parse_appearance_value));
    }
    state
}

fn parse_appearance_value(value: &serde_json::Value) -> Option<PetAppearance> {
    if let Some(mode) = value.get("mode").and_then(|item| item.as_str()) {
        return match mode {
            "mascot" => Some(PetAppearance {
                mode: "mascot".to_string(),
                kind: None,
                source: None,
                config: None,
            }),
            "companion" => Some(PetAppearance {
                mode: "companion".to_string(),
                kind: None,
                source: None,
                config: value.get("config").cloned(),
            }),
            "media" => {
                let kind = value.get("kind").and_then(|item| item.as_str())?;
                let source = value.get("source").and_then(|item| item.as_str())?;
                Some(PetAppearance {
                    mode: "media".to_string(),
                    kind: Some(kind.to_string()),
                    source: Some(source.to_string()),
                    config: None,
                })
            }
            _ => None,
        };
    }
    let kind = value.get("kind").and_then(|item| item.as_str())?;
    let source = value.get("source").and_then(|item| item.as_str())?;
    Some(PetAppearance {
        mode: "media".to_string(),
        kind: Some(kind.to_string()),
        source: Some(source.to_string()),
        config: None,
    })
}

fn save_persisted(app: &AppHandle, state: &PetPersistedState) {
    let Some(path) = state_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(state) {
        let _ = fs::write(path, raw);
    }
}

/// 查询桌面宠物当前是否可见。
pub fn is_desktop_pet_visible(app: &AppHandle) -> bool {
    app.get_webview_window(DESKTOP_PET_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(false))
}

/// 应用启动时按上次记录的开关恢复宠物显示。
pub fn restore_desktop_pet_on_startup(app: &AppHandle) {
    let persisted = load_persisted(app);
    if let Some(size) = persisted
        .size
        .as_deref()
        .and_then(PetSize::parse)
    {
        if let Ok(mut guard) = runtime_state().lock() {
            guard.size = size;
            guard.appearance = persisted.appearance.clone();
        }
    } else if let Ok(mut guard) = runtime_state().lock() {
        guard.appearance = persisted.appearance.clone();
    }
    if persisted.desktop_pet_enabled {
        show_desktop_pet(app);
    }
}

/// 持久化宠物开启偏好，供下次启动恢复。
fn remember_pet_enabled(app: &AppHandle, enabled: bool) {
    let mut state = load_persisted(app);
    state.desktop_pet_enabled = enabled;
    if let Ok(guard) = runtime_state().lock() {
        state.size = Some(guard.size.as_str().to_string());
        state.appearance = guard.appearance.clone();
    }
    save_persisted(app, &state);
}

fn persist_runtime(app: &AppHandle) {
    let mut state = load_persisted(app);
    if let Ok(guard) = runtime_state().lock() {
        state.size = Some(guard.size.as_str().to_string());
        state.appearance = guard.appearance.clone();
    }
    save_persisted(app, &state);
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
    emit_pet_state(app);
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

pub fn get_pet_size(app: &AppHandle) -> PetSize {
    if let Ok(guard) = runtime_state().lock() {
        return guard.size;
    }
    load_persisted(app)
        .size
        .as_deref()
        .and_then(PetSize::parse)
        .unwrap_or_default()
}

pub fn set_pet_size(app: &AppHandle, size: PetSize) {
    if let Ok(mut guard) = runtime_state().lock() {
        guard.size = size;
    }
    persist_runtime(app);
    let _ = app.emit("desktop-pet-size-changed", size.as_str());
}

pub fn get_pet_appearance(app: &AppHandle) -> Option<PetAppearance> {
    if let Ok(guard) = runtime_state().lock() {
        return guard.appearance.clone();
    }
    load_persisted(app).appearance
}

pub fn set_pet_appearance(app: &AppHandle, appearance: Option<PetAppearance>) {
    if let Ok(mut guard) = runtime_state().lock() {
        guard.appearance = appearance.clone();
    }
    persist_runtime(app);
    let _ = app.emit("desktop-pet-appearance-changed", appearance.clone());
    let _ = app.emit("desktop-pet-skin-changed", appearance);
}

pub fn clear_pet_appearance(app: &AppHandle) {
    set_pet_appearance(app, None);
}

pub fn set_pet_expression(app: &AppHandle, expression: Option<String>) {
    if let Ok(mut guard) = runtime_state().lock() {
        guard.expression = expression.clone();
    }
    let _ = app.emit("desktop-pet-expression-changed", expression);
}

fn emit_pet_state(app: &AppHandle) {
    let (size, appearance, expression) = if let Ok(guard) = runtime_state().lock() {
        (
            guard.size,
            guard.appearance.clone(),
            guard.expression.clone(),
        )
    } else {
        (PetSize::default(), None, None)
    };
    let _ = app.emit("desktop-pet-size-changed", size.as_str());
    let _ = app.emit("desktop-pet-appearance-changed", appearance.clone());
    let _ = app.emit("desktop-pet-skin-changed", appearance);
    let _ = app.emit("desktop-pet-expression-changed", expression);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pet_sizes() {
        assert_eq!(PetSize::parse("small"), Some(PetSize::Small));
        assert_eq!(PetSize::parse("MEDIUM"), Some(PetSize::Medium));
        assert_eq!(PetSize::parse("huge"), None);
    }
}
