//! CDP auto-route for Chromium windows with a DevTools port (Ghost MCP parity).

use ghost_session::cdp_route::CdpRoute;
use ghost_session::{By, GhostSession, WindowTarget};
use serde_json::{json, Value};

/// Try to handle an Anya computer-use action via CDP.
/// Returns `Ok(None)` when this action should fall through to UIA/desktop verbs.
pub async fn try_dispatch(
    session: &GhostSession,
    action: &str,
    args: &Value,
    t: &WindowTarget,
    r: &CdpRoute,
) -> Result<Option<Value>, String> {
    let out = match action {
        "see" => Some(see(session, args, r).await?),
        "find" | "find_control" | "findControl" => Some(find(session, args, t, r).await?),
        "act" | "click_control" | "clickControl" | "set_value" | "setValue" | "type" => {
            let mut p = args.clone();
            if matches!(action, "click_control" | "clickControl") {
                if let Some(obj) = p.as_object_mut() {
                    obj.insert("action".into(), json!("click"));
                }
            }
            if matches!(action, "set_value" | "setValue" | "type") {
                if let Some(obj) = p.as_object_mut() {
                    obj.insert("action".into(), json!("type"));
                    if let Some(v) = obj.remove("value").or_else(|| obj.remove("text")) {
                        obj.insert("text_input".into(), v);
                    }
                }
            }
            Some(act(session, &p, t, r).await?)
        }
        "key" | "hotkey" => Some(key(args, t, r).await?),
        "click" => Some(click_at(session, args, t, r).await?),
        "scroll" => Some(scroll(session, args, r).await?),
        "assert" => Some(assert_tool(session, args, r).await?),
        "wait" if matches!(args["for"].as_str().unwrap_or(""), "element" | "value") => {
            Some(wait(session, args, r).await?)
        }
        // drag / window / browser / screenshot / clipboard → desktop path
        _ => None,
    };
    Ok(out.map(|mut v| {
        if let Some(m) = v.as_object_mut() {
            m.insert("route".into(), r.to_json());
            m.insert("target".into(), t.to_json());
            m.insert("dispatch".into(), json!("cdp"));
        }
        v
    }))
}

async fn see(session: &GhostSession, args: &Value, r: &CdpRoute) -> Result<Value, String> {
    let mode = args["mode"].as_str().unwrap_or("full");
    if mode == "text" {
        let max_chars = args["limit"].as_u64().unwrap_or(20_000) as usize;
        let mut text = r.tab.text("").await.map_err(|e| e.to_string())?;
        let truncated = text.len() > max_chars;
        if truncated {
            let mut cut = max_chars.min(text.len());
            while cut > 0 && !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
        }
        return Ok(json!({
            "text": text,
            "chars": text.len(),
            "truncated": truncated,
            "coords": "viewport",
            "mode": "text",
        }));
    }
    let limit = args["limit"].as_u64().unwrap_or(150) as usize;
    let name_filter = args["name_filter"].as_str().or_else(|| args["name"].as_str());
    let els = session
        .cdp_describe(r, limit.max(1), name_filter, None)
        .await
        .map_err(|e| e.to_string())?;
    let controls: Vec<_> = els
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (cx, cy) = e.center();
            json!({
                "i": i,
                "role": e.role,
                "name": e.name,
                "enabled": e.enabled,
                "selector": e.selector,
                "center": { "x": cx, "y": cy },
                "rect": {
                    "left": e.rect.0,
                    "top": e.rect.1,
                    "right": e.rect.2,
                    "bottom": e.rect.3,
                },
            })
        })
        .collect();
    Ok(json!({
        "controls": controls,
        "shown": controls.len(),
        "coords": "viewport",
        "mode": mode,
        "hint": "Chromium window routed via CDP — prefer act by name/role (DOM), not pixel clicks.",
    }))
}

async fn find(
    session: &GhostSession,
    args: &Value,
    t: &WindowTarget,
    r: &CdpRoute,
) -> Result<Value, String> {
    let by = locator(args)?;
    let idx = args["index"].as_u64().map(|v| v as usize);
    let (e, matches) = session
        .cdp_find(r, &by, idx)
        .await
        .map_err(|e| format!("find: {e}"))?;
    let (cx, cy) = e.center();
    Ok(json!({
        "ok": true,
        "name": e.name,
        "role": e.role,
        "selector": e.selector,
        "center": { "x": cx, "y": cy },
        "rect": { "left": e.rect.0, "top": e.rect.1, "right": e.rect.2, "bottom": e.rect.3 },
        "hwnd": t.hwnd,
        "window": t.title,
        "source": "cdp",
        "matches": matches,
        "index": idx,
        "coords": "viewport",
    }))
}

