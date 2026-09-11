//! Top-level window list and focus.

use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, POINT, TRUE};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetAncestor, GetForegroundWindow, GetSystemMetrics,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
    SetForegroundWindow, ShowWindow, WindowFromPoint, GA_ROOT, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_RESTORE,
};

use crate::core::tools::error::ToolError;

#[link(name = "user32")]
extern "system" {
    fn ReleaseCapture() -> i32;
}

pub(crate) fn virtual_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

pub(crate) fn hwnd_id(hwnd: HWND) -> String {
    format!("{:x}", hwnd.0 as usize)
}

pub(crate) fn parse_hwnd_id(id: &str) -> Option<HWND> {
    let raw = id.trim().trim_start_matches("0x").trim_start_matches("0X");
    usize::from_str_radix(raw, 16)
        .ok()
        .filter(|n| *n != 0)
        .map(|n| HWND(n as *mut core::ffi::c_void))
}

pub(crate) fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

struct EnumState {
    out: Vec<(HWND, String)>,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam.0 as *mut EnumState) };
    if unsafe { !IsWindowVisible(hwnd).as_bool() } {
        return TRUE;
    }
    let title = window_title(hwnd);
    if !title.is_empty() {
        state.out.push((hwnd, title));
    }
    TRUE
}

pub(crate) fn enumerate_windows() -> Vec<(HWND, String)> {
    let mut state = EnumState { out: Vec::new() };
    let _ = unsafe { EnumWindows(Some(enum_cb), LPARAM(&mut state as *mut _ as isize)) };
    state.out
}

pub(crate) fn find_hwnd(title: Option<&str>, id: Option<&str>) -> Result<HWND, ToolError> {
    if let Some(id) = id.map(str::trim).filter(|s| !s.is_empty()) {
        let hwnd = parse_hwnd_id(id).ok_or_else(|| ToolError::new("invalid window id"))?;
        if unsafe { IsWindow(hwnd).as_bool() } {
            return Ok(hwnd);
        }
        return Err(ToolError::new("window id is not a live window"));
    }
    let needle = title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::new("focus_window needs title or id"))?
        .to_lowercase();
    let windows = enumerate_windows();
    windows
        .into_iter()
        .find(|(_, name)| name.to_lowercase().contains(&needle))
        .map(|(hwnd, _)| hwnd)
        .ok_or_else(|| ToolError::new(format!("no visible window matching `{needle}`")))
}

pub(crate) fn foreground_hwnd() -> Result<HWND, ToolError> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Err(ToolError::new("no foreground window"));
    }
    Ok(hwnd)
}

pub(crate) fn restore_window(hwnd: HWND) {
    if unsafe { IsIconic(hwnd).as_bool() } {
        let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
    }
}

pub(crate) fn list_windows() -> Result<String, ToolError> {
    let rows = enumerate_windows();
    if rows.is_empty() {
        return Ok("No visible titled windows.".into());
    }
    let lines: Vec<String> = rows
        .into_iter()
        .take(40)
        .map(|(hwnd, title)| format!("id={}  {title}", hwnd_id(hwnd)))
        .collect();
    Ok(lines.join("\n"))
}

pub(crate) fn focus_window(title: Option<&str>, id: Option<&str>) -> Result<String, ToolError> {
    let hwnd = find_hwnd(title, id)?;
    restore_window(hwnd);
    force_foreground(hwnd);
    Ok(format!(
        "focused `{}` id={}",
        window_title(hwnd),
        hwnd_id(hwnd)
    ))
}

/// Activate the top-level window under a screen point so the following click
/// is not swallowed by an unfocused target (or leftover WebView capture).
pub(crate) fn prepare_click_target(x: i32, y: i32) {
    unsafe {
        let _ = ReleaseCapture();
    }
    let hwnd = unsafe { WindowFromPoint(POINT { x, y }) };
    if hwnd.is_invalid() {
        return;
    }
    let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
    let target = if root.is_invalid() { hwnd } else { root };
    restore_window(target);
    force_foreground(target);
}

fn force_foreground(hwnd: HWND) {
    unsafe {
        let fg = GetForegroundWindow();
        let fg_tid = GetWindowThreadProcessId(fg, None);
        let target_tid = GetWindowThreadProcessId(hwnd, None);
        let self_tid = GetCurrentThreadId();
        let attach_fg = fg_tid != 0 && fg_tid != self_tid;
        let attach_target = target_tid != 0 && target_tid != self_tid && target_tid != fg_tid;
        if attach_fg {
            let _ = AttachThreadInput(self_tid, fg_tid, TRUE);
        }
        if attach_target {
            let _ = AttachThreadInput(self_tid, target_tid, TRUE);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attach_target {
            let _ = AttachThreadInput(self_tid, target_tid, FALSE);
        }
        if attach_fg {
            let _ = AttachThreadInput(self_tid, fg_tid, FALSE);
        }
    }
}
