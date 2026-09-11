use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::core::plugins::diagnostics::{clear_plugin_errors, record_plugin_error};
use crate::core::plugins::{
    capability_label, close_plugin_window, delete_plugin, export_plugin_zip, folder_as_picked,
    get_grant, grant_has, import_plugin_from_path, import_user_file, is_official_plugin_id,
    is_plugin_safe_mode, list_plugins, load_manifest, open_plugin_window, peek_plugin_id,
    permission_label, plugin_dir, plugin_safe_mode_reason, read_plugin_ui_source, shared_runtime,
    PickedFile, PluginFsPickOptions, PluginSummary, KNOWN_CAPABILITY_CATEGORIES, KNOWN_PERMISSIONS,
};
use crate::core::tools::memory::{plugins_dir, user_data_dir};

fn plugin_id_from_window(window: &WebviewWindow) -> Result<String, String> {
    window
        .label()
        .strip_prefix("plugin-")
        .map(str::to_string)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "not a plugin window".into())
}

fn assert_plugin_caller(window: &WebviewWindow, plugin_id: &str) -> Result<(), String> {
    load_manifest(plugin_id).map_err(|e| e.to_string())?;
    if window.label() == "workbench" {
        if !get_grant(plugin_id).enabled {
            return Err("plugin is not enabled".into());
        }
        return Ok(());
    }
    let owned = plugin_id_from_window(window)?;
    if owned != plugin_id {
        return Err("plugin id does not match this window".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_user_plugins() -> Result<Vec<PluginSummary>, String> {
    let _ = crate::core::plugins::shared_runtime();
    crate::core::plugins::bundled_ensure().map_err(|e| e.to_string())?;
    list_plugins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_permission_catalog() -> Vec<(String, String)> {
    KNOWN_PERMISSIONS
        .iter()
        .map(|id| ((*id).to_string(), permission_label(id).to_string()))
        .collect()
}

/// Bridges a runtime fault caught in the webview (activate/deactivate/mount)
/// back to the agent process, since that JS heap is invisible to Rust otherwise.
#[tauri::command]
pub fn report_plugin_runtime_error(
    window: WebviewWindow,
    plugin_id: String,
    phase: String,
    message: String,
) -> Result<(), String> {
    assert_plugin_caller(&window, &plugin_id)?;
    record_plugin_error(&plugin_id, &phase, &message);
    Ok(())
}

#[tauri::command]
pub fn clear_plugin_runtime_errors(plugin_id: Option<String>) -> Result<(), String> {
    clear_plugin_errors(plugin_id.as_deref());
    Ok(())
}

#[tauri::command]
pub fn plugin_capability_catalog() -> Vec<(String, String)> {
    KNOWN_CAPABILITY_CATEGORIES
        .iter()
        .map(|id| ((*id).to_string(), capability_label(id).to_string()))
        .collect()
}

#[tauri::command]
pub fn open_user_plugin(app: AppHandle, id: String) -> Result<(), String> {
    open_plugin_window(&app, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_user_plugin_window(app: AppHandle, id: String) -> Result<(), String> {
    close_plugin_window(&app, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_all_user_plugin_windows(app: AppHandle) {
    crate::core::plugins::close_all_plugin_windows(&app);
}

#[tauri::command]
pub fn delete_user_plugin(id: String) -> Result<(), String> {
    if is_official_plugin_id(&id) {
        return Err(format!("cannot uninstall official plugin `{id}`"));
    }
    shared_runtime()
        .uninstall_cleanup(&id)
        .map_err(|e| e.to_string())?;
    delete_plugin(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enable_user_plugin(id: String, permissions: Vec<String>) -> Result<(), String> {
    shared_runtime()
        .enable(&id, permissions)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disable_user_plugin(id: String) -> Result<(), String> {
    shared_runtime().disable(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reload_user_plugin(id: String) -> Result<(), String> {
    shared_runtime().reload(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_user_plugin(id: String, dest: String) -> Result<String, String> {
    export_plugin_zip(&id, std::path::Path::new(&dest))
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_user_plugin(path: String, overwrite: Option<bool>) -> Result<String, String> {
    let path = std::path::PathBuf::from(path);
    let overwrite = overwrite.unwrap_or(false);
    if overwrite {
        if let Ok(id) = peek_plugin_id(&path) {
            if is_official_plugin_id(&id) {
                return Err(format!("cannot import over official plugin `{id}`"));
            }
            let dest = plugin_dir(&id).map_err(|e| e.to_string())?;
            if dest.join("plugin.json").is_file() {
                shared_runtime()
                    .uninstall_cleanup(&id)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    import_plugin_from_path(&path, overwrite).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_plugin_ui_source(id: String) -> Result<String, String> {
    if is_plugin_safe_mode() {
        return Err("plugin safe mode: UI is not loaded".into());
    }
    if !get_grant(&id).enabled {
        return Err("plugin is not enabled".into());
    }
    read_plugin_ui_source(&id).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSafeModeStatus {
    pub active: bool,
    pub reason: String,
}

#[tauri::command]
pub fn plugin_safe_mode_status() -> PluginSafeModeStatus {
    PluginSafeModeStatus {
        active: is_plugin_safe_mode(),
        reason: if is_plugin_safe_mode() {
            plugin_safe_mode_reason()
        } else {
            String::new()
        },
    }
}

#[tauri::command]
pub fn clear_plugin_safe_mode() -> Result<(), String> {
    shared_runtime()
        .exit_safe_mode_and_restore()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_host_rpc(id: String, method: String, params: Value) -> Result<Value, String> {
    shared_runtime()
        .host_rpc(&id, &method, params)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_anya_user_dir() -> Result<String, String> {
    let dir = user_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_plugins_dir(app: AppHandle) -> Result<(), String> {
    let dir = plugins_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.display().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_storage_get(
    window: WebviewWindow,
    plugin_id: String,
    key: String,
) -> Result<Option<String>, String> {
    assert_plugin_caller(&window, &plugin_id)?;
    let store = load_store(&plugin_id)?;
    Ok(store
        .get(&key)
        .and_then(|value| value.as_str())
        .map(str::to_string))
}

#[tauri::command]
pub fn plugin_storage_set(
    window: WebviewWindow,
    plugin_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    assert_plugin_caller(&window, &plugin_id)?;
    if key.len() > 200 || value.len() > 64_000 {
        return Err("storage key/value too large".into());
    }
    let mut store = load_store(&plugin_id)?;
    store.insert(key, serde_json::Value::String(value));
    save_store(&plugin_id, &store)
}

#[tauri::command]
pub fn plugin_ask_anya(
    app: AppHandle,
    window: WebviewWindow,
    plugin_id: String,
    prompt: String,
) -> Result<(), String> {
    assert_plugin_caller(&window, &plugin_id)?;
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("prompt is empty".into());
    }
    let _ = app.emit_to(
        "workbench",
        "plugin-ask-anya",
        serde_json::json!({
            "pluginId": plugin_id,
            "prompt": prompt,
        }),
    );
    if let Some(workbench) = app.get_webview_window("workbench") {
        let _ = workbench.show();
        let _ = workbench.set_focus();
    }
    Ok(())
}

fn store_path(id: &str) -> Result<std::path::PathBuf, String> {
    Ok(plugin_dir(id)
        .map_err(|e| e.to_string())?
        .join("data")
        .join("store.json"))
}

fn load_store(id: &str) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let path = store_path(id)?;
    if !path.is_file() {
        return Ok(serde_json::Map::new());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    Ok(value.as_object().cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn plugin_fs_pick(
    app: AppHandle,
    window: WebviewWindow,
    plugin_id: String,
    options: Option<PluginFsPickOptions>,
) -> Result<Option<Vec<PickedFile>>, String> {
    assert_plugin_caller(&window, &plugin_id)?;
    if !grant_has(&plugin_id, "fs.pick") {
        return Err("permission `fs.pick` is not granted".into());
    }
    let options = options.unwrap_or_default();
    let Some(paths) = run_fs_pick_dialog(&app, &window, &options)? else {
        return Ok(None);
    };
    if options.directory {
        return Ok(Some(paths.iter().map(|p| folder_as_picked(p)).collect()));
    }
    let mut out = Vec::with_capacity(paths.len());
    for path in &paths {
        out.push(import_user_file(&plugin_id, path).map_err(|e| e.to_string())?);
    }
    Ok(Some(out))
}

fn run_fs_pick_dialog(
    app: &AppHandle,
    window: &WebviewWindow,
    options: &PluginFsPickOptions,
) -> Result<Option<Vec<std::path::PathBuf>>, String> {
    let mut builder = app.dialog().file().set_parent(window);
    for filter in &options.filters {
        if filter.name.is_empty() || filter.extensions.is_empty() {
            continue;
        }
        let exts: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        builder = builder.add_filter(&filter.name, &exts);
    }
    let to_path = |fp: tauri_plugin_dialog::FilePath| fp.into_path().map_err(|e| e.to_string());
    if options.directory {
        if options.multiple {
            match builder.blocking_pick_folders() {
                None => Ok(None),
                Some(items) => {
                    let mut paths = Vec::new();
                    for item in items {
                        paths.push(to_path(item)?);
                    }
                    Ok(Some(paths))
                }
            }
        } else {
            match builder.blocking_pick_folder() {
                None => Ok(None),
                Some(item) => Ok(Some(vec![to_path(item)?])),
            }
        }
    } else if options.multiple {
        match builder.blocking_pick_files() {
            None => Ok(None),
            Some(items) => {
                let mut paths = Vec::new();
                for item in items {
                    paths.push(to_path(item)?);
                }
                Ok(Some(paths))
            }
        }
    } else {
        match builder.blocking_pick_file() {
            None => Ok(None),
            Some(item) => Ok(Some(vec![to_path(item)?])),
        }
    }
}

fn save_store(id: &str, store: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    let path = store_path(id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut as_strings = serde_json::Map::new();
    for (k, v) in store {
        let s = v
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| v.to_string());
        as_strings.insert(k.clone(), serde_json::Value::String(s));
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(&as_strings).unwrap_or_else(|_| "{}".into()),
    )
    .map_err(|e| e.to_string())
}