async fn act(
    session: &GhostSession,
    args: &Value,
    t: &WindowTarget,
    r: &CdpRoute,
) -> Result<Value, String> {
    let action = args["action"].as_str().unwrap_or("click");
    let text = args["text_input"]
        .as_str()
        .or_else(|| args["text"].as_str())
        .or_else(|| args["value"].as_str());
    let by = locator(args)?;
    let idx = args["index"].as_u64().map(|v| v as usize);
    let (e, _) = match (&by, action, idx) {
        (By::Name(n), "type", None) => {
            let matches = session
                .cdp_describe(r, 16, Some(n), None)
                .await
                .map_err(|e| e.to_string())?;
            let total = matches.len();
            let pos = matches
                .iter()
                .position(|m| m.role == "edit" || m.role == "combobox" || m.value.is_some())
                .unwrap_or(0);
            match matches.into_iter().nth(pos) {
                Some(m) => (m, total),
                None => session.cdp_find(r, &by, None).await.map_err(|e| e.to_string())?,
            }
        }
        _ => session
            .cdp_find(r, &by, idx)
            .await
            .map_err(|e| e.to_string())?,
    };
    if !e.enabled {
        return Err(format!("Element not interactable: {} — disabled", e.name));
    }
    let (cx, cy) = e.center();
    let tab = &r.tab;
    let (verified, note): (Option<bool>, Option<String>) = match action {
        "click" => {
            tab.click(&e.selector, 5_000)
                .await
                .map_err(|e| e.to_string())?;
            (Some(true), None)
        }
        "double_click" => {
            let (x, y) = tab
                .element_center(&e.selector)
                .await
                .map_err(|e| e.to_string())?;
            tab.click_at_ex(x, y, "left", 2)
                .await
                .map_err(|e| e.to_string())?;
            (Some(true), None)
        }
        "right_click" => {
            let (x, y) = tab
                .element_center(&e.selector)
                .await
                .map_err(|e| e.to_string())?;
            tab.click_at_ex(x, y, "right", 1)
                .await
                .map_err(|e| e.to_string())?;
            (
                Some(true),
                Some("context menu events dispatched via CDP".into()),
            )
        }
        "hover" => {
            tab.hover(&e.selector, 5_000)
                .await
                .map_err(|e| e.to_string())?;
            (Some(true), None)
        }
        "type" => {
            let value = text.ok_or("act action=type requires text_input")?;
            let clear = args["clear"].as_bool().unwrap_or(true);
            tab.type_text(&e.selector, value, clear)
                .await
                .map_err(|e| e.to_string())?;
            let got = tab.value_of(&e.selector).await.unwrap_or_default();
            let ok = got.contains(value);
            (
                Some(ok),
                if ok {
                    None
                } else {
                    Some(format!(
                        "read-back {:?} does not contain typed text",
                        got.chars().take(80).collect::<String>()
                    ))
                },
            )
        }
        other => {
            return Err(format!(
                "cdp act: unknown action '{other}' (click|type|double_click|right_click|hover)"
            ))
        }
    };
    let mut out = json!({
        "ok": true,
        "mode": "cdp",
        "action": action,
        "name": e.name,
        "selector": e.selector,
        "center": { "x": cx, "y": cy },
        "coords": "viewport",
        "window": t.title,
        "verified": verified,
        "focus_preserved": true,
        "cursor_preserved": true,
    });
    if let Some(n) = note {
        out["note"] = Value::String(n);
    }
    Ok(out)
}

async fn key(args: &Value, t: &WindowTarget, r: &CdpRoute) -> Result<Value, String> {
    let keys = args["keys"]
        .as_str()
        .or_else(|| args["key"].as_str())
        .ok_or("key needs keys")?;
    let (mods, key_name) = parse_key_combo(keys)?;
    r.tab
        .press(&key_name, &mods)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "mode": "cdp",
        "keys": keys,
        "window": t.title,
        "focus_preserved": true,
        "cursor_preserved": true,
        "note": "Trusted CDP key event (modifier combos supported).",
    }))
}

async fn click_at(
    session: &GhostSession,
    args: &Value,
    t: &WindowTarget,
    r: &CdpRoute,
) -> Result<Value, String> {
    let x = args["x"].as_i64().ok_or("click needs x")? as i32;
    let y = args["y"].as_i64().ok_or("click needs y")? as i32;
    let viewport = args["coords"].as_str() == Some("viewport");
    let (vx, vy) = if viewport {
        (x as f64, y as f64)
    } else {
        session
            .cdp_screen_to_viewport(r, x, y)
            .await
            .map_err(|e| e.to_string())?
    };
    r.tab
        .click_at(vx, vy)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "mode": "cdp",
        "x": x,
        "y": y,
        "viewport": { "x": vx, "y": vy },
        "window": t.title,
        "focus_preserved": true,
        "cursor_preserved": true,
    }))
}

