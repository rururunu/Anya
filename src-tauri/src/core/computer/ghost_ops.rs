//! Translate Anya's computer tools to the private Ghost MCP surface.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};

use super::ghost_process::GhostProcess;

const BROWSER: &[(&str, &str)] = &[
    ("launch", "ghost_browser_launch"),
    ("attach", "ghost_browser_attach"),
    ("tabs", "ghost_browser_tabs"),
    ("close", "ghost_browser_close"),
];
const TAB: &[(&str, &str)] = &[
    ("open", "ghost_tab_open"),
    ("close", "ghost_tab_close"),
    ("find", "ghost_tab_find"),
    ("navigate", "ghost_tab_navigate"),
    ("click", "ghost_tab_click"),
    ("type", "ghost_tab_type"),
    ("text", "ghost_tab_text"),
    ("eval", "ghost_tab_eval"),
    ("wait", "ghost_tab_wait_for"),
];
pub(super) fn required_tools() -> impl Iterator<Item = &'static str> {
    [
        "ghost_stop",
        "ghost_reset",
        "ghost_window",
        "ghost_focus_policy",
        "ghost_set_focus_policy",
        "ghost_see",
        "ghost_find",
        "ghost_act",
        "ghost_key",
        "ghost_scroll",
        "ghost_drag",
        "ghost_wait",
        "ghost_assert",
        "ghost_screenshot",
        "ghost_clipboard",
    ]
    .into_iter()
    .chain(BROWSER.iter().chain(TAB).map(|&(_, name)| name))
}

