//! Open an app, file, or URI via ShellExecute — Level 1, no mouse.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::core::tools::error::ToolError;

const MAX_TARGET: usize = 512;
const MAX_ARGS: usize = 1024;

pub(crate) fn launch(target: &str, args: Option<&str>) -> Result<String, ToolError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(ToolError::new("launch needs target (exe, file, or URI)"));
    }
    if target.chars().count() > MAX_TARGET || target.contains('\0') {
        return Err(ToolError::new("launch target is invalid"));
    }
    if let Some(args) = args {
        if args.chars().count() > MAX_ARGS || args.contains('\0') {
            return Err(ToolError::new("launch args are invalid"));
        }
    }
    let file = wide(target);
    let params = args.map(str::trim).filter(|s| !s.is_empty()).map(wide);
    let code = unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR(file.as_ptr()),
            params
                .as_ref()
                .map(|p| PCWSTR(p.as_ptr()))
                .unwrap_or(PCWSTR::null()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let n = code.0 as isize;
    if n <= 32 {
        return Err(ToolError::new(format!(
            "ShellExecute failed (code {n}) for `{target}`"
        )));
    }
    match args.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => Ok(format!("launched `{target}` {a}")),
        None => Ok(format!("launched `{target}`")),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_nul() {
        assert!(launch("", None).is_err());
        assert!(launch("mspaint\0", None).is_err());
        assert!(launch("mspaint", Some("a\0b")).is_err());
    }
}