async fn scroll(session: &GhostSession, args: &Value, r: &CdpRoute) -> Result<Value, String> {
    let direction = args["direction"].as_str().unwrap_or("down");
    let amount = args["amount"].as_i64().unwrap_or(3).max(1) as f64;
    let step = 120.0 * amount;
    let (dx, dy) = match direction {
        "up" => (0.0, -step),
        "down" => (0.0, step),
        "left" => (-step, 0.0),
        "right" => (step, 0.0),
        other => {
            return Err(format!(
                "scroll: unknown direction '{other}'; use up|down|left|right"
            ))
        }
    };
    if args.get("until_name").is_some() || args.get("until_role").is_some() {
        let by = if let Some(n) = args["until_name"].as_str() {
            By::name(n)
        } else {
            By::role(args["until_role"].as_str().unwrap_or(""))
        };
        let max_scrolls = args["max_scrolls"].as_u64().unwrap_or(20).min(100);
        for _ in 0..max_scrolls {
            if session.cdp_find(r, &by, None).await.is_ok() {
                return Ok(json!({ "ok": true, "found": true, "mode": "cdp" }));
            }
            r.tab
                .scroll("", dx, dy)
                .await
                .map_err(|e| e.to_string())?;
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        }
        let found = session.cdp_find(r, &by, None).await.is_ok();
        return Ok(json!({ "ok": found, "found": found, "mode": "cdp" }));
    }
    r.tab
        .scroll("", dx, dy)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "mode": "cdp",
        "direction": direction,
        "amount": amount,
    }))
}

async fn assert_tool(
    session: &GhostSession,
    args: &Value,
    r: &CdpRoute,
) -> Result<Value, String> {
    let kind = args["kind"].as_str().unwrap_or("exists");
    let by = locator(args)?;
    match kind {
        "exists" => {
            session
                .cdp_find(r, &by, None)
                .await
                .map_err(|e| format!("assert failed: {e}"))?;
            Ok(json!({ "ok": true, "kind": "exists", "passed": true }))
        }
        "value" => {
            let expected = args["expected"].as_str().unwrap_or("");
            let (e, _) = session
                .cdp_find(r, &by, None)
                .await
                .map_err(|e| format!("assert failed: {e}"))?;
            let actual = r.tab.value_of(&e.selector).await.unwrap_or_default();
            if actual != expected {
                return Err(format!(
                    "assert value: expected {expected:?}, got {actual:?}"
                ));
            }
            Ok(json!({ "ok": true, "kind": "value", "value": actual }))
        }
        other => Err(format!("assert: unknown kind `{other}`")),
    }
}

async fn wait(session: &GhostSession, args: &Value, r: &CdpRoute) -> Result<Value, String> {
    let for_what = args["for"].as_str().unwrap_or("element");
    let timeout_ms = args["timeout_ms"].as_u64().unwrap_or(10_000);
    let by = locator(args)?;
    let start = std::time::Instant::now();
    match for_what {
        "element" => {
            let appears = args["appears"].as_bool().unwrap_or(true);
            loop {
                let found = session.cdp_find(r, &by, None).await.is_ok();
                if found == appears {
                    return Ok(json!({ "ok": true, "for": "element", "mode": "cdp" }));
                }
                if start.elapsed().as_millis() as u64 >= timeout_ms {
                    return Err(format!("wait for=element timed out ({timeout_ms}ms) via CDP"));
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        }
        "value" => {
            let expected = args["expected"].as_str().unwrap_or("");
            loop {
                if let Ok((e, _)) = session.cdp_find(r, &by, None).await {
                    let got = r.tab.value_of(&e.selector).await.unwrap_or_default();
                    if got == expected || got.contains(expected) {
                        return Ok(json!({
                            "ok": true,
                            "for": "value",
                            "value": got,
                            "mode": "cdp",
                        }));
                    }
                }
                if start.elapsed().as_millis() as u64 >= timeout_ms {
                    return Err(format!("wait for=value timed out ({timeout_ms}ms) via CDP"));
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        }
        other => Err(format!("cdp wait: unsupported for={other}")),
    }
}

fn locator(args: &Value) -> Result<By, String> {
    if let Some(n) = args["name"].as_str().filter(|s| !s.is_empty()) {
        return Ok(By::name(n));
    }
    if let Some(r) = args["role"].as_str().filter(|s| !s.is_empty()) {
        return Ok(By::role(r));
    }
    if let Some(d) = args["description"].as_str().filter(|s| !s.is_empty()) {
        return Ok(By::description(d));
    }
    Err("name, role, or description required".into())
}

fn parse_key_combo(keys: &str) -> Result<(Vec<String>, String), String> {
    let (modifiers, key): (Vec<&str>, &str) = if keys == "+" {
        (Vec::new(), "+")
    } else if let Some(rest) = keys.strip_suffix("++") {
        let mods: Vec<&str> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split('+').collect()
        };
        (mods, "+")
    } else {
        let mut parts: Vec<&str> = keys.split('+').collect();
        let key = parts.pop().unwrap_or("");
        (parts, key)
    };
    if key.is_empty() {
        return Err(format!("key: malformed {keys:?}"));
    }
    Ok((
        modifiers.into_iter().map(String::from).collect(),
        key.to_string(),
    ))
}
