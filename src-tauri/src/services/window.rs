use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app_state::AppState;
use crate::core::context::store::capture_now;
use crate::core::runtime::RequestContext;
use crate::services::overlay_native::{
    clear_minimize_pending, clear_overlay_native_minimized, hide_overlay_without_flash,
    is_minimize_pending, is_overlay_native_minimized, mark_minimize_pending,
    mark_overlay_native_minimized, minimize_window, reapply_toolwindow_style,
    show_overlay_without_flash,
};
use tauri::WebviewUrl;
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};

const WINDOW_BLUR_GUARD_MS: u64 = 200;
const WINDOW_MINIMIZE_BLUR_GUARD_MS: u64 = 800;

static OVERLAY_IGNORE_BLUR_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

static OVERLAY_CHAT_MODE_LABELS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static OVERLAY_POPUP_OPEN_LABELS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static PENDING_OVERLAY_CONTEXTS: OnceLock<Mutex<HashMap<String, RequestContext>>> = OnceLock::new();

fn chat_mode_labels() -> std::sync::MutexGuard<'static, HashSet<String>> {
    OVERLAY_CHAT_MODE_LABELS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn popup_open_labels() -> std::sync::MutexGuard<'static, HashSet<String>> {
    OVERLAY_POPUP_OPEN_LABELS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn pending_contexts() -> std::sync::MutexGuard<'static, HashMap<String, RequestContext>> {
    PENDING_OVERLAY_CONTEXTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn take_overlay_context(label: &str) -> Option<RequestContext> {
    pending_contexts().remove(label)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn mark_blur_guard() {
    mark_blur_guard_for(WINDOW_BLUR_GUARD_MS);
}

pub fn mark_blur_guard_for(ms: u64) {
    OVERLAY_IGNORE_BLUR_UNTIL_MS.store(now_millis().saturating_add(ms), Ordering::Relaxed);
}

pub fn should_ignore_overlay_blur() -> bool {
    now_millis() < OVERLAY_IGNORE_BLUR_UNTIL_MS.load(Ordering::Relaxed)
}

pub fn set_overlay_chat_mode(label: &str, enabled: bool) {
    let mut guard = chat_mode_labels();
    if enabled {
        guard.insert(label.to_string());
    } else {
        guard.remove(label);
    }
}

pub fn is_overlay_in_chat_mode(label: &str) -> bool {
    chat_mode_labels().contains(label)
}

fn is_available_input_window(app: &AppHandle, label: &str) -> bool {
    if is_overlay_in_chat_mode(label) {
        return false;
    }
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    let Ok(size) = window.inner_size() else {
        return false;
    };
    let scale = window.scale_factor().unwrap_or(1.0).max(0.1);
    size.height as f64 / scale <= 120.0
}

pub fn set_overlay_popup_open(label: &str, open: bool) {
    let mut guard = popup_open_labels();
    if open {
        guard.insert(label.to_string());
        mark_blur_guard();
    } else {
        guard.remove(label);
    }
}

pub fn is_overlay_popup_open(label: &str) -> bool {
    popup_open_labels().contains(label)
}

pub fn should_keep_overlay_visible(label: &str) -> bool {
    is_overlay_in_chat_mode(label)
        || is_overlay_popup_open(label)
        || is_overlay_native_minimized(label)
        || is_minimize_pending()
        || should_ignore_overlay_blur()
}

pub fn cleanup_overlay_state(label: &str) {
    chat_mode_labels().remove(label);
    popup_open_labels().remove(label);
    pending_contexts().remove(label);
    clear_overlay_native_minimized(label);
}

pub fn is_overlay_label(label: &str) -> bool {
    (label == "overlay" || label.starts_with("overlay-")) && !label.starts_with("overlay-preview-")
}

/// 最小化 overlay：Windows 原生最小化到任务栏，保留 chat 状态
pub fn minimize_overlay(app: &AppHandle, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };

    mark_minimize_pending();
    mark_blur_guard_for(WINDOW_MINIMIZE_BLUR_GUARD_MS);
    let _ = window.set_skip_taskbar(false);

    if minimize_window(&window).is_ok() {
        mark_overlay_native_minimized(label);
        return;
    }

    clear_minimize_pending();
    let _ = window.minimize();
    mark_overlay_native_minimized(label);
}

