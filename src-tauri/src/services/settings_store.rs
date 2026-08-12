use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::models::settings::AppSettings;

const SETTINGS_FILE: &str = "settings.json";
const RELEASE_APP_IDENTIFIER: &str = "ai.anya.desktop";
const DEBUG_APP_IDENTIFIER: &str = "ai.anya.desktop.debug";

fn app_identifier() -> &'static str {
    if cfg!(debug_assertions) {
        DEBUG_APP_IDENTIFIER
    } else {
        RELEASE_APP_IDENTIFIER
    }
}

pub struct SettingsState {
    pub settings: Mutex<AppSettings>,
}

/// Read the one setting needed before Tauri creates the WebView2 environment.
/// Hardware acceleration defaults off; only the enabled path skips browser args.
pub fn configure_prestart_webview() {
    let Some(app_data) = std::env::var_os("APPDATA") else {
        // No settings file yet — apply the default (GPU off).
        apply_disable_gpu_args();
        return;
    };
    let app_data = PathBuf::from(app_data);
    let path = app_data.join(app_identifier()).join(SETTINGS_FILE);
    let legacy_path = app_data
        .join(if cfg!(debug_assertions) {
            "ai.aaai.desktop.debug"
        } else {
            "ai.aaai.desktop"
        })
        .join(SETTINGS_FILE);
    let settings_file = if path.is_file() {
        path
    } else {
        legacy_path
    };
    let hardware_acceleration_enabled = fs::read_to_string(&settings_file)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|settings| {
            settings
                .get("hardwareAccelerationEnabled")
                .and_then(serde_json::Value::as_bool)
        })
        .unwrap_or(false);
    if hardware_acceleration_enabled {
        return;
    }
    apply_disable_gpu_args();
}

fn apply_disable_gpu_args() {
    const DISABLE_GPU_ARGS: &str = "--disable-gpu --disable-gpu-compositing";
    let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    if existing.contains("--disable-gpu") {
        return;
    }
    let arguments = if existing.trim().is_empty() {
        DISABLE_GPU_ARGS.to_string()
    } else {
        format!("{} {}", existing.trim(), DISABLE_GPU_ARGS)
    };
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", arguments);
}

impl SettingsState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(SETTINGS_FILE))
        .map_err(|error| error.to_string())
}

/// One-time copy from the previous app id so existing installs keep settings after the rename.
fn migrate_legacy_settings_file(new_path: &PathBuf) {
    const LEGACY_IDENTIFIERS: &[&str] = &["ai.aaai.desktop", "ai.aaai.desktop.debug"];
    let Some(app_data) = std::env::var_os("APPDATA").map(PathBuf::from) else {
        return;
    };
    for legacy_id in LEGACY_IDENTIFIERS {
        let legacy = app_data.join(legacy_id).join(SETTINGS_FILE);
        if !legacy.is_file() {
            continue;
        }
        if let Some(parent) = new_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&legacy, new_path).is_ok() {
            tracing::info!(
                from = %legacy.display(),
                to = %new_path.display(),
                "migrated settings from legacy app identifier"
            );
            return;
        }
    }
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    if !path.exists() {
        migrate_legacy_settings_file(&path);
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return AppSettings::default(),
    };

    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let mut settings: AppSettings = serde_json::from_value(parsed.clone()).unwrap_or_default();
    let has_restricted_shell = parsed
        .as_object()
        .is_some_and(|obj| obj.contains_key("restrictedShell"));
    if !has_restricted_shell {
        // Preserve legacy behavior for existing users who predate this field:
        // do not silently enable restricted shell on upgrade.
        settings.restricted_shell = false;
        settings.pending_restricted_shell_upgrade_notice = true;
    }
    let before_pins = settings.mcp_servers.clone();
    let settings = normalize_settings(settings);
    // Persist package-pin migrations so disk matches the runtime spawn args.
    if settings.mcp_servers != before_pins {
        let _ = persist_settings(app, &settings);
    }
    settings
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    settings.primary_hotkey =
        crate::services::hotkey::normalize_primary_hotkey(&settings.primary_hotkey);
    settings.secondary_hotkey =
        crate::services::hotkey::normalize_hotkey(&settings.secondary_hotkey);
    // OAuth client secrets now live in ignored local files, never in app settings.
    settings.gemini_oauth.client_secret.clear();
    // Pin mcp-remote package versions so OAuth token dirs stay stable across launches.
    let _ = crate::core::mcp::normalize_mcp_servers(&mut settings.mcp_servers);
    settings
}