pub(super) async fn dispatch(
    host: &mut GhostProcess,
    action: &str,
    args: &Value,
) -> Result<String, String> {
    let mut args = Value::Object(
        args.as_object()
            .cloned()
            .ok_or("Computer Use needs an argument object")?,
    );
    // Internal Ghost routing parameters belong to the bridge.
    for key in ["_target_hwnd", "hwnd", "background", "coords"] {
        args.as_object_mut().unwrap().remove(key);
    }
    alias(&mut args, "window", "title");
    alias(&mut args, "window", "windowTitle");
    let action = match action {
        "list_windows" | "listWindows" => {
            args = json!({"op":"list"});
            "window"
        }
        "focus_window" | "focusWindow" => {
            alias(&mut args, "name", "title");
            args["op"] = json!("focus");
            "window"
        }
        "launch" => {
            args["op"] = json!("launch");
            "window"
        }
        "click_control" | "clickControl" => {
            args["action"] = json!("click");
            "act"
        }
        "set_value" | "setValue" => {
            args["action"] = json!("type");
            alias(&mut args, "text_input", "value");
            "act"
        }
        "type" => {
            args["action"] = json!("type");
            alias(&mut args, "text_input", "text");
            "act"
        }
        "find_control" | "findControl" => "find",
        "hotkey" => "key",
        "screenInfo" => "screen_info",
        other => other,
    };

    let value = match action {
        "window" => window(host, args).await?,
        "screen_info" => {
            let mut policy = host.call("ghost_focus_policy", json!({})).await?;
            let windows = host.call("ghost_window", json!({"op":"list"})).await?;
            policy["windows"] = json!(windows["windows"].as_array().map_or(0, Vec::len));
            policy
        }
        "see" => {
            let filter = args["name_filter"].as_str().map(str::to_lowercase);
            let limit = args["limit"].as_u64().unwrap_or(150) as usize;
            if filter.is_some() && args["mode"] != "text" {
                args["limit"] = json!(0);
            }
            let mut seen = host.call("ghost_see", args).await?;
            if let (Some(filter), Some(elements)) = (filter, seen["elements"].as_array_mut()) {
                elements.retain(|el| {
                    el["name"]
                        .as_str()
                        .is_some_and(|s| s.to_lowercase().contains(&filter))
                });
                if limit > 0 {
                    elements.truncate(limit);
                }
            }
            seen
        }
        "find" => host.call("ghost_find", args).await?,
        "act" => {
            default(&mut args, "action", json!("click"));
            if args["action"] == "type" {
                alias(&mut args, "text_input", "text");
            }
            pin_window(host, &mut args).await?;
            host.call("ghost_act", args).await?
        }
        "wait" => {
            match args["for"].as_str().unwrap_or("ms") {
                "ms" | "idle" | "element" | "value" | "event" => {}
                _ => {
                    return Err("Supported wait conditions: ms, idle, element, value, event".into())
                }
            }
            alias(&mut args, "predicate", "pred");
            alias(&mut args, "text", "expected");
            default(&mut args, "timeout_ms", json!(10000));
            host.call("ghost_wait", args).await?
        }
        "assert" => {
            args["predicate"] = json!(match args["kind"].as_str().unwrap_or("exists") {
                "exists" => "element-exists",
                "value" => "value-equals",
                _ => return Err("Assert needs kind=exists or value".into()),
            });
            alias(&mut args, "text", "expected");
            host.call("ghost_assert", args).await?
        }
        "clipboard" if args["op"] == "paste" => {
            args["keys"] = json!("Ctrl+V");
            key(host, args).await?
        }
        "clipboard" => host.call("ghost_clipboard", args).await?,
        "key" => key(host, args).await?,
        "scroll" => {
            pin_window(host, &mut args).await?;
            if args["direction"].is_null() {
                let (delta, positive, negative) = if let Some(dy) = args["dy"].as_i64() {
                    (dy, "down", "up")
                } else if let Some(dx) = args["dx"].as_i64() {
                    (dx, "right", "left")
                } else {
                    (args["amount"].as_i64().unwrap_or(3), "down", "up")
                };
                args["direction"] = json!(if delta < 0 { negative } else { positive });
                args["amount"] = json!(delta.unsigned_abs().clamp(1, 100));
            }
            host.call("ghost_scroll", args).await?
        }
        "click" | "drag" => pointer(host, action, args).await?,
        "move" | "hover" => return Ok("Use act action=hover with a named control.".into()),
        "screenshot" => {
            let _suspend = crate::services::computer_use_hud::CaptureSuspend::enter();
            args["full"] = json!(args["scope"] == "desktop" && args["window"].is_null());
            args["max_dim"] = json!(1280);
            let image = host.call("ghost_screenshot", args).await?;
            return render_result(image);
        }
        "browser" => {
            let name = operation(BROWSER, args["op"].as_str().unwrap_or("launch"))?;
            alias(&mut args, "browser", "which");
            default(&mut args, "mode", json!("windowed"));
            host.call(name, args).await?
        }
        "tab" => {
            let name = operation(TAB, args["op"].as_str().unwrap_or("open"))?;
            alias(&mut args, "tab", "target_id");
            alias(&mut args, "query", "needle");
            alias(&mut args, "expression", "js");
            host.call(name, args).await?
        }
        other => return Err(format!("Unknown computer action '{other}'")),
    };
    render_result(value)
}

