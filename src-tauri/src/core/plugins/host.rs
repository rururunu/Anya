//! Open a plugin in an isolated WebView window.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use super::{load_manifest, PLUGIN_SCHEME};
use crate::core::tools::error::ToolError;

pub fn window_label(id: &str) -> String {
    format!("plugin-{id}")
}

/// Close a plugin window without going through tray `ExitRequested` (which would keep it open).
pub fn close_plugin_window(app: &AppHandle, id: &str) -> Result<(), ToolError> {
    destroy_plugin_window_label(app, &window_label(id))
}

/// Destroy every isolated plugin window (tray close-intercept cannot keep them).
pub fn close_all_plugin_windows(app: &AppHandle) {
    let labels: Vec<String> = app
        .webview_windows()
        .into_keys()
        .filter(|label| label.starts_with("plugin-"))
        .collect();
    for label in labels {
        let _ = destroy_plugin_window_label(app, &label);
    }
}

pub fn destroy_plugin_window_label(app: &AppHandle, label: &str) -> Result<(), ToolError> {
    if let Some(window) = app.get_webview_window(label) {
        window
            .destroy()
            .map_err(|e| ToolError::new(format!("close plugin window: {e}")))?;
    }
    Ok(())
}

pub fn open_plugin_window(app: &AppHandle, id: &str) -> Result<(), ToolError> {
    if super::safe::is_plugin_safe_mode() {
        return Err(ToolError::new("plugin safe mode: windows are not opened"));
    }
    let manifest = load_manifest(id)?;
    let label = window_label(&manifest.id);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        let _ = existing.unminimize();
        let _ = existing.show();
        return Ok(());
    }
    let entry = manifest.entry.trim_start_matches('/');
    let url = format!("{PLUGIN_SCHEME}://localhost/{}/{entry}", manifest.id)
        .parse()
        .map_err(|e| ToolError::new(format!("plugin url: {e}")))?;
    let init = host_bridge_script(&manifest.id);
    WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url))
        .title(&manifest.name)
        .inner_size(
            f64::from(manifest.window.width.max(400)),
            f64::from(manifest.window.height.max(300)),
        )
        .resizable(true)
        .decorations(true)
        .closable(true)
        .minimizable(true)
        .skip_taskbar(false)
        .always_on_top(false)
        .transparent(false)
        .shadow(true)
        .background_color(tauri::window::Color(0x12, 0x14, 0x1a, 0xff))
        .initialization_script(&init)
        .build()
        .map_err(|e| ToolError::new(format!("open plugin window: {e}")))?;
    Ok(())
}

fn host_bridge_script(plugin_id: &str) -> String {
    format!(
        r#"
(() => {{
  const id = {id};
  const invoke = (cmd, args) => {{
    const api = window.__TAURI_INTERNALS__;
    if (!api || typeof api.invoke !== "function") {{
      return Promise.reject(new Error("Anya plugin host is unavailable"));
    }}
    return api.invoke(cmd, args);
  }};
  window.AnyaPlugin = Object.freeze({{
    id,
    close: () => invoke("close_user_plugin_window", {{ id }}),
    storageGet: (key) => invoke("plugin_storage_get", {{ pluginId: id, key: String(key) }}),
    storageSet: (key, value) => invoke("plugin_storage_set", {{
      pluginId: id, key: String(key), value: value == null ? "" : String(value)
    }}),
    askAnya: (prompt) => invoke("plugin_ask_anya", {{ pluginId: id, prompt: String(prompt) }}),
    pick: (options) => invoke("plugin_fs_pick", {{ pluginId: id, options: options && typeof options === "object" ? options : {{}} }}),
  }});
  document.addEventListener("keydown", (ev) => {{
    if (ev.key === "Escape") window.AnyaPlugin.close();
  }});
}})();
"#,
        id = serde_json::to_string(plugin_id).unwrap_or_else(|_| "\"\"".into()),
    )
}
