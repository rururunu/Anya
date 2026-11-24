use tauri::{AppHandle, Manager};

use crate::models::settings::AppSettings;

/// Enables the wallpaper-tinted system backdrop on supported Windows versions.
pub fn apply_from_settings(app: &AppHandle, settings: &AppSettings) -> bool {
    #[cfg(windows)]
    {
        let Some(window) = app.get_webview_window("workbench") else {
            return false;
        };
        let Ok(raw) = window.hwnd() else {
            return false;
        };
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_MAINWINDOW,
            DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        };
        use windows::Win32::UI::Controls::MARGINS;

        let hwnd = HWND(raw.0);
        let dark = i32::from(settings.is_dark_mode());
        let backdrop = DWMSBT_MAINWINDOW.0;
        let margins = MARGINS { cxLeftWidth: -1, ..Default::default() };
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                std::ptr::from_ref(&dark).cast(),
                std::mem::size_of_val(&dark) as u32,
            );
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                std::ptr::from_ref(&backdrop).cast(),
                std::mem::size_of_val(&backdrop) as u32,
            )
            .and_then(|()| DwmExtendFrameIntoClientArea(hwnd, &margins))
            .is_ok()
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (app, settings);
        false
    }
}
