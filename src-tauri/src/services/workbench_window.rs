//! Workbench window size: default 2170×1210, remember the user's last resize.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

pub const DEFAULT_WIDTH: f64 = 2170.0;
pub const DEFAULT_HEIGHT: f64 = 1210.0;
const MIN_WIDTH: f64 = 960.0;
const MIN_HEIGHT: f64 = 640.0;
const STATE_FILE: &str = "workbench-window.json";
const SAVE_DEBOUNCE_MS: u64 = 300;

static RESTORED: AtomicBool = AtomicBool::new(false);
static SAVE_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct WorkbenchWindowState {
    width: f64,
    height: f64,
    #[serde(default)]
    maximized: bool,
}

impl Default for WorkbenchWindowState {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            maximized: false,
        }
    }
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(STATE_FILE))
}

fn load_state(app: &AppHandle) -> WorkbenchWindowState {
    let Some(path) = state_path(app) else {
        return WorkbenchWindowState::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_state(app: &AppHandle, state: &WorkbenchWindowState) {
    let Some(path) = state_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, raw);
    }
}

fn clamp_logical_size(width: f64, height: f64, max_width: f64, max_height: f64) -> (f64, f64) {
    (
        width.round().clamp(MIN_WIDTH, max_width.max(MIN_WIDTH)),
        height.round().clamp(MIN_HEIGHT, max_height.max(MIN_HEIGHT)),
    )
}

fn monitor_max_logical(window: &WebviewWindow) -> (f64, f64) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
    };
    let scale = monitor.scale_factor().max(0.1);
    let area = monitor.work_area();
    (
        (area.size.width as f64 / scale).max(MIN_WIDTH),
        (area.size.height as f64 / scale).max(MIN_HEIGHT),
    )
}

/// Apply the last saved (or default) workbench size before the first show.
pub fn apply_saved_workbench_size(window: &WebviewWindow) {
    if RESTORED.swap(true, Ordering::SeqCst) {
        return;
    }
    let state = load_state(window.app_handle());
    let (max_w, max_h) = monitor_max_logical(window);
    let (width, height) = clamp_logical_size(state.width, state.height, max_w, max_h);
    let _ = window.set_size(LogicalSize::new(width, height));
    if state.maximized {
        let _ = window.maximize();
    }
}

/// Persist the current workbench size for the next launch.
pub fn remember_workbench_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("workbench") else {
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let mut state = load_state(app);
    if !maximized {
        if let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) {
            let scale = scale.max(0.1);
            state.width = size.width as f64 / scale;
            state.height = size.height as f64 / scale;
        }
    }
    state.maximized = maximized;
    save_state(app, &state);
}

/// Debounce disk writes while the user is dragging a resize handle.
pub fn schedule_remember(app: &AppHandle) {
    let gen = SAVE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(SAVE_DEBOUNCE_MS)).await;
        if SAVE_GENERATION.load(Ordering::Relaxed) != gen {
            return;
        }
        remember_workbench_window(&app);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_requested_size_on_large_displays() {
        let (width, height) = clamp_logical_size(2170.0, 1210.0, 3840.0, 2160.0);
        assert_eq!((width, height), (2170.0, 1210.0));
    }

    #[test]
    fn clamp_fits_small_work_area() {
        let (width, height) = clamp_logical_size(2170.0, 1210.0, 1920.0, 1080.0);
        assert_eq!((width, height), (1920.0, 1080.0));
    }

    #[test]
    fn default_state_matches_config() {
        let state = WorkbenchWindowState::default();
        assert_eq!(state.width, DEFAULT_WIDTH);
        assert_eq!(state.height, DEFAULT_HEIGHT);
        assert!(!state.maximized);
    }
}
