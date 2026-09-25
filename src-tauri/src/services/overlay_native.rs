use std::cell::Cell;

thread_local! {
    static OVERLAY_VISIBILITY_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Win32 `ShowWindow`/`hide` synthesizes `Focused` before `is_visible` updates.
/// Nested hide/show from that event overflows the UI thread stack.
struct OverlayVisibilityGuard;

impl OverlayVisibilityGuard {
    fn enter() -> Option<Self> {
        OVERLAY_VISIBILITY_DEPTH.with(|depth| {
            if depth.get() > 0 {
                return None;
            }
            depth.set(1);
            Some(Self)
        })
    }
}

impl Drop for OverlayVisibilityGuard {
    fn drop(&mut self) {
        OVERLAY_VISIBILITY_DEPTH.with(|depth| depth.set(0));
    }
}

#[cfg(windows)]
mod imp {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};

    use super::OverlayVisibilityGuard;

    use tauri::WebviewWindow;

    pub fn set_overlay_bounds(
        window: &WebviewWindow,
        position: tauri::PhysicalPosition<i32>,
        size: tauri::PhysicalSize<u32>,
    ) -> Result<(), String> {
        let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
                SWP_NOACTIVATE | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
            .map_err(|e| e.to_string())
        }
    }
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, IsIconic, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SW_MINIMIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };

    static OVERLAY_NATIVE_MINIMIZED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    static OVERLAY_MINIMIZE_PENDING: AtomicBool = AtomicBool::new(false);

    fn minimized_labels() -> std::sync::MutexGuard<'static, HashSet<String>> {
        OVERLAY_NATIVE_MINIMIZED
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn mark_overlay_native_minimized(label: &str) {
        minimized_labels().insert(label.to_string());
    }

    pub fn is_overlay_native_minimized(label: &str) -> bool {
        minimized_labels().contains(label)
    }

    pub fn clear_overlay_native_minimized(label: &str) {
        minimized_labels().remove(label);
    }

    pub fn mark_minimize_pending() {
        OVERLAY_MINIMIZE_PENDING.store(true, Ordering::Release);
    }

    pub fn clear_minimize_pending() {
        OVERLAY_MINIMIZE_PENDING.store(false, Ordering::Release);
    }

    pub fn is_minimize_pending() -> bool {
        OVERLAY_MINIMIZE_PENDING.load(Ordering::Acquire)
    }

    fn local_hwnd(window: &WebviewWindow) -> Result<HWND, String> {
        let raw = window.hwnd().map_err(|error| error.to_string())?.0;
        Ok(HWND(raw))
    }

    pub fn minimize_window(window: &WebviewWindow) -> Result<(), String> {
        let hwnd = local_hwnd(window)?;
        unsafe { minimize_hwnd(hwnd) }
    }

    pub fn reapply_toolwindow_style(window: &WebviewWindow) {
        let Ok(hwnd) = local_hwnd(window) else {
            return;
        };
        unsafe {
            apply_toolwindow_style(hwnd);
        }
    }

    /// Show without a Win32/DWM flash: cloak → show → uncloak.
    ///
    /// Do **not** set window `background_color` to (0,0,0,0) on Windows: the
    /// window-layer alpha is ignored, so it becomes an opaque black rectangle
    /// that leaks past CSS `border-radius` as corner triangles.
    ///
    /// Do **not** force `DWMWA_NCRENDERING_POLICY` or `DWMWA_TRANSITIONS_FORCEDISABLED`
    /// either: both make DWM take a different composition path for this
    /// layered/transparent window that stops alpha-blending the rounded-corner
    /// edge against the desktop, painting a solid black ring around the content
    /// instead of a clean transparent edge. Cloak/uncloak alone is enough to
    /// hide the default Win32 frame during the show transition.
    pub fn show_overlay_without_flash(window: &WebviewWindow) -> Result<(), String> {
        let Some(_guard) = OverlayVisibilityGuard::enter() else {
            return Ok(());
        };
        let hwnd = local_hwnd(window)?;
        unsafe {
            apply_toolwindow_style(hwnd);
            set_cloaked(hwnd, true);
        }
        window.show().map_err(|error| error.to_string())?;
        let _ = window.set_focus();
        unsafe {
            set_cloaked(hwnd, false);
        }
        Ok(())
    }

    /// Hide without DWM close animation flash.
    pub fn hide_overlay_without_flash(window: &WebviewWindow) -> Result<(), String> {
        let Some(_guard) = OverlayVisibilityGuard::enter() else {
            return Ok(());
        };
        let hwnd = local_hwnd(window)?;
        unsafe {
            set_cloaked(hwnd, true);
        }
        let result = window.hide().map_err(|error| error.to_string());
        unsafe {
            // Keep cloaked=false while hidden so state stays clean for next show.
            set_cloaked(hwnd, false);
        }
        result
    }

    unsafe fn set_cloaked(hwnd: HWND, cloaked: bool) {
        let value = BOOL(i32::from(cloaked));
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            &value as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&value) as u32,
        );
    }

    unsafe fn minimize_hwnd(hwnd: HWND) -> Result<(), String> {
        if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return Ok(());
        }

        // Tool windows minimize to nowhere; register as a normal app window first.
        apply_appwindow_style(hwnd);
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
        Ok(())
    }

    unsafe fn apply_appwindow_style(hwnd: HWND) {
        let _ = SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next_style = (ex_style | WS_EX_APPWINDOW.0) & !WS_EX_TOOLWINDOW.0;
        if ex_style != next_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize);
            let _ = SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }

    unsafe fn apply_toolwindow_style(hwnd: HWND) {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next_style = (ex_style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        let style_changed = ex_style != next_style;
        if style_changed {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize);
        }

        let flags = if style_changed {
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED
        } else {
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
        };
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
mod imp {
    use tauri::WebviewWindow;

    pub fn set_overlay_bounds(
        window: &WebviewWindow,
        position: tauri::PhysicalPosition<i32>,
        size: tauri::PhysicalSize<u32>,
    ) -> Result<(), String> {
        window.set_position(position).map_err(|e| e.to_string())?;
        window.set_size(size).map_err(|e| e.to_string())
    }

    pub fn mark_overlay_native_minimized(_label: &str) {}
    pub fn is_overlay_native_minimized(_label: &str) -> bool {
        false
    }
    pub fn clear_overlay_native_minimized(_label: &str) {}
    pub fn mark_minimize_pending() {}
    pub fn clear_minimize_pending() {}
    pub fn is_minimize_pending() -> bool {
        false
    }
    pub fn reapply_toolwindow_style(_window: &WebviewWindow) {}

    pub fn show_overlay_without_flash(window: &WebviewWindow) -> Result<(), String> {
        let Some(_guard) = super::OverlayVisibilityGuard::enter() else {
            return Ok(());
        };
        window.show().map_err(|error| error.to_string())?;
        let _ = window.set_focus();
        Ok(())
    }

    pub fn hide_overlay_without_flash(window: &WebviewWindow) -> Result<(), String> {
        let Some(_guard) = super::OverlayVisibilityGuard::enter() else {
            return Ok(());
        };
        window.hide().map_err(|error| error.to_string())
    }

    pub fn minimize_window(window: &WebviewWindow) -> Result<(), String> {
        window.minimize().map_err(|error| error.to_string())
    }
}

#[cfg(not(windows))]
pub use imp::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_guard_rejects_reentry() {
        let first = OverlayVisibilityGuard::enter().expect("first enter");
        assert!(OverlayVisibilityGuard::enter().is_none());
        drop(first);
        assert!(OverlayVisibilityGuard::enter().is_some());
    }
}