pub fn handle_overlay_focused(app: &AppHandle, label: &str) {
    if !is_overlay_label(label) {
        return;
    }

    let Some(window) = app.get_webview_window(label) else {
        return;
    };

    clear_minimize_pending();

    if !is_overlay_native_minimized(label) {
        return;
    }

    if window.is_minimized().unwrap_or(false) {
        return;
    }

    clear_overlay_native_minimized(label);
    configure_overlay_window(&window);
    let _ = window.emit_to(label, "overlay-shown", ());
}

/// 隐藏指定 overlay 窗口（不销毁，用于基础 overlay 窗口）
pub fn hide_overlay(app: &AppHandle, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };

    if !window.is_visible().unwrap_or(false) {
        return;
    }

    set_overlay_chat_mode(label, false);
    set_overlay_popup_open(label, false);
    if hide_overlay_without_flash(&window).is_err() {
        let _ = window.hide();
    }
    let _ = window.emit_to(label, "overlay-hidden", ());
}

/// 销毁指定 overlay 窗口（用于动态创建的 overlay-N 窗口）
pub fn destroy_overlay(app: &AppHandle, label: &str) {
    cleanup_overlay_state(label);
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    // 销毁窗口会导致焦点切换到其他窗口，设置 blur guard 防止其他 overlay
    // 窗口因失焦事件而被错误地隐藏（否则会导致其他窗口的消息框消失）
    mark_blur_guard();
    let _ = window.destroy();
}

pub fn configure_overlay_window(window: &tauri::WebviewWindow) {
    let _ = window.set_shadow(false);
    let _ = window.set_maximizable(false);
    let _ = window.set_skip_taskbar(true);

    reapply_toolwindow_style(window);
    // Re-assert after style changes — some Windows builds re-enable DWM shadow.
    let _ = window.set_shadow(false);
}

