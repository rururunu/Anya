//! Computer-use tool actions (screenshot, pointer, keyboard, UIA, launch).

use std::time::Duration;

use serde_json::Value;

use super::points;
use super::win;
use super::SETTLE_MS;
use crate::core::tools::error::ToolError;

pub(super) enum ShotScope {
    Desktop,
    Foreground,
    Title(String),
}

pub(super) fn parse_shot_scope(args: &Value) -> ShotScope {
    if let Some(title) = args
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return ShotScope::Title(title.to_string());
    }
    match args
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("foreground")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "desktop" | "screen" | "full" => ShotScope::Desktop,
        _ => ShotScope::Foreground,
    }
}

pub(super) fn execute(plugin_id: &str, action: &str, args: &Value) -> Result<String, ToolError> {
    match action {
        "screenshot" => screenshot(plugin_id, args),
        "screenInfo" | "screen_info" => screen_info(),
        "listWindows" | "list_windows" => list_windows(),
        "focusWindow" | "focus_window" => focus_window(args),
        "findControl" | "find_control" => find_control(args),
        "clickControl" | "click_control" => click_control(args),
        "setValue" | "set_value" => set_value(args),
        "launch" => launch(args),
        "click" => click(args),
        "drag" => drag(args),
        "move" => move_pointer(args),
        "hover" => hover(args),
        "scroll" => scroll(args),
        "wait" => wait(args),
        "type" => type_text(args),
        "key" => key(args),
        _ => Err(ToolError::new(format!(
            "unknown computer action `{action}`"
        ))),
    }
}

fn screenshot(plugin_id: &str, args: &Value) -> Result<String, ToolError> {
    let target = match parse_shot_scope(args) {
        ShotScope::Desktop => win::CaptureTarget::Desktop,
        ShotScope::Foreground => win::CaptureTarget::Foreground,
        ShotScope::Title(title) => win::CaptureTarget::Title(title),
    };
    let capture = win::capture_to_plugin(plugin_id, target)?;
    let scale = capture.image_w as f64 / capture.screen_w.max(1) as f64;
    points::remember_capture(scale, capture.origin_x, capture.origin_y);
    let tree = win::overlay_for_hwnd(capture.hwnd, capture.origin_x, capture.origin_y, scale);
    if capture.unchanged {
        return Ok(format!(
            "Screenshot unchanged ({}); same pixels as the last capture. click x/y still use the previous image.\n{tree}",
            capture.label
        ));
    }
    std::thread::sleep(Duration::from_millis(SETTLE_MS));
    let href = capture.path.to_string_lossy().replace('\\', "/");
    Ok(format!(
        "Screenshot {}x{} of {}x{} {} (scale {scale:.4}, dpi {}). JPEG pixels → screen via this scale (physical, not CSS). Prefer click_control index/name/id from the list; pixel click only for canvas.\n![image](path:{href})\n{tree}",
        capture.image_w, capture.image_h, capture.screen_w, capture.screen_h, capture.label, capture.dpi
    ))
}

fn click_control(args: &Value) -> Result<String, ToolError> {
    let name = args["name"].as_str();
    let id = args["id"]
        .as_str()
        .or_else(|| args["automationId"].as_str())
        .or_else(|| args["automation_id"].as_str());
    let index = args["index"].as_u64();
    let (out, x, y) = win::click_control(name, id, index)?;
    settle_observed(out, x, y)
}

fn click(args: &Value) -> Result<String, ToolError> {
    let (x, y) = points::to_screen_xy(args)?;
    let button = args["button"]
        .as_str()
        .unwrap_or("left")
        .trim()
        .to_ascii_lowercase();
    let count = args["count"].as_u64().unwrap_or(1).clamp(1, 3) as u32;
    let modifiers = points::parse_modifiers(args)?;
    win::mouse_click(x, y, &button, count, &modifiers)?;
    let mods = format_mods(&modifiers);
    settle_observed(
        format!("clicked {button}{mods} x{count} at screen ({x},{y})"),
        x,
        y,
    )
}

fn drag(args: &Value) -> Result<String, ToolError> {
    let button = args["button"]
        .as_str()
        .unwrap_or("left")
        .trim()
        .to_ascii_lowercase();
    let path = points::drag_screen_points(args)?;
    let modifiers = points::parse_modifiers(args)?;
    win::mouse_drag(&path, &button, &modifiers)?;
    let (x, y) = path.last().copied().unwrap_or((0, 0));
    let mods = format_mods(&modifiers);
    settle_mouse(format!(
        "dragged {button}{mods} {} points ending at screen ({x},{y})",
        path.len()
    ))
}

fn move_pointer(args: &Value) -> Result<String, ToolError> {
    let (x, y) = points::to_screen_xy(args)?;
    win::mouse_move(x, y)?;
    let ms = points::wait_ms(args, 0);
    if ms > 0 {
        std::thread::sleep(Duration::from_millis(ms));
        return Ok(format!("moved pointer to screen ({x},{y}) and hovered {ms}ms"));
    }
    Ok(format!("moved pointer to screen ({x},{y})"))
}