pub fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let raw = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, raw).map_err(|error| error.to_string())
}

pub fn get_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let state = app
        .try_state::<SettingsState>()
        .ok_or_else(|| "settings state is unavailable".to_string())?;

    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

pub fn set_settings(app: &AppHandle, next: AppSettings) -> Result<AppSettings, String> {
    let next = normalize_settings(next);
    persist_settings(app, &next)?;

    let state = app
        .try_state::<SettingsState>()
        .ok_or_else(|| "settings state is unavailable".to_string())?;

    {
        let mut settings = state.settings.lock().map_err(|error| error.to_string())?;
        *settings = next.clone();
    }

    apply_runtime_settings(&next);
    register_enabled_mcp_tools(app);

    broadcast_settings(app, &next);
    Ok(next)
}

pub fn apply_runtime_settings(settings: &AppSettings) {
    apply_chat_request_settings(settings);
    crate::services::hotkey::configure_primary_hotkey(&settings.primary_hotkey);
    crate::services::hotkey::configure_primary_hotkey_enabled(settings.primary_hotkey_enabled);
    crate::services::hotkey::configure_secondary_hotkey(&settings.secondary_hotkey);
    crate::services::hotkey::configure_secondary_hotkey_enabled(settings.secondary_hotkey_enabled);
    crate::core::tools::tool_approval::shared_tool_approval_store()
        .configure(settings.tool_approval_mode);
    crate::core::tools::sandbox::configure(
        settings.allow_outside_workspace_writes,
        settings.restricted_shell,
        settings.shell_timeout_secs,
        settings.shell_stall_timeout_secs,
    );
    crate::core::lsp::shared_lsp_manager().configure(settings);
    crate::core::mcp::shared_mcp_manager().configure(settings);
    crate::core::tools::skills::configure_enabled_builtin_skills(&settings.enabled_builtin_skills);
    crate::services::pin_badge::configure_from_settings(settings);
}

pub fn apply_chat_request_settings(settings: &AppSettings) {
    crate::core::tools::memory::shared_memory_store().configure(settings);
    crate::runtime::search::shared_search_runtime().configure(settings);
}

pub fn register_enabled_mcp_tools(app: &AppHandle) {
    // Connecting MCP (npx/uvx cold start) can block for a long time; never
    // hold startup or the settings UI on that work.
    if let Some(app_state) = app.try_state::<crate::app_state::AppState>() {
        let registry: Arc<_> = app_state.core.tools().registry();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = crate::core::mcp::shared_mcp_manager().register_enabled(registry.as_ref());
        });
    }
}

pub fn broadcast_settings(app: &AppHandle, settings: &AppSettings) {
    let _ = app.emit("settings-changed", settings.clone());
}

#[cfg(test)]
mod tests {
    use super::normalize_settings;
    use crate::models::settings::AppSettings;

    #[test]
    fn normalizes_runtime_only_settings_before_storage() {
        let mut settings = AppSettings::default();
        settings.gemini_oauth.client_secret = "secret-from-ui".into();

        let normalized = normalize_settings(settings);

        assert!(normalized.gemini_oauth.client_secret.is_empty());
    }

    #[test]
    fn legacy_json_without_restricted_shell_stays_off_with_notice() {
        let raw = serde_json::json!({
            "language": "zhCn",
            "chatModel": "gpt-4o"
        });
        let mut settings: AppSettings = serde_json::from_value(raw.clone()).unwrap_or_default();
        let has_restricted_shell = raw
            .as_object()
            .is_some_and(|obj| obj.contains_key("restrictedShell"));
        if !has_restricted_shell {
            settings.restricted_shell = false;
            settings.pending_restricted_shell_upgrade_notice = true;
        }
        assert!(!settings.restricted_shell);
        assert!(settings.pending_restricted_shell_upgrade_notice);
    }
}