fn show_and_focus_overlay(window: &tauri::WebviewWindow) {
    configure_overlay_window(window);
    if show_overlay_without_flash(window).is_err() {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn next_overlay_label(app: &AppHandle) -> String {
    let mut i = 1u32;
    loop {
        let label = format!("overlay-{i}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
        i += 1;
    }
}

fn create_new_overlay(app: &AppHandle, context: &RequestContext) {
    let label = next_overlay_label(app);
    pending_contexts().insert(label.clone(), context.clone());
    match WebviewWindowBuilder::new(app, &label, WebviewUrl::App("/#/overlay".into()))
        .title(app.package_info().name.clone())
        // Keep in sync with Overlay.vue INPUT_HEIGHT (82px bar + 1px dock borders).
        .inner_size(640.0, 84.0)
        .min_inner_size(640.0, 84.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .center()
        .focused(false)
        .resizable(false)
        .maximizable(false)
        .build()
    {
        Ok(window) => {
            let _ = window.center();
            tracing::debug!(label = %label, source = "toggle_overlay", "overlay interactive ready");
            show_and_focus_overlay(&window);
            // 不在这里发 overlay-shown，前端 onMounted 检测 isVisible() 后自行初始化
            mark_blur_guard();
        }
        Err(e) => {
            pending_contexts().remove(&label);
            eprintln!("failed to create overlay window: {e:?}");
        }
    }
}

fn emit_context_captured(app: &AppHandle, label: &str, context: &RequestContext) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.emit_to(label, "context-captured", context);
    }
}

fn resolve_environment_context(app: &AppHandle, captured: RequestContext) -> RequestContext {
    let resolved = app
        .try_state::<AppState>()
        .map(|state| state.core.chat().environment_context_for_overlay())
        .unwrap_or(captured);
    tracing::debug!(
        active_window = ?resolved.active_window,
        active_file = ?resolved.active_file,
        workspace = ?resolved.workspace,
        has_git_status = resolved.git_status.is_some(),
        "overlay resolved environment context"
    );
    resolved
}

fn has_selected_context(context: &RequestContext) -> bool {
    context
        .selection
        .as_ref()
        .is_some_and(|selection| !selection.trim().is_empty())
        || !context.selected_files.is_empty()
        || !context.selected_images.is_empty()
}

/// 主快捷键逻辑：
/// 1. 如果存在未开始聊天的 input 窗口（未处于 chat mode）→ 切换显示/隐藏
/// 2. 如果所有可见 overlay 都在 chat mode → 创建新窗口
///
/// `mouse_pos`：双击 Alt 时的鼠标物理坐标，有值则弹窗定位到鼠标附近，
/// 否则居中显示（无选中内容时的兜底行为）。
pub fn toggle_overlay(app: &AppHandle, mouse_pos: Option<(i32, i32)>) {
    tracing::debug!(source = "toggle_overlay", "overlay opening start");
    let all_windows = app.webview_windows();

    // 找所有 overlay 类型窗口，按 label 排序以保证确定性
    let mut overlay_labels: Vec<String> = all_windows
        .keys()
        .filter(|label| is_overlay_label(label))
        .cloned()
        .collect();
    overlay_labels.sort();

    // An active chat represents an in-progress session. The global shortcut
    // must open a separate draft instead of reusing another hidden input
    // window, otherwise users cannot start a second conversation reliably.
    let has_visible_chat = overlay_labels.iter().any(|label| {
        is_overlay_in_chat_mode(label)
            && app
                .get_webview_window(label)
                .is_some_and(|window| window.is_visible().unwrap_or(false))
    });
    if has_visible_chat {
        let context = resolve_environment_context(app, capture_now());
        if let Some((mx, my)) = mouse_pos.filter(|_| has_selected_context(&context)) {
            place_and_show_overlay_at_mouse(app, mx, my, &context);
        } else {
            create_new_overlay(app, &context);
        }
        return;
    }

    let input_label = overlay_labels
        .iter()
        .find(|label| is_available_input_window(app, label));

    if let Some(label) = input_label {
        if let Some(window) = app.get_webview_window(label) {
            let visible = window.is_visible().unwrap_or(false);
            if visible {
                hide_overlay(app, label);
            } else {
                let context = resolve_environment_context(app, capture_now());
                if let Some((mx, my)) = mouse_pos.filter(|_| has_selected_context(&context)) {
                    const WIN_W: f64 = 640.0;
                    const WIN_H: f64 = 84.0;
                    const OFFSET: i32 = 16;
                    let (x, y) = calc_position_near_mouse(&window, mx, my, WIN_W, WIN_H, OFFSET);
                    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                } else {
                    let _ = window.center();
                }
                emit_context_captured(app, label, &context);
                tracing::debug!(label = %label, source = "toggle_overlay", "overlay interactive ready");
                show_and_focus_overlay(&window);
                let _ = window.emit_to(label, "overlay-shown", ());
                mark_blur_guard();
            }
        }
    } else {
        let context = resolve_environment_context(app, capture_now());
        if let Some((mx, my)) = mouse_pos.filter(|_| has_selected_context(&context)) {
            place_and_show_overlay_at_mouse(app, mx, my, &context);
        } else {
            create_new_overlay(app, &context);
        }
    }
}

/// 计算 overlay 在鼠标附近的位置（物理像素），带屏幕边界保护
/// - mouse_x / mouse_y：rdev 报告的物理坐标
/// - win_w_logical / win_h_logical：窗口逻辑尺寸
/// - offset：鼠标与窗口左上角的间距（物理像素）
fn calc_position_near_mouse(
    window: &tauri::WebviewWindow,
    mouse_x: i32,
    mouse_y: i32,
    win_w_logical: f64,
    win_h_logical: f64,
    offset: i32,
) -> (i32, i32) {
    // 找包含鼠标的显示器，回退到主显示器
    let monitor = window
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            monitors.into_iter().find(|m| {
                let p = m.position();
                let s = m.size();
                mouse_x >= p.x
                    && mouse_x < p.x + s.width as i32
                    && mouse_y >= p.y
                    && mouse_y < p.y + s.height as i32
            })
        })
        .or_else(|| window.primary_monitor().ok().flatten());

    let (screen_x, screen_y, screen_w, screen_h, scale) = match monitor {
        Some(m) => {
            let p = m.position();
            let s = m.size();
            (p.x, p.y, s.width as i32, s.height as i32, m.scale_factor())
        }
        None => return (mouse_x + offset, mouse_y + offset),
    };

    let win_w_phys = (win_w_logical * scale) as i32;
    let win_h_phys = (win_h_logical * scale) as i32;

    let mut x = mouse_x + offset;
    let mut y = mouse_y + offset;

    // 右边界溢出 → 移到鼠标左侧
    if x + win_w_phys > screen_x + screen_w {
        x = mouse_x - win_w_phys - offset;
    }
    // 下边界溢出 → 移到鼠标上方
    if y + win_h_phys > screen_y + screen_h {
        y = mouse_y - win_h_phys - offset;
    }
    x = x.max(screen_x);
    y = y.max(screen_y);

    (x, y)
}

