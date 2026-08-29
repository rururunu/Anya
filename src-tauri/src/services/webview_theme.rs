use tauri::{AppHandle, Manager, Theme};

use crate::models::settings::AppSettings;

/// Keep WebView2 on the light color-scheme so native paints are not inverted.
/// Dark appearance comes from CSS tokens on `html[data-theme="dark"]`.
pub fn apply_webview_theme(app: &AppHandle, _settings: &AppSettings) {
    for (_, window) in app.webview_windows() {
        if let Err(error) = window.set_theme(Some(Theme::Light)) {
            tracing::warn!(
                label = %window.label(),
                ?error,
                "failed to sync webview theme"
            );
        }
    }
}
