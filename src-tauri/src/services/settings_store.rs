use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::models::settings::AppSettings;

static WEBVIEW_GPU_DISABLED: AtomicBool = AtomicBool::new(false);
static SETTINGS_WRITE_LOCK: Mutex<()> = Mutex::new(());
static SETTINGS_UPDATE_LOCK: Mutex<()> = Mutex::new(());
pub const RESET_MARKER: &str = ".factory-reset";

fn legacy_migration_allowed(path: &Path) -> bool {
    !path
        .parent()
        .is_some_and(|dir| dir.join(RESET_MARKER).exists())
}

fn read_valid_settings(path: &Path) -> Result<(serde_json::Value, AppSettings), String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if !parsed.is_object() {
        return Err("settings must be an object".into());
    }
    let settings = serde_json::from_value(parsed.clone()).map_err(|e| e.to_string())?;
    Ok((parsed, settings))
}

fn read_settings_with_backup(path: &Path) -> Result<(serde_json::Value, AppSettings), String> {
    read_valid_settings(path).or_else(|_| read_valid_settings(&path.with_extension("json.bak")))
}

fn persist_settings_file(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let _guard = SETTINGS_WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    if read_valid_settings(path).is_ok() {
        let previous = fs::read(path).map_err(|e| e.to_string())?;
        crate::core::tools::file_io::atomic_write(&path.with_extension("json.bak"), previous)
            .map_err(|e| e.to_string())?;
    } else if path.exists() {
        // Keep the damaged original for recovery before an explicit save.
        let damaged = path.with_extension(format!("json.corrupt-{}", uuid::Uuid::new_v4()));
        fs::copy(path, damaged).map_err(|e| e.to_string())?;
    }
    crate::core::tools::file_io::atomic_write(path, raw).map_err(|e| e.to_string())
}

/// Keep embedded wallpaper bytes out of settings snapshots and IPC events.
/// Content-addressed names also let identical images shared by themes use one file.
fn materialize_background_image(
    image: &mut Option<String>,
    directory: &Path,
) -> Result<bool, String> {
    let Some(source) = image.as_deref() else {
        return Ok(false);
    };
    if !source.starts_with("data:") {
        return Ok(false);
    }
    let encoded = source
        .strip_prefix("data:image/")
        .and_then(|value| value.split_once(";base64,"))
        .ok_or("unsupported embedded theme background image")?;
    let extension = match encoded.0.to_ascii_lowercase().as_str() {
        "png" => "png",
        "jpeg" | "jpg" => "jpg",
        "webp" => "webp",
        "gif" => "gif",
        "svg+xml" => "svg",
        _ => return Err("unsupported embedded theme background image format".into()),
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.1)
        .map_err(|error| format!("invalid embedded theme background image: {error}"))?;
    let digest = Sha256::digest(&bytes);
    let path = directory.join(format!("{digest:x}.{extension}"));
    if !path.exists() {
        crate::core::tools::file_io::atomic_write(&path, bytes)
            .map_err(|error| error.to_string())?;
    }
    *image = Some(path.to_string_lossy().into_owned());
    Ok(true)
}

fn materialize_background_images(
    settings: &mut AppSettings,
    directory: &Path,
) -> Result<bool, String> {
    let mut changed = false;
    if let Some(background) = settings.custom_background.as_mut() {
        changed |= materialize_background_image(&mut background.image, directory)?;
    }
    for theme in &mut settings.custom_themes {
        if let Some(background) = theme.background.as_mut() {
            changed |= materialize_background_image(&mut background.image, directory)?;
        }
    }
    Ok(changed)
}

/// Once the current settings are path-based, replace an old oversized recovery
/// snapshot with a valid copy of the current settings. The atomic replacement
/// leaves the legacy backup intact if it fails.
fn refresh_legacy_background_backup(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let backup = path.with_extension("json.bak");
    let old = match fs::read_to_string(&backup) {
        Ok(old) => old,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if !old.contains("data:image/") {
        return Ok(());
    }
    let current = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    if current
        .windows(b"data:image/".len())
        .any(|part| part == b"data:image/")
    {
        return Ok(());
    }
    if read_valid_settings(path).is_err() {
        return Ok(());
    }
    crate::core::tools::file_io::atomic_write(&backup, current).map_err(|error| error.to_string())
}

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
    commit_lock: Mutex<()>,
}

