//! SendInput mouse and keyboard.

use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK,
    MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN,
    VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

use super::windows::{prepare_click_target, virtual_screen};
use crate::core::tools::error::ToolError;

const MOVE_SETTLE_MS: u64 = 20;
const CLICK_HOLD_MS: u64 = 20;
const MULTI_CLICK_MS: u64 = 50;
const DRAG_STEP_PX: i32 = 6;
const DRAG_MAX_STEPS: usize = 400;
const DRAG_PRESS_MS: u64 = 25;
const DRAG_BATCH: usize = 16;

pub(crate) fn mouse_move(x: i32, y: i32) -> Result<(), ToolError> {
    let _ = unsafe { SetCursorPos(x, y) };
    send_mouse(abs_move_input(x, y))
}

pub(crate) fn mouse_click(
    x: i32,
    y: i32,
    button: &str,
    count: u32,
    modifiers: &[String],
) -> Result<(), ToolError> {
    with_modifiers(modifiers, || {
        let (down, up) = button_flags(button);
        prepare_click_target(x, y);
        mouse_move(x, y)?;
        std::thread::sleep(Duration::from_millis(MOVE_SETTLE_MS));
        mouse_move(x, y)?;
        let (dx, dy) = abs_mouse(x, y);
        for i in 0..count {
            send_mouse(abs_button_input(dx, dy, down))?;
            std::thread::sleep(Duration::from_millis(CLICK_HOLD_MS));
            send_mouse(abs_button_input(dx, dy, up))?;
            if i + 1 < count {
                std::thread::sleep(Duration::from_millis(MULTI_CLICK_MS));
            }
        }
        Ok(())
    })
}

/// Hold the button from the first point through interpolated segments, then release.
///
/// While the button is down we only SendInput MOVE — `SetCursorPos` injects a
/// move *without* MK_LBUTTON and paint apps cancel the stroke.
pub(crate) fn mouse_drag(
    points: &[(i32, i32)],
    button: &str,
    modifiers: &[String],
) -> Result<(), ToolError> {
    if points.len() < 2 {
        return Err(ToolError::new("drag needs at least two points"));
    }
    let track = expand_drag_track(points);
    with_modifiers(modifiers, || {
        let (down, up) = button_flags(button);
        let (sx, sy) = track[0];
        prepare_click_target(sx, sy);
        mouse_move(sx, sy)?;
        std::thread::sleep(Duration::from_millis(MOVE_SETTLE_MS));
        mouse_move(sx, sy)?;
        send_mouse(button_input(down))?;
        std::thread::sleep(Duration::from_millis(DRAG_PRESS_MS));
        send_mouse(abs_move_input(sx, sy))?;
        let move_result = drag_along(&track);
        let (ex, ey) = *track.last().unwrap();
        send_mouse(abs_move_input(ex, ey))?;
        std::thread::sleep(Duration::from_millis(CLICK_HOLD_MS));
        let up_result = send_mouse(button_input(up));
        move_result.and(up_result)
    })
}

fn drag_along(points: &[(i32, i32)]) -> Result<(), ToolError> {
    let mut batch = Vec::new();
    for pair in points.windows(2) {
        for (x, y) in interpolate_segment(pair[0], pair[1]) {
            batch.push(abs_move_input(x, y));
            if batch.len() >= DRAG_BATCH {
                send_inputs(&mut batch)?;
                batch.clear();
            }
        }
    }
    send_inputs(&mut batch)
}

fn button_flags(button: &str) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS) {
    match button {
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    }
}

pub(crate) fn mouse_scroll_at(x: i32, y: i32, dx: i64, dy: i64) -> Result<(), ToolError> {
    prepare_click_target(x, y);
    mouse_move(x, y)?;
    std::thread::sleep(Duration::from_millis(MOVE_SETTLE_MS));
    mouse_move(x, y)?;
    mouse_scroll(dx, dy)
}

