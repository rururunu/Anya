use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::core::runtime::RequestContext;
use crate::services::window::{
    destroy_overlay, hide_overlay, is_overlay_label, minimize_overlay, set_overlay_chat_mode,
    set_overlay_popup_open, show_settings_window,
};

fn window_sessions() -> &'static Mutex<HashMap<String, String>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
struct TrackedToast {
    title: String,
    body: String,
}

fn tracked_toasts() -> &'static Mutex<HashMap<String, TrackedToast>> {
    static TOASTS: OnceLock<Mutex<HashMap<String, TrackedToast>>> = OnceLock::new();
    TOASTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// System toasts only when the workbench is closed (hidden to tray / not on screen).
fn workbench_is_open(app: &AppHandle) -> bool {
    app.get_webview_window("workbench").is_some_and(|window| {
        window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
    })
}

#[tauri::command]
pub fn set_window_session_view(window: WebviewWindow, session_id: Option<String>) {
    if let Ok(mut sessions) = window_sessions().lock() {
        match session_id.filter(|id| !id.is_empty()) {
            Some(session_id) => {
                sessions.insert(window.label().to_string(), session_id);
            }
            None => {
                sessions.remove(window.label());
            }
        }
    }
}

fn dismiss_matching_notification(
    app_id: &str,
    title: &str,
    body: &str,
) -> windows::core::Result<()> {
    use windows::core::HSTRING;
    use windows::UI::Notifications::ToastNotificationManager;

    let app_id = HSTRING::from(app_id);
    let history = ToastNotificationManager::History()?;
    let notifications = history.GetHistoryWithId(&app_id)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;

    for index in 0..notifications.Size()? {
        let toast = notifications.GetAt(index)?;
        let text_nodes = toast
            .Content()?
            .GetElementsByTagName(&HSTRING::from("text"))?;
        let mut has_title = false;
        let mut has_body = false;
        for text_index in 0..text_nodes.Length()? {
            let text = text_nodes.Item(text_index)?.InnerText()?.to_string();
            has_title |= text == title;
            has_body |= text == body;
        }
        if has_title && has_body {
            notifier.Hide(&toast)?;
            break;
        }
    }
    Ok(())
}

fn remember_toast(keys: &[String], title: &str, body: &str) {
    if keys.is_empty() {
        return;
    }
    let toast = TrackedToast {
        title: title.to_string(),
        body: body.to_string(),
    };
    if let Ok(mut guard) = tracked_toasts().lock() {
        for key in keys {
            if !key.is_empty() {
                guard.insert(key.clone(), toast.clone());
            }
        }
    }
}

fn take_tracked_toasts(request_id: Option<&str>, session_id: Option<&str>) -> Vec<TrackedToast> {
    let mut out = Vec::new();
    let Ok(mut guard) = tracked_toasts().lock() else {
        return out;
    };
    if let Some(id) = request_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(toast) = guard.remove(id) {
            out.push(toast);
        }
    }
    if let Some(id) = session_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(toast) = guard.remove(id) {
            if !out.iter().any(|t| t.title == toast.title && t.body == toast.body) {
                out.push(toast);
            }
        }
    }
    out
}

