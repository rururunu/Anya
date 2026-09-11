//! Cheap post-action UIA probe — no JPEG recapture.

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

use super::uia::{control_kind, with_uia};
use super::windows::{foreground_hwnd, window_title};
use crate::core::tools::error::ToolError;

pub(crate) fn after_pointer(screen_x: i32, screen_y: i32) -> String {
    let fg = foreground_hwnd()
        .map(window_title)
        .unwrap_or_default();
    let under = with_uia(|automation| {
        let el: IUIAutomationElement = unsafe {
            automation
                .ElementFromPoint(POINT {
                    x: screen_x,
                    y: screen_y,
                })
        }
        .map_err(|e| ToolError::new(format!("ElementFromPoint: {e}")))?;
        Ok(describe_element(&el))
    })
    .unwrap_or_else(|_| "no UIA element".into());
    let warn = if fg.to_ascii_lowercase().contains("anya") {
        " WARNING: foreground looks like Anya — focus_window the target app."
    } else {
        ""
    };
    if fg.is_empty() {
        format!("{under}{warn}")
    } else {
        format!("foreground=`{fg}`; {under}{warn}")
    }
}

fn describe_element(element: &IUIAutomationElement) -> String {
    let name = unsafe { element.CurrentName() }
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unnamed".into());
    let kind = unsafe { element.CurrentControlType() }
        .ok()
        .map(control_kind)
        .unwrap_or("other");
    let aid = unsafe { element.CurrentAutomationId() }
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    match aid {
        Some(id) => format!("{kind} \"{name}\" aid={id}"),
        None => format!("{kind} \"{name}\""),
    }
}
