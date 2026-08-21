use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::models::settings::{AppSettings, ColorScheme};

static WANT_GLASS: AtomicBool = AtomicBool::new(false);
static DARK: AtomicBool = AtomicBool::new(false);
static COVERING: AtomicBool = AtomicBool::new(false);

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
    let dark = settings.color_scheme == ColorScheme::Dark;
    WANT_GLASS.store(want, Ordering::Relaxed);
    DARK.store(dark, Ordering::Relaxed);
    let covering = is_covering_display(&window);
    COVERING.store(covering, Ordering::Relaxed);
    apply_to_window(&window, want && !covering, dark);
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
    apply_to_window(
        &window,
        WANT_GLASS.load(Ordering::Relaxed) && !covering,
        DARK.load(Ordering::Relaxed),
    );
}

fn apply_to_window(window: &WebviewWindow, enabled: bool, dark: bool) {
    #[cfg(windows)]
    windows_imp::apply(window, enabled, dark);

    #[cfg(not(windows))]
    {
        let _ = (window, enabled, dark);
    }
}

#[cfg(windows)]
mod windows_imp {
    use std::ffi::c_void;

    use tauri::WebviewWindow;
    use windows::core::s;
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    use windows::Win32::UI::WindowsAndMessaging::{GetWindow, GW_CHILD, GW_HWNDNEXT};

    #[repr(C)]
    struct Margins {
        cx_left_width: i32,
        cx_right_width: i32,
        cy_top_height: i32,
        cy_bottom_height: i32,
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
    /// Win10/11 blur without Acrylic's noise grain.
    const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
    const WCA_ACCENT_POLICY: u32 = 0x13;
    const ACCENT_FLAG_DRAW_ALL_BORDERS: u32 = 0x20 | 0x40 | 0x80 | 0x100;

    type SetWindowCompositionAttributeFn =
        unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: HWND, margins: *const Margins) -> i32;
    }

    pub fn apply(window: &WebviewWindow, enabled: bool, dark: bool) {
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

        let margins = if enabled {
            Margins {
                cx_left_width: -1,
                cx_right_width: -1,
                cy_top_height: -1,
                cy_bottom_height: -1,
            }
        } else {
            Margins {
                cx_left_width: 0,
                cx_right_width: 0,
                cy_top_height: 0,
                cy_bottom_height: 0,
            }
        };
        unsafe {
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        }

        // Win11 SYSTEMBACKDROP (Mica) is too subtle behind WebView2.
        // Acrylic (accent 4) adds a heavy noise texture; BlurBehind is the same
        // desktop blur without that grain.
        let tint = if dark {
            (18_u8, 18_u8, 20_u8, 236_u8)
        } else {
            (232_u8, 232_u8, 232_u8, 232_u8)
        };
        visit_hwnds(hwnd, |child| {
            set_blur(child, enabled, tint);
        });
    }

    fn visit_hwnds(root: HWND, mut visit: impl FnMut(HWND)) {
        visit(root);
        let Ok(mut child) = (unsafe { GetWindow(root, GW_CHILD) }) else {
            return;
        };
        while !child.0.is_null() {
            visit(child);
            match unsafe { GetWindow(child, GW_HWNDNEXT) } {
                Ok(next) => child = next,
                Err(_) => break,
            }
        }
    }

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
                ACCENT_ENABLE_BLURBEHIND
            } else {
                ACCENT_DISABLED
            },
            accent_flags: if enabled {
                ACCENT_FLAG_DRAW_ALL_BORDERS
            } else {
                0
            },
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
