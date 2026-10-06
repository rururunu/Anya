use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::core::context::models::{CaptureError, WindowInfo};

pub struct WindowDetector;

impl WindowDetector {
    pub fn detect() -> Result<WindowInfo, CaptureError> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return Err(CaptureError::WindowDetection("no foreground window".into()));
            }

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return Err(CaptureError::WindowDetection(
                    "failed to read process id".into(),
                ));
            }

            let process_name = read_process_name(pid)?;
            let title = read_window_title(hwnd)?;

            Ok(WindowInfo {
                hwnd: hwnd.0 as isize,
                pid,
                process_name,
                title,
            })
        }
    }
}

unsafe fn read_process_name(pid: u32) -> Result<String, CaptureError> {
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
        .map_err(|error| CaptureError::WindowDetection(format!("OpenProcess failed: {error}")))?;

    let mut buffer = [0u16; 32768];
    let mut len = buffer.len() as u32;
    let result = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        windows::core::PWSTR(buffer.as_mut_ptr()),
        &mut len,
    );
    let _ = CloseHandle(process);
    if result.is_err() || len == 0 {
        return Err(CaptureError::WindowDetection(
            "could not query foreground process image".into(),
        ));
    }

    let path = String::from_utf16_lossy(&buffer[..len as usize]);
    Ok(path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string())
}

unsafe fn read_window_title(hwnd: HWND) -> Result<String, CaptureError> {
    let mut buffer = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buffer);
    if len == 0 {
        return Ok(String::new());
    }

    Ok(String::from_utf16_lossy(&buffer[..len as usize]))
}
