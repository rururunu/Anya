use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::models::settings::AppSettings;

static WANT_GLASS: AtomicBool = AtomicBool::new(false);
static DARK: AtomicBool = AtomicBool::new(false);
static COVERING: AtomicBool = AtomicBool::new(false);
static TINT: AtomicU32 = AtomicU32::new(0xEB_10_10_10);

fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some((r, g, b))
    } else if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
        Some((r, g, b))
    } else {
        None
    }
}

pub fn resolve_theme_tint(settings: &AppSettings) -> (u8, u8, u8, u8) {
    let dark = settings.is_dark_mode();
    let default_rgb = if dark {
        (16_u8, 16_u8, 16_u8)
    } else {
        (232_u8, 232_u8, 232_u8)
    };

    if let crate::models::settings::ColorScheme::Custom(id) = &settings.color_scheme {
        if let Some(custom) = settings.custom_themes.iter().find(|t| &t.id == id) {
            if let Some(hex) = custom
                .tokens
                .get("--peek-sidebar")
                .or_else(|| custom.tokens.get("--peek-bg"))
            {
                if let Some((r, g, b)) = parse_hex_color(hex) {
                    let a = if dark { 160_u8 } else { 180_u8 };
                    return (r, g, b, a);
                }
            }
        }
    }

    let a = if dark { 160_u8 } else { 180_u8 };
    (default_rgb.0, default_rgb.1, default_rgb.2, a)
}

fn is_covering_display(window: &WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false)
}

/// Apply (or clear) the workbench system backdrop so transparent CSS chrome
/// can show the desktop blur. No-op when the workbench window is missing.
///
/// Maximized / fullscreen windows skip the native blur: DWM samples the
/// window's own pixels and composites a grainy ghost of titlebar icons.
pub fn apply_from_settings(app: &AppHandle, settings: &AppSettings) {
    let Some(window) = app.get_webview_window("workbench") else {
        return;
    };
    let want = settings.chrome_frosted_glass;
    let dark = settings.is_dark_mode();
    let tint = resolve_theme_tint(settings);
    let packed = u32::from(tint.0)
        | (u32::from(tint.1) << 8)
        | (u32::from(tint.2) << 16)
        | (u32::from(tint.3) << 24);
    WANT_GLASS.store(want, Ordering::Relaxed);
    DARK.store(dark, Ordering::Relaxed);
    TINT.store(packed, Ordering::Relaxed);
    let covering = is_covering_display(&window);
    COVERING.store(covering, Ordering::Relaxed);
    apply_to_window(&window, want && !covering, dark, tint);
}

/// Re-apply after maximize / restore. Cheap no-op unless covering state changed.
pub fn sync_covering(app: &AppHandle) {
    let Some(window) = app.get_webview_window("workbench") else {
        return;
    };
    let covering = is_covering_display(&window);
    if COVERING.swap(covering, Ordering::Relaxed) == covering {
        return;
    }
    let packed = TINT.load(Ordering::Relaxed);
    let tint = (
        (packed & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 24) & 0xFF) as u8,
    );
    apply_to_window(
        &window,
        WANT_GLASS.load(Ordering::Relaxed) && !covering,
        DARK.load(Ordering::Relaxed),
        tint,
    );
}

fn apply_to_window(window: &WebviewWindow, enabled: bool, dark: bool, tint: (u8, u8, u8, u8)) {
    #[cfg(windows)]
    windows_imp::apply(window, enabled, dark, tint);

    #[cfg(not(windows))]
    {
        let _ = (window, enabled, dark, tint);
    }
}

#[cfg(windows)]
mod windows_imp {
    use std::ffi::c_void;

    use tauri::WebviewWindow;
    use windows::core::s;
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

    pub fn apply(window: &WebviewWindow, enabled: bool, dark: bool, tint: (u8, u8, u8, u8)) {
        let Ok(raw) = window.hwnd() else {
            return;
        };
        let hwnd = HWND(raw.0);
        let dark_u32 = u32::from(dark);

        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_u32 as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }

        // On Windows 11 22H2+ (build >= 22523), DWMWA_SYSTEMBACKDROP_TYPE with
        // DWMSBT_TRANSIENTWINDOW enables the modern hardware-accelerated Acrylic frosted glass blur.
        let backdrop = if enabled {
            DWMSBT_TRANSIENTWINDOW
        } else {
            DWMSBT_NONE
        };
        let backdrop_res = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop.0 as *const i32 as *const c_void,
                std::mem::size_of::<i32>() as u32,
            )
        };

        // Fall back to SetWindowCompositionAttribute (Acrylic accent 4) on Windows 10
        // or systems where SYSTEMBACKDROP_TYPE is not supported.
        // If disabling, clear SWCA accent state as well.
        if backdrop_res.is_err() || !enabled {
            set_blur(hwnd, enabled, tint);
        }
    }

    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }

    #[repr(C)]
    struct WindowCompositionAttribData {
        attrib: u32,
        pv_data: *mut c_void,
        cb_data: usize,
    }

    const ACCENT_DISABLED: u32 = 0;
    const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
    const WCA_ACCENT_POLICY: u32 = 0x13;

    type SetWindowCompositionAttributeFn =
        unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

    fn set_blur(hwnd: HWND, enabled: bool, tint: (u8, u8, u8, u8)) {
        let Some(set_attr) = set_window_composition_attribute() else {
            return;
        };
        let (r, g, b, mut a) = tint;
        if enabled && a == 0 {
            a = 1;
        }
        let mut policy = AccentPolicy {
            accent_state: if enabled {
                ACCENT_ENABLE_ACRYLICBLURBEHIND
            } else {
                ACCENT_DISABLED
            },
            accent_flags: 0,
            gradient_color: u32::from(r)
                | (u32::from(g) << 8)
                | (u32::from(b) << 16)
                | (u32::from(a) << 24),
            animation_id: 0,
        };
        let mut data = WindowCompositionAttribData {
            attrib: WCA_ACCENT_POLICY,
            pv_data: &mut policy as *mut AccentPolicy as *mut c_void,
            cb_data: std::mem::size_of::<AccentPolicy>(),
        };
        unsafe {
            let _ = set_attr(hwnd, &mut data);
        }
    }

    fn set_window_composition_attribute() -> Option<SetWindowCompositionAttributeFn> {
        unsafe {
            let module = LoadLibraryA(s!("user32.dll")).ok()?;
            let proc = GetProcAddress(module, s!("SetWindowCompositionAttribute"))?;
            Some(std::mem::transmute::<_, SetWindowCompositionAttributeFn>(
                proc,
            ))
        }
    }
}