fn place_and_show_overlay_at_mouse(
    app: &AppHandle,
    mouse_x: i32,
    mouse_y: i32,
    context: &RequestContext,
) {
    const WIN_W: f64 = 640.0;
    // Keep in sync with Overlay.vue INPUT_HEIGHT (82px bar + 1px dock borders).
    const WIN_H: f64 = 84.0;
    const OFFSET: i32 = 16;

    let all_windows = app.webview_windows();
    let mut overlay_labels: Vec<String> = all_windows
        .keys()
        .filter(|label| is_overlay_label(label))
        .cloned()
        .collect();
    overlay_labels.sort();

    let input_label = overlay_labels
        .iter()
        .find(|label| is_available_input_window(app, label));

    if let Some(label) = input_label {
        if let Some(window) = app.get_webview_window(label) {
            let (x, y) = calc_position_near_mouse(&window, mouse_x, mouse_y, WIN_W, WIN_H, OFFSET);
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            let label_str = label.clone();
            let _ = window.emit_to(&label_str, "context-captured", context);
            tracing::debug!(label = %label_str, source = "toggle_overlay", "overlay interactive ready");
            show_and_focus_overlay(&window);
            let _ = window.emit_to(&label_str, "overlay-shown", ());
            mark_blur_guard();
        }
    } else {
        let label = next_overlay_label(app);
        pending_contexts().insert(label.clone(), context.clone());
        match WebviewWindowBuilder::new(app, &label, WebviewUrl::App("/#/overlay".into()))
            .title(app.package_info().name.clone())
            .inner_size(WIN_W, WIN_H)
            .min_inner_size(640.0, WIN_H)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .visible(false)
            .skip_taskbar(true)
            .center()
            .focused(false)
            .resizable(false)
            .maximizable(false)
            .build()
        {
            Ok(window) => {
                let (x, y) =
                    calc_position_near_mouse(&window, mouse_x, mouse_y, WIN_W, WIN_H, OFFSET);
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                tracing::debug!(label = %label, source = "toggle_overlay", "overlay interactive ready");
                show_and_focus_overlay(&window);
                mark_blur_guard();
            }
            Err(e) => {
                pending_contexts().remove(&label);
                eprintln!("failed to create overlay window near mouse: {e:?}");
            }
        }
    }
}

pub fn show_settings_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("workbench") else {
        tracing::error!("workbench window is missing from the application configuration");
        return;
    };

    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    let _ = window.set_always_on_top(false);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("open-workbench-settings", ());
}

pub fn show_workbench_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("workbench") else {
        tracing::error!("workbench window is missing from the application configuration");
        return;
    };

    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    let _ = window.set_always_on_top(false);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("workbench-opened", ());
}

/// Open (or create) an input overlay and attach the given images as selected context.
/// Used by pin-window AI badges (PixPin / Snipaste) and the local HTTP API.
pub fn open_overlay_with_images(app: &AppHandle, images: Vec<String>) {
    if images.is_empty() {
        return;
    }

    let mut context = resolve_environment_context(app, RequestContext::default());
    context.selected_images = images;

    let all_windows = app.webview_windows();
    let mut overlay_labels: Vec<String> = all_windows
        .keys()
        .filter(|label| is_overlay_label(label))
        .cloned()
        .collect();
    overlay_labels.sort();

    let input_label = overlay_labels
        .iter()
        .find(|label| is_available_input_window(app, label))
        .cloned();

    if let Some(label) = input_label {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.center();
            emit_context_captured(app, &label, &context);
            tracing::debug!(
                label = %label,
                image_count = context.selected_images.len(),
                source = "open_overlay_with_images",
                "overlay interactive ready"
            );
            show_and_focus_overlay(&window);
            let _ = window.emit_to(&label, "overlay-shown", ());
            mark_blur_guard();
            return;
        }
    }

    create_new_overlay(app, &context);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_uses_centered_positioning() {
        assert!(!has_selected_context(&RequestContext::default()));
    }

    #[test]
    fn text_or_files_use_selection_positioning() {
        let text = RequestContext {
            selection: Some("selected text".to_string()),
            ..RequestContext::default()
        };
        let files = RequestContext {
            selected_files: vec!["src/main.rs".to_string()],
            ..RequestContext::default()
        };
        assert!(has_selected_context(&text));
        assert!(has_selected_context(&files));
    }
}
