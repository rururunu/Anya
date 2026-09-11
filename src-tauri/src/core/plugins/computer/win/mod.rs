//! Windows capture, input, window list, and UI Automation for computer use.

mod capture;
mod input;
mod launch;
mod observe;
mod snapshot;
mod uia;
mod windows;

pub(super) use capture::{capture_to_plugin, CaptureTarget};
pub(super) use input::{hotkey, mouse_click, mouse_drag, mouse_move, mouse_scroll_at, type_text};
pub(super) use launch::launch;
pub(super) use observe::after_pointer;
pub(super) use snapshot::overlay_for_hwnd;
pub(super) use uia::{click_control, find_control, set_value};
pub(super) use windows::{focus_window, list_windows, virtual_screen};

