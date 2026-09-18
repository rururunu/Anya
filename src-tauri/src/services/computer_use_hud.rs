//! Session-scoped Computer Use HUD: open once, stay up, close explicitly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const COMPUTER_USE_HUD_LABEL: &str = "computer-use-hud";
pub const COMPUTER_USE_BANNER_LABEL: &str = "computer-use-banner";

const BANNER_WIDTH: u32 = 440;
const BANNER_HEIGHT: u32 = 56;
const BANNER_TOP_INSET: f64 = 44.0;

static APP: OnceLock<AppHandle> = OnceLock::new();
static OPEN: AtomicBool = AtomicBool::new(false);
static CAPTURE_SUSPENDED: AtomicBool = AtomicBool::new(false);

/// Remember the app handle so computer tools can toggle the HUD without plumbing AppHandle.
pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

fn app_handle() -> Option<&'static AppHandle> {
    APP.get()
}

/// Open the glow + banner. Idempotent — subsequent calls are no-ops while open.
pub fn open() {
    if OPEN.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(app) = app_handle().cloned() else {
        OPEN.store(false, Ordering::SeqCst);
        return;
    };
    let _ = app.clone().run_on_main_thread(move || show_windows(&app));
}

/// Close the glow + banner (chat finished / user stop). Idempotent.
pub fn close() {
    if !OPEN.swap(false, Ordering::SeqCst) {
        CAPTURE_SUSPENDED.store(false, Ordering::SeqCst);
        return;
    }
    CAPTURE_SUSPENDED.store(false, Ordering::SeqCst);
    let Some(app) = app_handle().cloned() else {
        return;
    };
    let _ = app.clone().run_on_main_thread(move || hide_windows(&app));
}

/// Alias used by chat lifecycle and the stop button.
pub fn force_dismiss() {
    crate::core::plugins::stop_computer_engine();
    close();
}

/// Temporarily cloak the HUD so desktop screenshots do not include the glow.
pub struct CaptureSuspend {
    owned: bool,
}

impl CaptureSuspend {
    pub fn enter() -> Self {
        if CAPTURE_SUSPENDED.swap(true, Ordering::SeqCst) {
            return Self { owned: false };
        }
        if let Some(app) = app_handle() {
            cloak_label(app, COMPUTER_USE_HUD_LABEL, true);
            cloak_label(app, COMPUTER_USE_BANNER_LABEL, true);
        }
        Self { owned: true }
    }
}

impl Drop for CaptureSuspend {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        CAPTURE_SUSPENDED.store(false, Ordering::SeqCst);
        if !OPEN.load(Ordering::SeqCst) {
            return;
        }
        if let Some(app) = app_handle() {
            cloak_label(app, COMPUTER_USE_HUD_LABEL, false);
            cloak_label(app, COMPUTER_USE_BANNER_LABEL, false);
        }
    }
}

fn show_windows(app: &AppHandle) {
    if let Some(glow) = app.get_webview_window(COMPUTER_USE_HUD_LABEL) {
        configure_glow(&glow);
        fit_virtual_screen(&glow);
        show_no_activate(&glow);
    } else {
        tracing::warn!("computer-use-hud window is missing from application config");
    }

    if let Some(banner) = app.get_webview_window(COMPUTER_USE_BANNER_LABEL) {
        configure_banner(&banner);
        place_banner(&banner);
        if let Ok(settings) = crate::services::settings_store::get_settings(app) {
            crate::services::webview_theme::apply_webview_theme(app, &settings);
        }
        show_no_activate(&banner);
    } else {
        tracing::warn!("computer-use-banner window is missing from application config");
    }

    let _ = app.emit("computer-use-hud-visibility-changed", true);
}

fn hide_windows(app: &AppHandle) {
    for label in [COMPUTER_USE_HUD_LABEL, COMPUTER_USE_BANNER_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            hide_window(&window);
        }
    }
    let _ = app.emit("computer-use-hud-visibility-changed", false);
}

fn configure_glow(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    let _ = window.set_ignore_cursor_events(true);
    crate::services::overlay_native::reapply_toolwindow_style(window);
    let _ = window.set_shadow(false);
}

fn configure_banner(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    let _ = window.set_ignore_cursor_events(false);
    crate::services::overlay_native::reapply_toolwindow_style(window);
    let _ = window.set_shadow(false);
}

fn fit_virtual_screen(window: &WebviewWindow) {
    let Some((x, y, w, h)) = virtual_screen() else {
        return;
    };
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
}

fn place_banner(window: &WebviewWindow) {
    let Some((vx, vy, vw, _vh)) = virtual_screen() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = ((BANNER_WIDTH as f64) * scale).round() as u32;
    let height = ((BANNER_HEIGHT as f64) * scale).round() as u32;
    let x = vx + ((vw - width as i32) / 2).max(0);
    let y = vy + ((BANNER_TOP_INSET * scale).round() as i32).max(24);
    let _ = window.set_size(PhysicalSize::new(width, height));
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn virtual_screen() -> Option<(i32, i32, i32, i32)> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        let (x, y, w, h) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        if w > 0 && h > 0 {
            Some((x, y, w, h))
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn show_no_activate(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};

        set_cloaked(window, true);
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0), SW_SHOWNOACTIVATE);
            }
        } else {
            let _ = window.show();
        }
        set_cloaked(window, false);
    }
    #[cfg(not(windows))]
    {
        let _ = window.show();
    }
}

fn hide_window(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

        set_cloaked(window, true);
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0), SW_HIDE);
            }
        }
        let _ = window.hide();
        set_cloaked(window, false);
    }
    #[cfg(not(windows))]
    {
        let _ = window.hide();
    }
}

fn cloak_label(app: &AppHandle, label: &str, cloaked: bool) {
    if let Some(window) = app.get_webview_window(label) {
        set_cloaked(&window, cloaked);
    }
}

fn set_cloaked(window: &WebviewWindow, cloaked: bool) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{BOOL, HWND};
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};

        let Ok(hwnd) = window.hwnd() else {
            return;
        };
        let value = BOOL(i32::from(cloaked));
        unsafe {
            let _ = DwmSetWindowAttribute(
                HWND(hwnd.0),
                DWMWA_CLOAK,
                &value as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&value) as u32,
            );
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, cloaked);
    }
}