fn mouse_scroll(dx: i64, dy: i64) -> Result<(), ToolError> {
    if dy != 0 {
        send_mouse(wheel_input(MOUSEEVENTF_WHEEL, dy))?;
    }
    if dx != 0 {
        send_mouse(wheel_input(MOUSEEVENTF_HWHEEL, dx))?;
    }
    Ok(())
}

pub(crate) fn type_text(text: &str) -> Result<(), ToolError> {
    let mut inputs = Vec::new();
    for ch in text.encode_utf16() {
        if ch == b'\n' as u16 || ch == b'\r' as u16 {
            send_inputs(&mut inputs)?;
            inputs.clear();
            hotkey("enter")?;
            continue;
        }
        inputs.push(unicode_key(ch, false));
        inputs.push(unicode_key(ch, true));
    }
    send_inputs(&mut inputs)
}

pub(crate) fn hotkey(spec: &str) -> Result<(), ToolError> {
    let parts: Vec<String> = spec
        .split('+')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(ToolError::new("empty key spec"));
    }
    let vks: Vec<VIRTUAL_KEY> = parts.iter().map(|p| vk_of(p)).collect::<Result<_, _>>()?;
    let mut inputs = Vec::new();
    for vk in &vks {
        inputs.push(vk_input(*vk, false));
    }
    for vk in vks.iter().rev() {
        inputs.push(vk_input(*vk, true));
    }
    send_inputs(&mut inputs)
}

fn vk_of(name: &str) -> Result<VIRTUAL_KEY, ToolError> {
    Ok(match name {
        "ctrl" | "control" => VK_CONTROL,
        "alt" | "option" => VK_MENU,
        "shift" => VK_SHIFT,
        "win" | "windows" | "meta" | "super" | "cmd" => VK_LWIN,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "esc" | "escape" => VK_ESCAPE,
        "backspace" => VK_BACK,
        "delete" | "del" => VK_DELETE,
        "space" => VK_SPACE,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        other if other.len() == 1 => {
            let ch = other.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() {
                VIRTUAL_KEY(ch.to_ascii_uppercase() as u16)
            } else {
                return Err(ToolError::new(format!("unsupported key `{other}`")));
            }
        }
        other if other.starts_with('f') && other.len() <= 3 => {
            let n: u16 = other[1..].parse().unwrap_or(0);
            if (1..=12).contains(&n) {
                VIRTUAL_KEY(0x6F + n)
            } else {
                return Err(ToolError::new(format!("unsupported key `{other}`")));
            }
        }
        other => return Err(ToolError::new(format!("unsupported key `{other}`"))),
    })
}

fn unicode_key(ch: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: Default::default(),
                wScan: ch,
                dwFlags: if up {
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_UNICODE
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn vk_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = if up {
        KEYEVENTF_KEYUP
    } else {
        Default::default()
    };
    if vk == VK_LWIN {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn with_modifiers(
    names: &[String],
    body: impl FnOnce() -> Result<(), ToolError>,
) -> Result<(), ToolError> {
    let mut vks = Vec::new();
    for name in names {
        vks.push(vk_of(name)?);
    }
    for vk in &vks {
        send_inputs(&mut [vk_input(*vk, false)])?;
    }
    let result = body();
    for vk in vks.iter().rev() {
        let _ = send_inputs(&mut [vk_input(*vk, true)]);
    }
    result
}

fn wheel_input(flags: MOUSE_EVENT_FLAGS, delta: i64) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: (delta.clamp(-20, 20) * 120) as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn abs_move_input(x: i32, y: i32) -> INPUT {
    let (dx, dy) = abs_mouse(x, y);
    mouse_input(dx, dy, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK)
}

fn abs_button_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    mouse_input(
        dx,
        dy,
        flags | MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
    )
}

fn button_input(flags: MOUSE_EVENT_FLAGS) -> INPUT {
    mouse_input(0, 0, flags)
}

fn mouse_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn abs_mouse(x: i32, y: i32) -> (i32, i32) {
    let (vx, vy, vw, vh) = virtual_screen();
    let vw = vw.max(2);
    let vh = vh.max(2);
    let nx = ((x.saturating_sub(vx) as i64) * 65535) / (vw as i64 - 1);
    let ny = ((y.saturating_sub(vy) as i64) * 65535) / (vh as i64 - 1);
    (nx.clamp(0, 65535) as i32, ny.clamp(0, 65535) as i32)
}

fn send_mouse(input: INPUT) -> Result<(), ToolError> {
    let mut inputs = [input];
    send_inputs(&mut inputs)
}

fn send_inputs(inputs: &mut [INPUT]) -> Result<(), ToolError> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(ToolError::new("SendInput failed"));
    }
    Ok(())
}