fn operation(table: &[(&str, &'static str)], op: &str) -> Result<&'static str, String> {
    table
        .iter()
        .find(|(key, _)| *key == op)
        .map(|&(_, name)| name)
        .ok_or_else(|| format!("Unsupported Ghost operation '{op}'"))
}

fn default(args: &mut Value, key: &str, value: Value) {
    if args[key].is_null() {
        args[key] = value;
    }
}

fn alias(args: &mut Value, key: &str, alias: &str) {
    let value = args[alias].clone();
    if !value.is_null() {
        default(args, key, value);
    }
}

async fn window(host: &mut GhostProcess, mut args: Value) -> Result<Value, String> {
    match args["op"].as_str().unwrap_or("list") {
        "clear_anchor" => {
            host.call("ghost_window", json!({"op":"anchor","clear":true}))
                .await
        }
        "policy" => {
            alias(&mut args, "policy", "name");
            default(&mut args, "policy", json!("background"));
            host.call("ghost_set_focus_policy", args).await
        }
        "launch" => {
            for key in ["target", "path", "command"] {
                alias(&mut args, "exe", key);
            }
            let exe = args["exe"].as_str().ok_or("Window launch needs exe")?;
            let hint = std::path::Path::new(exe)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(exe)
                .to_owned();
            if let Some(arguments) = args["args"].as_str().filter(|s| !s.is_empty()) {
                args["exe"] = json!(format!("{exe} {arguments}"));
            }
            let mut launched = foreground_call(host, "ghost_window", args, None).await?;
            if let Ok(anchor) = host
                .call("ghost_window", json!({"op":"anchor","name":hint}))
                .await
            {
                launched["target"] = anchor["anchor"].clone();
            }
            Ok(launched)
        }
        "focus" if args["raise"] == true => foreground_call(host, "ghost_window", args, None).await,
        "focus" => {
            args["op"] = json!("anchor");
            let target = host.call("ghost_window", args).await?;
            Ok(
                json!({"anchored": !target["anchor"].is_null(), "raised": false, "target": target["anchor"]}),
            )
        }
        "state" => {
            default(&mut args, "state", json!("restore"));
            host.call("ghost_window", args).await
        }
        "list" | "anchor" => host.call("ghost_window", args).await,
        _ => Err("Unsupported window operation".into()),
    }
}

async fn pin_window(host: &mut GhostProcess, args: &mut Value) -> Result<Value, String> {
    let target_required = "Select a live target with window= or window anchor.";
    let query = match args["window"].as_str() {
        Some(window)
            if !window.trim().is_empty() && !window.trim().eq_ignore_ascii_case("foreground") =>
        {
            json!({"op":"anchor","name":window})
        }
        Some(_) => return Err(target_required.into()),
        None => json!({"op":"anchor"}),
    };
    let result = host.call("ghost_window", query).await?;
    let target = result["anchor"].clone();
    let title = target["title"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(target_required)?;
    args["window"] = json!(title);
    Ok(target)
}

async fn key(host: &mut GhostProcess, mut args: Value) -> Result<Value, String> {
    alias(&mut args, "keys", "key");
    let keys = args["keys"]
        .as_str()
        .ok_or("Provide keys")?
        .to_ascii_lowercase();
    if keys.starts_with("down:") || keys.starts_with("up:") {
        return Err("Use a complete key press or chord.".into());
    }
    pin_window(host, &mut args).await?;
    if !keys.trim_matches('+').contains('+')
        || matches!(
            keys.as_str(),
            "ctrl+c" | "ctrl+x" | "ctrl+v" | "ctrl+a" | "ctrl+z"
        )
    {
        args["background"] = json!(true);
        host.call("ghost_key", args).await
    } else {
        foreground_call(host, "ghost_key", args, None).await
    }
}

async fn pointer(host: &mut GhostProcess, action: &str, mut args: Value) -> Result<Value, String> {
    let target = pin_window(host, &mut args).await?;
    let hwnd = target["hwnd"]
        .as_i64()
        .filter(|id| *id != 0)
        .ok_or("Target has no native window handle")?;
    let (tool, points) = if action == "drag" {
        for (key, old) in [
            ("from_x", "x"),
            ("from_y", "y"),
            ("to_x", "x2"),
            ("to_y", "y2"),
        ] {
            alias(&mut args, key, old);
        }
        (
            "ghost_drag",
            vec![
                point(&args, "from_x", "from_y")?,
                point(&args, "to_x", "to_y")?,
            ],
        )
    } else {
        ("ghost_click_at", vec![point(&args, "x", "y")?])
    };
    let real = action == "drag"
        || args["real"] == true
        || matches!(
            args["input"].as_str(),
            Some("real" | "foreground" | "sendinput")
        );
    if real {
        if target["surface"] == "hidden" {
            return Err("Real pointer input requires a visible window.".into());
        }
        foreground_call(host, tool, args, Some((hwnd, &points))).await
    } else {
        args["hwnd"] = json!(hwnd);
        // Ensure the posted-click route pins the resolved HWND.
        let policy = host.call("ghost_focus_policy", json!({})).await?;
        let previous = policy["policy"]
            .as_str()
            .ok_or("Ghost focus policy unavailable")?;
        call_with_policy(host, "background", previous, tool, args, None).await
    }
}

fn point(args: &Value, x: &str, y: &str) -> Result<(i32, i32), String> {
    let coord = |key: &str| {
        args[key]
            .as_i64()
            .and_then(|val| i32::try_from(val).ok())
            .ok_or_else(|| format!("Provide integer coordinate {key}"))
    };
    Ok((coord(x)?, coord(y)?))
}

async fn foreground_call(
    host: &mut GhostProcess,
    tool: &str,
    args: Value,
    pointer: Option<(i64, &[(i32, i32)])>,
) -> Result<Value, String> {
    let policy = host.call("ghost_focus_policy", json!({})).await?;
    let previous = policy["policy"]
        .as_str()
        .ok_or("Ghost focus policy unavailable")?;
    call_with_policy(host, "foreground", previous, tool, args, pointer).await
}

async fn call_with_policy(
    host: &mut GhostProcess,
    policy: &str,
    previous: &str,
    tool: &str,
    args: Value,
    pointer: Option<(i64, &[(i32, i32)])>,
) -> Result<Value, String> {
    if previous != policy {
        host.call("ghost_set_focus_policy", json!({"policy":policy}))
            .await?;
    }
    let result = async {
        if let Some((hwnd, points)) = pointer {
            host.call("ghost_window", json!({"op":"focus","name":args["window"]}))
                .await?;
            verify_pointer_target(hwnd, points)?;
        }
        host.call(tool, args).await
    }
    .await;
    if previous != policy {
        host.call("ghost_set_focus_policy", json!({"policy":previous})).await
            .map_err(|err| format!("Unable to restore Ghost focus policy. Inspect the target before retrying: {err}"))?;
    }
    result
}

fn verify_pointer_target(hwnd: i64, points: &[(i32, i32)]) -> Result<(), String> {
    use windows::Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, WindowFromPoint, GA_ROOT},
    };
    let expected = HWND(hwnd as isize as *mut _);
    // Raw Ghost pointer actions use desktop coordinates. Confirm the raised
    // target owns each point before asking Ghost to generate real input.
    let matches = unsafe {
        GetForegroundWindow() == expected
            && points
                .iter()
                .all(|&(x, y)| GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) == expected)
    };
    if matches {
        Ok(())
    } else {
        Err("The requested coordinates are outside the foreground target window.".into())
    }
}

fn render_result(mut value: Value) -> Result<String, String> {
    let mut images = Vec::new();
    if let Some(object) = value.as_object_mut() {
        for (key, extension) in [
            ("png_base64", "png"),
            ("jpeg_base64", "jpg"),
            ("marked_jpeg_base64", "jpg"),
        ] {
            if let Some(encoded) = object
                .remove(key)
                .and_then(|v| v.as_str().map(str::to_owned))
            {
                let bytes = STANDARD
                    .decode(encoded)
                    .map_err(|err| format!("Invalid Ghost image: {err}"))?;
                let dir = crate::core::tools::memory::user_data_dir()
                    .join("computer")
                    .join("captures");
                std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
                let path = dir.join(format!("shot-{}.{extension}", uuid::Uuid::new_v4()));
                std::fs::write(&path, bytes).map_err(|err| err.to_string())?;
                images.push(format!(
                    "![image](path:{})",
                    path.to_string_lossy().replace('\\', "/")
                ));
            }
        }
    }
    if images.is_empty() {
        Ok(value.to_string())
    } else {
        Ok(format!("{value}\n{}", images.join("\n")))
    }
}