/// Hide a previously shown interaction toast (e.g. phone answered the ask/approval).
pub fn dismiss_tracked_interaction_notifications(
    app: &AppHandle,
    request_id: Option<&str>,
    session_id: Option<&str>,
) {
    let app_id = app.config().identifier.clone();
    for toast in take_tracked_toasts(request_id, session_id) {
        if let Err(error) = dismiss_matching_notification(&app_id, &toast.title, &toast.body) {
            tracing::warn!(%error, "failed to dismiss interaction notification");
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionNotificationRequest {
    session_id: String,
    #[serde(default)]
    request_id: Option<String>,
    title: String,
    body: String,
    ignore_label: String,
    open_label: String,
    #[serde(default)]
    persistent: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissInteractionNotificationRequest {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
}

#[tauri::command]
pub fn dismiss_interaction_notification(
    app: AppHandle,
    request: DismissInteractionNotificationRequest,
) -> Result<(), String> {
    dismiss_tracked_interaction_notifications(
        &app,
        request.request_id.as_deref(),
        request.session_id.as_deref(),
    );
    Ok(())
}

#[tauri::command]
pub fn show_interaction_notification(
    app: AppHandle,
    request: InteractionNotificationRequest,
) -> Result<(), String> {
    // Only notify when the workbench is closed (tray). Open / minimized-on-taskbar
    // means the user can already see the conversation UI.
    if workbench_is_open(&app) {
        return Ok(());
    }
    let mut keys = Vec::new();
    if let Some(request_id) = request
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        keys.push(request_id.to_string());
    }
    if !request.session_id.trim().is_empty() {
        keys.push(request.session_id.clone());
    }
    remember_toast(&keys, &request.title, &request.body);

    std::thread::spawn(move || {
        let mut notification = notify_rust::Notification::new();
        notification
            .summary(&request.title)
            .body(&request.body)
            .action("ignore", &request.ignore_label)
            .action("open", &request.open_label);

        if request.persistent {
            notification.urgency(notify_rust::Urgency::Critical);
        }

        if !tauri::is_dev() {
            notification.app_id(&app.config().identifier);
        }

        match notification.show() {
            Ok(handle) => {
                let result = handle.wait_for_response(
                    |response: &notify_rust::NotificationResponse| {
                        let should_open = match response {
                            notify_rust::NotificationResponse::Default => true,
                            notify_rust::NotificationResponse::Action(action) => action == "open",
                            _ => false,
                        };
                        let should_dismiss = matches!(
                            response,
                            notify_rust::NotificationResponse::Default
                                | notify_rust::NotificationResponse::Action(_)
                        );
                        if should_dismiss {
                            dismiss_tracked_interaction_notifications(
                                &app,
                                request.request_id.as_deref(),
                                Some(request.session_id.as_str()),
                            );
                        }
                        if !should_open {
                            return;
                        }
                        let app_handle = app.clone();
                        let session_id = request.session_id.clone();
                        let _ = app.run_on_main_thread(move || {
                            crate::services::window::show_workbench_window(&app_handle);
                            let _ = app_handle
                                .emit_to("workbench", "workbench-open-session", session_id);
                        });
                    },
                );
                if let Err(error) = result {
                    tracing::warn!(%error, "failed to wait for interaction notification action");
                }
            }
            Err(error) => tracing::error!(%error, "failed to show interaction notification"),
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn open_session_in_overlay(app: AppHandle, session_id: String) -> Result<(), String> {
    tracing::debug!(source = "open_session_in_overlay", "overlay opening start");
    let captured = crate::core::context::store::capture_now();
    let context = app
        .try_state::<crate::app_state::AppState>()
        .map(|state| state.core.chat().environment_context())
        .unwrap_or(captured);
    let all_windows = app.webview_windows();
    let overlay_windows: Vec<_> = all_windows
        .iter()
        .filter(|(label, _)| crate::services::window::is_overlay_label(label))
        .map(|(_, window)| window)
        .collect();

    for window in &overlay_windows {
        let _ = window.emit("context-captured", &context);
        tracing::debug!(
            label = %window.label(),
            source = "open_session_in_overlay",
            "overlay interactive ready"
        );
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("overlay-shown", ());
    }

    // 延迟 150 毫秒，等待 Webview 激活并且前端 listener 完成挂载/苏醒
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let _ = app.emit("open-session", session_id);
    Ok(())
}

#[tauri::command]
pub fn open_session_in_workbench(
    app: AppHandle,
    session_id: String,
    overlay_label: String,
) -> Result<(), String> {
    let workbench = app
        .get_webview_window("workbench")
        .ok_or_else(|| "workbench window is missing".to_string())?;

    if workbench.is_minimized().unwrap_or(false) {
        let _ = workbench.unminimize();
    }
    let _ = workbench.set_always_on_top(false);
    workbench.show().map_err(|error| error.to_string())?;
    let _ = workbench.set_focus();
    let emit_result = workbench
        .emit("workbench-open-session", session_id)
        .map_err(|error| error.to_string());

    if overlay_label == "overlay" {
        hide_overlay(&app, &overlay_label);
    } else if is_overlay_label(&overlay_label) {
        destroy_overlay(&app, &overlay_label);
    }

    emit_result
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    show_settings_window(&app);
}

#[tauri::command]
pub fn hide_overlay_window(app: AppHandle, label: Option<String>) {
    let label = label.unwrap_or_else(|| "overlay".to_string());
    hide_overlay(&app, &label);
}

#[tauri::command]
pub fn minimize_overlay_window(app: AppHandle, label: Option<String>) {
    let label = label.unwrap_or_else(|| "overlay".to_string());
    minimize_overlay(&app, &label);
}

/// 前端调用：关闭并销毁窗口（适用于聊天窗口的关闭按钮）
#[tauri::command]
pub fn close_overlay_window(app: AppHandle, label: String) {
    if label == "overlay" {
        // 基础窗口只隐藏不销毁
        hide_overlay(&app, &label);
    } else if is_overlay_label(&label) {
        destroy_overlay(&app, &label);
    }
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn set_overlay_chat_mode_command(label: String, enabled: bool) {
    set_overlay_chat_mode(&label, enabled);
}

#[tauri::command]
pub fn set_overlay_popup_open_command(label: String, open: bool) {
    set_overlay_popup_open(&label, open);
}

#[tauri::command]
pub fn take_overlay_context(label: String) -> Option<RequestContext> {
    crate::services::window::take_overlay_context(&label)
}

use std::path::{Path, PathBuf};

fn preview_image_store() -> &'static Mutex<String> {
    static STORE: OnceLock<Mutex<String>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(String::new()))
}

/// Persist preview payload as a local file and return `path:<abs>` for the frontend.
fn cache_preview_payload(app: &AppHandle, path_or_base64: &str) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache dir unavailable: {e}"))?
        .join("image-preview");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("failed to create preview cache: {e}"))?;

    let (ext, bytes) = if let Some(rest) = path_or_base64.strip_prefix("data:") {
        let (meta, data) = rest
            .split_once(',')
            .ok_or_else(|| "invalid data URL".to_string())?;
        let mime = meta.split(';').next().unwrap_or("image/png");
        let ext = match mime {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "png",
        };
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD
            .decode(data.trim())
            .map_err(|e| format!("invalid base64 image: {e}"))?;
        (ext, bytes)
    } else if Path::new(path_or_base64).exists() {
        let ext = if path_or_base64.ends_with(".jpg") || path_or_base64.ends_with(".jpeg") {
            "jpg"
        } else if path_or_base64.ends_with(".gif") {
            "gif"
        } else if path_or_base64.ends_with(".webp") {
            "webp"
        } else {
            "png"
        };
        let bytes =
            std::fs::read(path_or_base64).map_err(|e| format!("Failed to read image file: {e}"))?;
        (ext, bytes)
    } else {
        return Err("unsupported image payload".into());
    };

    let out: PathBuf = cache_dir.join(format!("current.{ext}"));
    std::fs::write(&out, bytes).map_err(|e| format!("failed to write preview cache: {e}"))?;
    Ok(format!("path:{}", out.to_string_lossy()))
}

#[tauri::command]
pub fn get_preview_image() -> String {
    if let Ok(guard) = preview_image_store().lock() {
        guard.clone()
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn open_image_preview(app: AppHandle, path_or_base64: String) -> Result<(), String> {
    let stored = cache_preview_payload(&app, &path_or_base64)?;
    if let Ok(mut guard) = preview_image_store().lock() {
        *guard = stored;
    }

    crate::services::window::set_overlay_popup_open("overlay", true);

    // Reuse an existing preview window when possible.
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-preview-") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("preview-image-updated", ());
            return Ok(());
        }
    }

    let label = format!("overlay-preview-{}", uuid::Uuid::new_v4());

    let mut x_pos = None;
    let mut y_pos = None;

    if let Some(overlay_win) = app.get_webview_window("overlay") {
        if let (Ok(outer_pos), Ok(outer_size)) =
            (overlay_win.outer_position(), overlay_win.outer_size())
        {
            let scale_factor = overlay_win.scale_factor().unwrap_or(1.0);
            let logical_pos = outer_pos.to_logical::<f64>(scale_factor);
            let logical_size = outer_size.to_logical::<f64>(scale_factor);

            let left_x = logical_pos.x - 740.0;
            if left_x >= 10.0 {
                x_pos = Some(left_x);
            } else {
                x_pos = Some(logical_pos.x + logical_size.width + 20.0);
            }
            y_pos = Some(logical_pos.y);
        }
    }

    let url_str = "/#/image-preview";

    let mut window_builder =
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App(url_str.into()))
            .title("Preview")
            .inner_size(720.0, 520.0)
            .resizable(true)
            .decorations(false);

    if let (Some(x), Some(y)) = (x_pos, y_pos) {
        window_builder = window_builder.position(x, y);
    } else {
        window_builder = window_builder.center();
    }

    let window = window_builder
        .build()
        .map_err(|e| format!("Failed to build window: {e}"))?;
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}