fn interpolate_segment(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = ((dx as f64).hypot(dy as f64)).max(1.0);
    let n = ((dist / DRAG_STEP_PX as f64).ceil() as usize).clamp(1, DRAG_MAX_STEPS);
    (1..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            (
                from.0 + (dx as f64 * t).round() as i32,
                from.1 + (dy as f64 * t).round() as i32,
            )
        })
        .collect()
}

fn expand_drag_track(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if (3..=16).contains(&points.len()) {
        catmull_rom(points)
    } else {
        points.to_vec()
    }
}

fn catmull_rom(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut padded = Vec::with_capacity(points.len() + 2);
    padded.push(points[0]);
    padded.extend_from_slice(points);
    padded.push(*points.last().unwrap());
    let mut out = vec![points[0]];
    for i in 1..padded.len() - 2 {
        let p0 = padded[i - 1];
        let p1 = padded[i];
        let p2 = padded[i + 1];
        let p3 = padded[i + 2];
        let dist = ((p2.0 - p1.0) as f64).hypot((p2.1 - p1.1) as f64).max(1.0);
        let n = ((dist / DRAG_STEP_PX as f64).ceil() as usize).clamp(2, DRAG_MAX_STEPS);
        for s in 1..=n {
            let t = s as f64 / n as f64;
            out.push(catmull_point(p0, p1, p2, p3, t));
        }
    }
    out
}

fn catmull_point(
    p0: (i32, i32),
    p1: (i32, i32),
    p2: (i32, i32),
    p3: (i32, i32),
    t: f64,
) -> (i32, i32) {
    let t2 = t * t;
    let t3 = t2 * t;
    let x = 0.5
        * ((2.0 * p1.0 as f64)
            + (-p0.0 as f64 + p2.0 as f64) * t
            + (2.0 * p0.0 as f64 - 5.0 * p1.0 as f64 + 4.0 * p2.0 as f64 - p3.0 as f64) * t2
            + (-p0.0 as f64 + 3.0 * p1.0 as f64 - 3.0 * p2.0 as f64 + p3.0 as f64) * t3);
    let y = 0.5
        * ((2.0 * p1.1 as f64)
            + (-p0.1 as f64 + p2.1 as f64) * t
            + (2.0 * p0.1 as f64 - 5.0 * p1.1 as f64 + 4.0 * p2.1 as f64 - p3.1 as f64) * t2
            + (-p0.1 as f64 + 3.0 * p1.1 as f64 - 3.0 * p2.1 as f64 + p3.1 as f64) * t3);
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_a_line_including_the_end() {
        let steps = interpolate_segment((0, 0), (16, 0));
        assert_eq!(*steps.last().unwrap(), (16, 0));
        assert!(steps.len() >= 2);
    }

    #[test]
    fn catmull_rom_bends_a_corner() {
        let track = expand_drag_track(&[(0, 0), (10, 8), (20, 0)]);
        assert!(track.len() > 3);
        assert_eq!(*track.first().unwrap(), (0, 0));
        assert_eq!(*track.last().unwrap(), (20, 0));
        assert!(track.iter().any(|(_, y)| *y >= 4));
    }

    #[test]
    fn win_key_maps() {
        assert_eq!(vk_of("win").unwrap(), VK_LWIN);
        assert_eq!(vk_of("meta").unwrap(), VK_LWIN);
        assert!(vk_of("ctrl").is_ok());
    }
}