fn hover(args: &Value) -> Result<String, ToolError> {
    let (x, y) = points::to_screen_xy(args)?;
    win::mouse_move(x, y)?;
    let ms = points::wait_ms(args, 400).max(50);
    std::thread::sleep(Duration::from_millis(ms));
    Ok(format!("hovered at screen ({x},{y}) for {ms}ms"))
}

fn wait(args: &Value) -> Result<String, ToolError> {
    let ms = points::wait_ms(args, 400).clamp(50, 4000);
    std::thread::sleep(Duration::from_millis(ms));
    Ok(format!("waited {ms}ms"))
}

fn scroll(args: &Value) -> Result<String, ToolError> {
    let (x, y) = points::to_screen_xy(args)?;
    let dy = args["dy"]
        .as_i64()
        .or_else(|| args["deltaY"].as_i64())
        .unwrap_or(0);
    let dx = args["dx"]
        .as_i64()
        .or_else(|| args["deltaX"].as_i64())
        .unwrap_or(0);
    win::mouse_scroll_at(x, y, dx, dy)?;
    settle_mouse(format!("scrolled dx={dx} dy={dy} at screen ({x},{y})"))
}

fn format_mods(modifiers: &[String]) -> String {
    if modifiers.is_empty() {
        String::new()
    } else {
        format!(" ({})", modifiers.join("+"))
    }
}

fn settle_mouse(msg: String) -> Result<String, ToolError> {
    std::thread::sleep(Duration::from_millis(SETTLE_MS));
    Ok(msg)
}

fn settle_observed(msg: String, x: i32, y: i32) -> Result<String, ToolError> {
    std::thread::sleep(Duration::from_millis(SETTLE_MS));
    Ok(format!("{msg}. {}", win::after_pointer(x, y)))
}

fn screen_info() -> Result<String, ToolError> {
    let (vx, vy, vw, vh) = win::virtual_screen();
    Ok(format!(
        "virtual screen origin ({vx},{vy}) size {vw}x{vh} (physical pixels). click x/y are JPEG pixels of the last screenshot, mapped by its scale. Prefer launch / key / click_control."
    ))
}

fn list_windows() -> Result<String, ToolError> {
    win::list_windows()
}

fn focus_window(args: &Value) -> Result<String, ToolError> {
    let title = args["title"].as_str();
    let id = args["id"].as_str().or_else(|| args["hwnd"].as_str());
    win::focus_window(title, id)
}

fn find_control(args: &Value) -> Result<String, ToolError> {
    let name = args["name"]
        .as_str()
        .or_else(|| args["id"].as_str())
        .unwrap_or("")
        .trim();
    if name.is_empty() {
        return Err(ToolError::new("find_control needs name or id"));
    }
    let title = args["windowTitle"]
        .as_str()
        .or_else(|| args["title"].as_str());
    win::find_control(name, title)
}

fn set_value(args: &Value) -> Result<String, ToolError> {
    let value = args["value"]
        .as_str()
        .or_else(|| args["text"].as_str())
        .unwrap_or("");
    if value.is_empty() {
        return Err(ToolError::new("set_value needs value"));
    }
    let name = args["name"].as_str();
    let id = args["id"]
        .as_str()
        .or_else(|| args["automationId"].as_str());
    let index = args["index"].as_u64();
    win::set_value(name, id, index, value)
}

fn launch(args: &Value) -> Result<String, ToolError> {
    let target = args["target"]
        .as_str()
        .or_else(|| args["file"].as_str())
        .or_else(|| args["path"].as_str())
        .unwrap_or("")
        .trim();
    if target.is_empty() {
        return Err(ToolError::new("launch needs target (mspaint, notepad, file path, or URI)"));
    }
    let extra = args["args"].as_str().or_else(|| args["arguments"].as_str());
    let out = win::launch(target, extra)?;
    let ms = points::wait_ms(args, 200).min(2000);
    if ms > 0 {
        std::thread::sleep(Duration::from_millis(ms));
    }
    Ok(format!(
        "{out}. App should be opening — type/key/click_control next; do not screenshot-tour Start."
    ))
}

fn type_text(args: &Value) -> Result<String, ToolError> {
    let text = args["text"].as_str().unwrap_or("");
    if text.is_empty() {
        return Err(ToolError::new("type needs text"));
    }
    if text.chars().count() > 4000 {
        return Err(ToolError::new("type text is too long (max 4000 chars)"));
    }
    win::type_text(text)?;
    std::thread::sleep(Duration::from_millis(SETTLE_MS));
    Ok(format!("typed {} characters", text.chars().count()))
}

fn key(args: &Value) -> Result<String, ToolError> {
    let keys = args["keys"]
        .as_str()
        .or_else(|| args["key"].as_str())
        .unwrap_or("")
        .trim();
    if keys.is_empty() {
        return Err(ToolError::new("key needs keys (e.g. enter, ctrl+c)"));
    }
    win::hotkey(keys)?;
    std::thread::sleep(Duration::from_millis(SETTLE_MS));
    Ok(format!("pressed {keys}"))
}