pub fn webview_gpu_disabled() -> bool {
    WEBVIEW_GPU_DISABLED.load(Ordering::Relaxed)
}

fn should_disable_webview_gpu(
    hardware_acceleration_enabled: bool,
    chrome_frosted_glass: bool,
) -> bool {
    !hardware_acceleration_enabled && !chrome_frosted_glass
}

/// Read the settings needed before Tauri creates the WebView2 environment.
/// Hardware acceleration defaults off. Frosted-glass chrome also needs GPU
/// compositing, otherwise transparent regions cannot blend with DWM Acrylic.
pub fn configure_prestart_webview() {
    let Some(app_data) = std::env::var_os("APPDATA") else {
        // No settings file yet — apply the default (GPU off).
        disable_webview_gpu();
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
    let settings_file = if path.is_file() || !legacy_migration_allowed(&path) {
        path
    } else {
        legacy_path
    };
    let parsed = read_settings_with_backup(&settings_file)
        .ok()
        .map(|(value, _)| value);
    let hardware_acceleration_enabled = parsed
        .as_ref()
        .and_then(|settings| settings.get("hardwareAccelerationEnabled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let chrome_frosted_glass = parsed
        .as_ref()
        .and_then(|settings| settings.get("chromeFrostedGlass"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if should_disable_webview_gpu(hardware_acceleration_enabled, chrome_frosted_glass) {
        disable_webview_gpu();
        return;
    }
    WEBVIEW_GPU_DISABLED.store(false, Ordering::Relaxed);
}

fn disable_webview_gpu() {
    apply_disable_gpu_args();
    WEBVIEW_GPU_DISABLED.store(true, Ordering::Relaxed);
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
            commit_lock: Mutex::new(()),
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
    if !legacy_migration_allowed(new_path) {
        return;
    }
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

    let (parsed, mut settings) = match read_settings_with_backup(&path) {
        Ok(settings) => settings,
        Err(error) => {
            tracing::error!(%error, "settings could not be loaded; preserving original files");
            return AppSettings::default();
        }
    };
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
    let timeout_migrated = migrate_shell_timeout(&mut settings);
    let mut settings = normalize_settings(settings);
    let mut migrated_backgrounds = false;
    let mut converted = settings.clone();
    match materialize_background_images(&mut converted, &path.with_file_name("theme-backgrounds")) {
        Ok(changed) => {
            settings = converted;
            migrated_backgrounds = changed;
        }
        Err(error) => tracing::warn!(%error, "embedded theme backgrounds could not be migrated"),
    }
    // Persist package-pin migrations so disk matches the runtime spawn args.
    if settings.mcp_servers != before_pins || timeout_migrated || migrated_backgrounds {
        if let Err(error) = persist_settings(app, &settings) {
            tracing::warn!(%error, "migrated settings could not be persisted");
        }
    }
    if let Err(error) = refresh_legacy_background_backup(&path, &settings) {
        tracing::warn!(%error, "legacy theme background backup could not be refreshed");
    }
    settings
}

fn migrate_shell_timeout(settings: &mut AppSettings) -> bool {
    if settings.shell_timeout_migrated {
        return false;
    }
    // 120 seconds was the old unrestricted default. Other values and
    // restricted-shell limits are intentional and remain untouched.
    if !settings.restricted_shell && settings.shell_timeout_secs == 120 {
        settings.shell_timeout_secs = 3600;
    }
    settings.shell_timeout_migrated = true;
    true
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    settings.migrate_legacy_deepseek_models();
    settings.primary_hotkey =
        crate::services::hotkey::normalize_primary_hotkey(&settings.primary_hotkey);
    settings.secondary_hotkey =
        crate::services::hotkey::normalize_hotkey(&settings.secondary_hotkey);
    // Pin mcp-remote package versions so OAuth token dirs stay stable across launches.
    let _ = crate::core::mcp::normalize_mcp_servers(&mut settings.mcp_servers);
    settings
}

pub fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;

    persist_settings_file(&path, settings)
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

/// Serialize writers, but allow readers to use the last committed snapshot during
/// slow disk I/O. Failure never publishes a partial update.
fn commit_update<R>(
    state: &SettingsState,
    change: impl FnOnce(&mut AppSettings) -> Result<R, String>,
    persist: impl FnOnce(&AppSettings) -> Result<(), String>,
) -> Result<(AppSettings, AppSettings, R), String> {
    let _commit = state
        .commit_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let previous = state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let mut next = previous.clone();
    let result = change(&mut next)?;
    let next = normalize_settings(next);
    if next != previous {
        persist(&next)?;
        *state.settings.lock().map_err(|error| error.to_string())? = next.clone();
    }
    Ok((previous, next, result))
}

/// All writers use this transaction, including tools and remote commands.
/// Serializing effects as well as commits prevents an older update being broadcast last.
pub fn update_settings<R>(
    app: &AppHandle,
    change: impl FnOnce(&mut AppSettings) -> Result<R, String>,
) -> Result<(AppSettings, R), String> {
    let _transaction = SETTINGS_UPDATE_LOCK
        .lock()
        .map_err(|error| error.to_string())?;
    let state = app
        .try_state::<SettingsState>()
        .ok_or("settings state is unavailable")?;
    let asset_directory = settings_path(app)?.with_file_name("theme-backgrounds");
    let (previous, next, result) = commit_update(
        &state,
        |settings| {
            let result = change(settings)?;
            materialize_background_images(settings, &asset_directory)?;
            Ok(result)
        },
        |next| persist_settings(app, next),
    )?;
    if previous != next {
        apply_changed_runtime_settings(Some(&previous), &next);
        if previous.chrome_frosted_glass != next.chrome_frosted_glass
            || previous.opacity != next.opacity
            || previous.color_scheme != next.color_scheme
            || previous.custom_themes != next.custom_themes
        {
            crate::services::workbench_glass::apply_from_settings(app, &next);
        }
        if previous.color_scheme != next.color_scheme
            || previous.custom_themes != next.custom_themes
        {
            crate::services::webview_theme::apply_webview_theme(app, &next);
        }
        if previous.mcp_servers != next.mcp_servers
            || previous.smithery_api_key != next.smithery_api_key
        {
            register_enabled_mcp_tools(app);
        }
        broadcast_settings(app, &next);
    }
    Ok((next, result))
}

pub fn patch_settings(
    app: &AppHandle,
    patch: crate::models::settings::AppSettingsPatch,
) -> Result<AppSettings, String> {
    update_settings(app, |current| {
        *current = current.merge(patch);
        Ok(())
    })
    .map(|(saved, _)| saved)
}

pub fn apply_runtime_settings(settings: &AppSettings) {
    apply_changed_runtime_settings(None, settings);
}

fn apply_changed_runtime_settings(previous: Option<&AppSettings>, settings: &AppSettings) {
    macro_rules! changed { ($($field:ident),+) => { previous.map_or(true, |old| $(old.$field != settings.$field)||+) }; }
    if changed!(memory_enabled, mem0_api_key, mem0_user_id, mem0_base_url) {
        crate::core::tools::memory::shared_memory_store().configure(settings);
    }
    if changed!(
        web_search_enabled,
        web_search_provider,
        serper_api_key,
        tavily_api_key
    ) {
        crate::runtime::search::shared_search_runtime().configure(settings);
    }
    if changed!(primary_hotkey) {
        crate::services::hotkey::configure_primary_hotkey(&settings.primary_hotkey);
    }
    if changed!(primary_hotkey_enabled) {
        crate::services::hotkey::configure_primary_hotkey_enabled(settings.primary_hotkey_enabled);
    }
    if changed!(secondary_hotkey) {
        crate::services::hotkey::configure_secondary_hotkey(&settings.secondary_hotkey);
    }
    if changed!(secondary_hotkey_enabled) {
        crate::services::hotkey::configure_secondary_hotkey_enabled(
            settings.secondary_hotkey_enabled,
        );
    }
    if changed!(tool_approval_mode) {
        crate::core::tools::tool_approval::shared_tool_approval_store()
            .configure(settings.tool_approval_mode);
    }
    if changed!(
        allow_outside_workspace_writes,
        restricted_shell,
        shell_timeout_secs,
        shell_stall_timeout_secs
    ) {
        crate::core::tools::sandbox::configure(
            settings.allow_outside_workspace_writes,
            settings.restricted_shell,
            settings.shell_timeout_secs,
            settings.shell_stall_timeout_secs,
        );
    }
    if changed!(lsp_enabled, lsp_servers) {
        crate::core::lsp::shared_lsp_manager().configure(settings);
    }
    if changed!(mcp_servers, smithery_api_key) {
        crate::core::mcp::shared_mcp_manager().configure(settings);
    }
    if changed!(enabled_builtin_skills) {
        crate::core::tools::skills::configure_enabled_builtin_skills(
            &settings.enabled_builtin_skills,
        );
    }
    if changed!(pixpin_pin_ai_enabled, snipaste_pin_ai_enabled) {
        crate::services::pin_badge::configure_from_settings(settings);
    }
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
    use crate::models::settings::{AppSettings, CustomThemeConfig, ThemeBackgroundConfig};

    #[test]
    fn embedded_theme_backgrounds_become_shared_files_and_paths() {
        let directory =
            std::env::temp_dir().join(format!("anya-theme-images-{}", uuid::Uuid::new_v4()));
        let embedded = "data:image/png;base64,iVBORw0KGgo=";
        let mut settings = AppSettings::default();
        settings.custom_background = Some(ThemeBackgroundConfig {
            image: Some(embedded.into()),
            ..Default::default()
        });
        settings.custom_themes.push(CustomThemeConfig {
            id: "test-theme".into(),
            name: "Test".into(),
            mode: "dark".into(),
            description: None,
            tokens: Default::default(),
            background: Some(ThemeBackgroundConfig {
                image: Some(embedded.into()),
                ..Default::default()
            }),
            updated_at: 0,
        });
        assert!(super::materialize_background_images(&mut settings, &directory).unwrap());
        let global_path = settings
            .custom_background
            .as_ref()
            .unwrap()
            .image
            .as_ref()
            .unwrap();
        let theme_path = settings.custom_themes[0]
            .background
            .as_ref()
            .unwrap()
            .image
            .as_ref()
            .unwrap();
        assert_eq!(global_path, theme_path);
        assert_eq!(
            std::fs::read(global_path).unwrap(),
            [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
        assert!(!serde_json::to_string(&settings)
            .unwrap()
            .contains("data:image"));
        assert!(!super::materialize_background_images(&mut settings, &directory).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_embedded_background_does_not_replace_source() {
        let mut image = Some("data:image/png;base64,invalid!".to_string());
        let directory =
            std::env::temp_dir().join(format!("anya-theme-images-{}", uuid::Uuid::new_v4()));
        assert!(super::materialize_background_image(&mut image, &directory).is_err());
        assert_eq!(image.as_deref(), Some("data:image/png;base64,invalid!"));
        assert!(!directory.exists());
    }

    #[test]
    fn legacy_background_backup_is_refreshed_after_migration() {
        let directory =
            std::env::temp_dir().join(format!("anya-theme-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        let current = AppSettings::default();
        std::fs::write(&path, serde_json::to_vec(&current).unwrap()).unwrap();
        let mut legacy = current.clone();
        legacy.custom_background = Some(ThemeBackgroundConfig {
            image: Some("data:image/png;base64,iVBORw0KGgo=".into()),
            ..Default::default()
        });
        let backup = path.with_extension("json.bak");
        std::fs::write(&backup, serde_json::to_vec(&legacy).unwrap()).unwrap();
        super::refresh_legacy_background_backup(&path, &current).unwrap();
        let refreshed = std::fs::read_to_string(&backup).unwrap();
        assert!(!refreshed.contains("data:image"));
        assert!(serde_json::from_str::<AppSettings>(&refreshed).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reset_marker_blocks_legacy_recovery_even_without_settings() {
        let root = std::env::temp_dir().join(format!("anya-reset-marker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.json");
        assert!(super::legacy_migration_allowed(&path));
        std::fs::write(root.join(super::RESET_MARKER), "reset").unwrap();
        assert!(!super::legacy_migration_allowed(&path));
        super::migrate_legacy_settings_file(&path);
        assert!(!path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_updates_preserve_both_writers_and_failed_save_preserves_memory() {
        let state = std::sync::Arc::new(super::SettingsState::new(AppSettings::default()));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let initial = state.settings.lock().unwrap().clone();
        let handles: Vec<_> = (0..2)
            .map(|writer| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..100 {
                        super::commit_update(
                            &state,
                            |current| {
                                if writer == 0 {
                                    current.zoom += 1;
                                } else {
                                    current.shell_timeout_secs += 1;
                                }
                                Ok(())
                            },
                            |_| Ok(()),
                        )
                        .unwrap();
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let saved = state.settings.lock().unwrap().clone();
        assert_eq!(saved.zoom, initial.zoom + 100);
        assert_eq!(saved.shell_timeout_secs, initial.shell_timeout_secs + 100);
        assert!(super::commit_update(
            &state,
            |current| {
                current.deepseek_api_key = "must-not-commit".into();
                Ok(())
            },
            |_| Err("disk unavailable".into())
        )
        .is_err());
        assert_eq!(*state.settings.lock().unwrap(), saved);
    }

    #[test]
    fn slow_save_does_not_lock_settings_readers() {
        let state = super::SettingsState::new(AppSettings::default());
        let original = state.settings.lock().unwrap().clone();
        super::commit_update(
            &state,
            |settings| {
                settings.zoom += 1;
                Ok(())
            },
            |_| {
                // Executed while persistence is in progress: window handlers
                // must still be able to read the previous committed settings.
                let readable = state
                    .settings
                    .try_lock()
                    .expect("disk I/O must not block readers");
                assert_eq!(*readable, original);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(state.settings.lock().unwrap().zoom, original.zoom + 1);
    }

    #[test]
    fn corrupted_settings_recover_backup_without_destroying_original() {
        let base = std::env::temp_dir().join(format!("anya-settings-{}", uuid::Uuid::new_v4()));
        let path = base.join("settings.json");
        let mut settings = AppSettings::default();
        settings.shell_timeout_secs = 777;
        super::persist_settings_file(&path, &settings).unwrap();
        settings.shell_timeout_secs = 888;
        super::persist_settings_file(&path, &settings).unwrap();
        std::fs::write(&path, "{broken").unwrap();
        let (_, recovered) = super::read_settings_with_backup(&path).unwrap();
        assert_eq!(recovered.shell_timeout_secs, 777);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
        super::persist_settings_file(&path, &recovered).unwrap();
        assert_eq!(
            super::read_valid_settings(&path)
                .unwrap()
                .1
                .shell_timeout_secs,
            777
        );
        assert!(std::fs::read_dir(&base).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("corrupt-")));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn invalid_settings_without_backup_are_preserved() {
        let base =
            std::env::temp_dir().join(format!("anya-settings-invalid-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join("settings.json");
        std::fs::write(&path, "null").unwrap();
        assert!(super::read_settings_with_backup(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "null");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn shell_timeout_migration_is_once_and_preserves_custom_limits() {
        for (restricted, original, expected) in
            [(false, 120, 3600), (true, 120, 120), (false, 90, 90)]
        {
            let mut settings = AppSettings {
                restricted_shell: restricted,
                shell_timeout_secs: original,
                shell_timeout_migrated: false,
                ..AppSettings::default()
            };
            assert!(super::migrate_shell_timeout(&mut settings));
            assert_eq!(settings.shell_timeout_secs, expected);
            settings.shell_timeout_secs = 120;
            assert!(!super::migrate_shell_timeout(&mut settings));
            assert_eq!(settings.shell_timeout_secs, 120);
        }
    }

    #[test]
    fn frosted_glass_keeps_webview_gpu_enabled() {
        assert!(super::should_disable_webview_gpu(false, false));
        assert!(!super::should_disable_webview_gpu(true, false));
        assert!(!super::should_disable_webview_gpu(false, true));
        assert!(!super::should_disable_webview_gpu(true, true));
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
