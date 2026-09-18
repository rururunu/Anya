//! Map Anya computer-use tool names onto GhostSession APIs.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use ghost_session::{By, GhostSession, LocateMode, Region, Target};
use serde_json::{json, Value};

use super::ghost_cdp;
use super::ghost_scope::{
    attach_target_json, enrich_not_found, needs_foreground_act_fallback, prepare, resolve_target,
    with_focus_guard, with_temporary_raise,
};
use crate::core::plugins::manifest::plugin_dir;

pub(super) async fn dispatch(
    session: &GhostSession,
    plugin_id: &str,
    action: &str,
    args: &Value,
) -> Result<String, String> {
    let prepared = prepare(session, action, args).await;
    let args: &Value = prepared.args.as_ref().unwrap_or(args);

    // CDP auto-route (Ghost MCP): Chromium window with DevTools port → DOM verbs.
    if let Some(t) = prepared.target.as_ref() {
        if let Some(route) = session.cdp_route_for(t).await {
            match ghost_cdp::try_dispatch(session, action, args, t, &route).await {
                Ok(Some(v)) => {
                    return Ok(attach_target_json(v, Some(t)).to_string());
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(enrich_not_found(session, e, args, t, Some(&route)).await);
                }
            }
        }
    }

    let target_ref = prepared.target.as_ref();
    let result = with_focus_guard(session, target_ref, action, async {
        dispatch_verb(session, plugin_id, action, args).await
    })
    .await;

    match (result, prepared.target.as_ref()) {
        (Ok(body), Some(t)) => {
            let mut v = serde_json::from_str::<Value>(&body).unwrap_or(json!({ "result": body }));
            v = attach_target_json(v, Some(t));
            Ok(v.to_string())
        }
        (Err(e), Some(t)) => Err(enrich_not_found(session, e, args, t, None).await),
        (other, _) => other,
    }
}

async fn dispatch_verb(
    session: &GhostSession,
    plugin_id: &str,
    action: &str,
    args: &Value,
) -> Result<String, String> {
    match action {
        "see" => see(session, args).await,
        "find" => find(session, args).await,
        "act" => act(session, args).await,
        "wait" => wait(session, args).await,
        "assert" => assert_tool(session, args).await,
        "window" => window(session, args).await,
        "key" | "hotkey" => key(session, args).await,
        "scroll" => scroll(session, args).await,
        "drag" => drag(session, args).await,
        "clipboard" => clipboard(session, args).await,
        "screenshot" => screenshot(session, plugin_id, args).await,
        "screen_info" | "screenInfo" => screen_info(session).await,
        "list_windows" | "listWindows" => window(session, &json!({ "op": "list" })).await,
        "focus_window" | "focusWindow" => {
            let name = args["title"]
                .as_str()
                .or_else(|| args["name"].as_str())
                .unwrap_or("");
            window(session, &json!({ "op": "focus", "name": name })).await
        }
        "launch" => {
            let exe = args["exe"]
                .as_str()
                .or_else(|| args["target"].as_str())
                .or_else(|| args["path"].as_str())
                .or_else(|| args["command"].as_str())
                .unwrap_or("");
            let mut payload = json!({ "op": "launch", "exe": exe });
            if let Some(a) = args["args"].as_str() {
                payload["args"] = json!(a);
            }
            window(session, &payload).await
        }
        "find_control" | "findControl" => find(session, args).await,
        "click_control" | "clickControl" => {
            let mut p = args.clone();
            if let Some(obj) = p.as_object_mut() {
                obj.insert("action".into(), json!("click"));
            }
            act(session, &p).await
        }
        "set_value" | "setValue" => {
            let mut p = args.clone();
            if let Some(obj) = p.as_object_mut() {
                obj.insert("action".into(), json!("type"));
                if let Some(v) = obj.remove("value") {
                    obj.insert("text_input".into(), v);
                }
            }
            act(session, &p).await
        }
        "click" => click_at(session, args).await,
        "type" => {
            let mut p = args.clone();
            if let Some(obj) = p.as_object_mut() {
                obj.insert("action".into(), json!("type"));
                if let Some(t) = obj.remove("text") {
                    obj.insert("text_input".into(), t);
                }
            }
            act(session, &p).await
        }
        "move" | "hover" => Ok(
            "hover/move is discouraged; use act (click/type) with name/role instead".into(),
        ),
        "browser" => browser(session, args).await,
        "tab" => tab(session, args).await,
        other => Err(format!("unknown computer action `{other}`")),
    }
}

async fn see(session: &GhostSession, args: &Value) -> Result<String, String> {
    let scoped = resolve_target(session, args).await;
    // Ghost MCP: scoped see upgrades fast→full (fast is FG-only).
    let mode = match (args["mode"].as_str(), &scoped) {
        (None | Some("fast"), Some(_)) => "full",
        (Some(m), _) => m,
        (None, None) => "fast",
    };
    let window = scoped.as_ref().map(|t| t.title.as_str());
    let limit = args["limit"].as_u64().unwrap_or(150) as usize;
    let name_filter = args["name_filter"]
        .as_str()
        .or_else(|| args["name"].as_str())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    if mode == "text" {
        if let Some(name) = args["name"].as_str() {
            if let Some(t) = &scoped {
                let _ = session.find_background(&t.title, By::name(name), None).await;
                return with_temporary_raise(session, &t.title, || async {
                    let text = session
                        .get_text(By::name(name))
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({
                        "text": text,
                        "mode": "text",
                        "target": t.to_json(),
                    })
                    .to_string())
                })
                .await;
            }
            let text = session
                .get_text(By::name(name))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(json!({ "text": text, "mode": "text" }).to_string());
        }
        if let Some(t) = &scoped {
            let (text, truncated) = session
                .read_text(Some(&t.title), 8_000)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(json!({
                "text": text,
                "truncated": truncated,
                "mode": "text",
                "target": t.to_json(),
            })
            .to_string());
        }
    }

    let elements = match (mode, window) {
        ("fast", None) => session.describe_screen_fast().await.map_err(|e| e.to_string())?,
        _ => session
            .describe_screen(window)
            .await
            .map_err(|e| e.to_string())?,
    };

    let mut rows = Vec::new();
    let mut n = 0usize;
    for (i, el) in elements.iter().enumerate() {
        if let Some(f) = &name_filter {
            if !el.name.to_ascii_lowercase().contains(f) {
                continue;
            }
        }
        if limit > 0 && n >= limit {
            break;
        }
        let cx = (el.left + el.right) / 2;
        let cy = (el.top + el.bottom) / 2;
        rows.push(json!({
            "i": i,
            "role": el.role,
            "name": el.name,
            "enabled": el.enabled,
            "center": { "x": cx, "y": cy },
            "rect": {
                "left": el.left,
                "top": el.top,
                "width": (el.right - el.left).max(0),
                "height": (el.bottom - el.top).max(0),
            },
        }));
        n += 1;
    }
    Ok(json!({
        "controls": rows,
        "shown": n,
        "total": elements.len(),
        "mode": mode,
        "target": scoped.as_ref().map(|t| t.to_json()),
        "hint": "Prefer act by name/role with window=; read verified. Do not pixel-click named controls.",
    })
    .to_string())
}

async fn act(session: &GhostSession, args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("click");
    let text = args["text_input"]
        .as_str()
        .or_else(|| args["text"].as_str())
        .or_else(|| args["value"].as_str());
    let index = args["index"].as_u64().map(|v| v as usize);
    let by = locator(args)?;
    let role = args["role"].as_str();

    if let Some(t) = resolve_target(session, args).await {
        // Description → ground cascade (OCR/VLM), then real click/type at coords.
        if let By::Description(desc) = &by {
            let mode = match args["mode"].as_str().unwrap_or("instant") {
                "deliberate" => LocateMode::Deliberate,
                "instant_only" => LocateMode::InstantOnly,
                _ => LocateMode::Instant,
            };
            let grounded = session
                .ground(Target::Description(desc.clone()), mode)
                .await
                .map_err(|e| e.to_string())?;
            let (cx, cy) = grounded.center;
            let confidence = grounded.confidence;
            let src = format!("{:?}", grounded.source);
            let title = t.title.clone();
            let target_json = t.to_json();
            let text_owned = text.map(|s| s.to_string());
            let action_owned = action.to_string();
            return with_temporary_raise(session, &title, || async {
                session
                    .click_at(cx, cy)
                    .await
                    .map_err(|e| e.to_string())?;
                if action_owned == "type" {
                    if let Some(txt) = text_owned.as_deref() {
                        session
                            .paste_text(txt)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
                Ok(json!({
                    "ok": true,
                    "dispatch": "ground",
                    "source": src,
                    "confidence": confidence,
                    "center": { "x": cx, "y": cy },
                    "action": action_owned,
                    "target": target_json,
                    "note": "Grounded description then real click (and paste if type).",
                })
                .to_string())
            })
            .await;
        }

        let hwnd_hint = args["_target_hwnd"]
            .as_i64()
            .map(|h| h as isize)
            .or_else(|| (t.hwnd != 0 && !t.is_hidden()).then_some(t.hwnd));
        match session
            .act_background(&t.title, by.clone(), action, text, index, hwnd_hint, role)
            .await
        {
            Ok(mut result) => {
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("target".into(), t.to_json());
                    obj.insert("dispatch".into(), json!("background"));
                }
                return Ok(result.to_string());
            }
            Err(e) => {
                let msg = e.to_string();
                if !needs_foreground_act_fallback(&msg) {
                    return Err(enrich_not_found(session, msg, args, &t, None).await);
                }
                let result = with_temporary_raise(session, &t.title, || async {
                    session
                        .act(by.clone(), action, text)
                        .await
                        .map_err(|e| e.to_string())
                })
                .await?;
                let mut out = result;
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("target".into(), t.to_json());
                    obj.insert("dispatch".into(), json!("foreground_fallback"));
                }
                return Ok(out.to_string());
            }
        }
    }

    // No window / anchor: honest FG path (Ghost MCP warns via target_note).
    let result = session
        .act(by, action, text)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "result": result,
        "dispatch": "foreground",
        "target_note": "no window anchored — acted on the human foreground. Pass window= or window launch/anchor first.",
    })
    .to_string())
}

async fn find(session: &GhostSession, args: &Value) -> Result<String, String> {
    let by = locator(args)?;
    let index = args["index"].as_u64().map(|v| v as usize);
    let mode = match args["mode"].as_str().unwrap_or("instant") {
        "deliberate" => LocateMode::Deliberate,
        "instant_only" => LocateMode::InstantOnly,
        _ => LocateMode::Instant,
    };

    if let Some(t) = resolve_target(session, args).await {
        match session
            .find_background(&t.title, by.clone(), index)
            .await
        {
            Ok(found) => {
                return Ok(json!({
                    "ok": true,
                    "dispatch": "background",
                    "source": "uia",
                    "target": t.to_json(),
                    "found": found,
                })
                .to_string());
            }
            Err(e) => {
                let msg = e.to_string();
                // Description / vision miss → ground cascade then report coords.
                if let By::Description(desc) = &by {
                    let grounded = session
                        .ground(Target::Description(desc.clone()), mode)
                        .await
                        .map_err(|err| err.to_string())?;
                    return Ok(json!({
                        "ok": true,
                        "dispatch": "ground",
                        "source": format!("{:?}", grounded.source),
                        "confidence": grounded.confidence,
                        "center": { "x": grounded.center.0, "y": grounded.center.1 },
                        "rect": {
                            "left": grounded.rect.0,
                            "top": grounded.rect.1,
                            "right": grounded.rect.2,
                            "bottom": grounded.rect.3,
                        },
                        "name": grounded.name,
                        "target": t.to_json(),
                        "note": "Grounded by vision/OCR — use click real=true or act at these coords.",
                    })
                    .to_string());
                }
                return Err(msg);
            }
        }
    }

    // No window: ground against foreground.
    let target = match &by {
        By::Name(n) => Target::Name(n.clone()),
        By::Role(r) => Target::Role(r.clone()),
        By::Description(d) => Target::Description(d.clone()),
    };
    let grounded = session
        .ground(target, mode)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "dispatch": "ground",
        "source": format!("{:?}", grounded.source),
        "confidence": grounded.confidence,
        "center": { "x": grounded.center.0, "y": grounded.center.1 },
        "name": grounded.name,
        "target_note": "no window anchored — grounded on human foreground.",
    })
    .to_string())
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
    if let Some(id) = args["id"]
        .as_str()
        .or_else(|| args["automationId"].as_str())
        .or_else(|| args["automation_id"].as_str())
        .filter(|s| !s.is_empty())
    {
        // AutomationId often mirrors accessible name; treat as name search.
        return Ok(By::name(id));
    }
    Err("act requires name, role, or description".into())
}

async fn wait(session: &GhostSession, args: &Value) -> Result<String, String> {
    let for_what = args["for"].as_str().unwrap_or("ms");
    let timeout_ms = args["timeout_ms"].as_u64().unwrap_or(10_000);
    match for_what {
        "element" => {
            let by = locator(args)?;
            let appears = args["appears"].as_bool().unwrap_or(true);
            if let Some(t) = resolve_target(session, args).await {
                let start = std::time::Instant::now();
                loop {
                    let found = session
                        .find_background(&t.title, by.clone(), None)
                        .await
                        .is_ok();
                    if found == appears {
                        return Ok(json!({
                            "ok": true,
                            "for": "element",
                            "target": t.to_json(),
                            "dispatch": "background",
                        })
                        .to_string());
                    }
                    if start.elapsed().as_millis() as u64 >= timeout_ms {
                        return Err(format!(
                            "wait for=element timed out ({timeout_ms}ms) in '{}'",
                            t.title
                        ));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                }
            }
            session
                .wait_for_element(by, appears, timeout_ms)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "for": "element" }).to_string())
        }
        "value" => {
            let by = locator(args)?;
            let pred = args["pred"].as_str().unwrap_or("equals");
            let expected = args["expected"].as_str().unwrap_or("");
            if let Some(t) = resolve_target(session, args).await {
                return with_temporary_raise(session, &t.title, || async {
                    let got = session
                        .wait_for_value(by.clone(), pred, expected, timeout_ms)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({
                        "ok": true,
                        "for": "value",
                        "value": got,
                        "target": t.to_json(),
                    })
                    .to_string())
                })
                .await;
            }
            let got = session
                .wait_for_value(by, pred, expected, timeout_ms)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "for": "value", "value": got }).to_string())
        }
        "idle" => {
            let window = resolve_target(session, args).await.map(|t| t.title);
            session
                .wait_for_idle(window.as_deref(), 3, timeout_ms)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "for": "idle" }).to_string())
        }
        "event" => {
            let since = args["since_seq"]
                .as_u64()
                .unwrap_or_else(|| session.event_seq());
            match session.wait_for_event(since, timeout_ms).await {
                Ok(seq) => Ok(json!({ "seq": seq, "timed_out": false }).to_string()),
                Err(_) => Ok(json!({ "seq": session.event_seq(), "timed_out": true }).to_string()),
            }
        }
        _ => {
            let ms = args["ms"]
                .as_u64()
                .or_else(|| args["seconds"].as_f64().map(|s| (s * 1000.0) as u64))
                .unwrap_or(500);
            let ok = session.wait(ms).await;
            Ok(json!({ "ok": ok, "for": "ms", "ms": ms }).to_string())
        }
    }
}

async fn assert_tool(session: &GhostSession, args: &Value) -> Result<String, String> {
    let kind = args["kind"].as_str().unwrap_or("exists");
    match kind {
        "exists" => {
            let by = locator(args)?;
            if let Some(t) = resolve_target(session, args).await {
                session
                    .find_background(&t.title, by, None)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(json!({
                    "ok": true,
                    "kind": "exists",
                    "target": t.to_json(),
                    "dispatch": "background",
                })
                .to_string());
            }
            session.find(by).await.map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "kind": "exists" }).to_string())
        }
        "value" => {
            let by = locator(args)?;
            let expected = args["expected"].as_str().unwrap_or("");
            if let Some(t) = resolve_target(session, args).await {
                return with_temporary_raise(session, &t.title, || async {
                    let got = session.get_text(by.clone()).await.map_err(|e| e.to_string())?;
                    if got != expected {
                        return Err(format!("assert value: expected {expected:?}, got {got:?}"));
                    }
                    Ok(json!({
                        "ok": true,
                        "kind": "value",
                        "value": got,
                        "target": t.to_json(),
                    })
                    .to_string())
                })
                .await;
            }
            let got = session.get_text(by).await.map_err(|e| e.to_string())?;
            if got != expected {
                return Err(format!("assert value: expected {expected:?}, got {got:?}"));
            }
            Ok(json!({ "ok": true, "kind": "value", "value": got }).to_string())
        }
        other => Err(format!("assert: unknown kind `{other}` (exists|value)")),
    }
}

async fn window(session: &GhostSession, args: &Value) -> Result<String, String> {
    let op = args["op"].as_str().unwrap_or("list");
    match op {
        "list" => {
            let wins = session.list_windows().await.map_err(|e| e.to_string())?;
            let list: Vec<_> = wins
                .iter()
                .map(|w| {
                    json!({
                        "name": w.name,
                        "hwnd": w.hwnd,
                        "pid": w.pid,
                        "focused": w.focused,
                        "state": w.state,
                    })
                })
                .collect();
            let anchor = session.live_anchor().await.map(|t| t.to_json());
            Ok(json!({ "windows": list, "anchor": anchor }).to_string())
        }
        "focus" | "anchor" => {
            let name = args["name"].as_str().ok_or("window focus/anchor needs name")?;
            let t = session
                .resolve_target(Some(name))
                .await
                .map_err(|e| e.to_string())?;
            let want_raise = op == "focus" && args["raise"].as_bool().unwrap_or(false);
            let mut raised = false;
            let mut focus_error: Option<String> = None;
            if want_raise {
                match with_temporary_raise(session, &t.title, || async { Ok(()) }).await {
                    Ok(()) => raised = true,
                    Err(e) => focus_error = Some(e),
                }
            }
            Ok(json!({
                "ok": true,
                "op": op,
                "anchored": true,
                "raised": raised,
                "locked": session.focus_locked(),
                "policy": session.focus_policy(),
                "focus_error": focus_error,
                "target": t.to_json(),
                "note": if raised {
                    "Raised briefly. Policy restored to background — act/key stay window-scoped."
                } else if op == "focus" && !want_raise {
                    "Anchored only (background policy, like Ghost MCP). Pass raise=true only if you must steal focus. Prefer act with window=title."
                } else {
                    "Anchored. Use act/see/key with this window — do NOT set GHOST_FOCUS_LOCK."
                },
            })
            .to_string())
        }
        "launch" => {
            let exe = args["exe"]
                .as_str()
                .or_else(|| args["target"].as_str())
                .or_else(|| args["path"].as_str())
                .ok_or("window launch needs exe")?;
            let cmdline = match args["args"].as_str() {
                Some(a) if !a.is_empty() => format!("{exe} {a}"),
                _ => exe.to_string(),
            };
            let pid = session
                .launch(&cmdline)
                .await
                .map_err(|e| e.to_string())?;
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let tip = std::path::Path::new(exe)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(exe);
            let target = session.resolve_target(Some(tip)).await.ok();
            Ok(json!({
                "ok": true,
                "exe": exe,
                "pid": pid,
                "target": target.as_ref().map(|t| t.to_json()),
                "note": "Launched on your desktop and anchored when a window appeared. Continue with see/act using window=title.",
            })
            .to_string())
        }
        "state" => {
            let name = args["name"].as_str().ok_or("window state needs name")?;
            let state = args["state"].as_str().unwrap_or("restore");
            session
                .window_state(name, state)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "name": name, "state": state }).to_string())
        }
        "clear_anchor" => {
            session.clear_anchor();
            Ok(json!({ "ok": true, "anchor": null }).to_string())
        }
        "policy" => {
            // Canvas / games: policy=prefer_background or foreground. Always unlock first.
            let policy = args["policy"]
                .as_str()
                .or_else(|| args["name"].as_str())
                .unwrap_or("background");
            ghost_session::engine::focus::set_lock(false);
            session
                .set_focus_policy(policy)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "ok": true,
                "policy": session.focus_policy(),
                "locked": session.focus_locked(),
                "note": "Default is background. Use prefer_background/foreground only for canvas (Paint brush) or games; drag already auto-raises.",
            })
            .to_string())
        }
        other => Err(format!(
            "window: unknown op `{other}` (list|focus|anchor|launch|state|clear_anchor|policy)"
        )),
    }
}

async fn key(session: &GhostSession, args: &Value) -> Result<String, String> {
    let keys = args["keys"]
        .as_str()
        .or_else(|| args["key"].as_str())
        .ok_or("key needs keys")?;
    let (mods, key_name) = parse_key_combo(keys)?;
    let scoped = resolve_target(session, args).await;

    if let Some(t) = scoped {
        if mods.is_empty() {
            let mut v = session
                .key_background(&t.title, &key_name)
                .await
                .map_err(|e| e.to_string())?;
            if let Some(obj) = v.as_object_mut() {
                obj.insert("target".into(), t.to_json());
                obj.insert("dispatch".into(), json!("background"));
            }
            return Ok(v.to_string());
        }
        let ctrl_only = mods.len() == 1
            && matches!(mods[0].to_ascii_lowercase().as_str(), "ctrl" | "control");
        if ctrl_only {
            if let Some(cmd) = ghost_session::EditCommand::from_ctrl_key(&key_name) {
                let cmd_name = match cmd {
                    ghost_session::EditCommand::Copy => "copy",
                    ghost_session::EditCommand::Cut => "cut",
                    ghost_session::EditCommand::Paste => "paste",
                    ghost_session::EditCommand::Undo => "undo",
                    ghost_session::EditCommand::SelectAll => "select_all",
                };
                let mut v = session
                    .edit_command_background(&t.title, cmd_name)
                    .await
                    .map_err(|e| e.to_string())?;
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("target".into(), t.to_json());
                    obj.insert("dispatch".into(), json!("background"));
                    obj.insert("keys".into(), json!(keys));
                }
                return Ok(v.to_string());
            }
        }
        let title = t.title.clone();
        let target_json = t.to_json();
        let keys_owned = keys.to_string();
        return with_temporary_raise(session, &title, || async {
            if mods.is_empty() {
                session
                    .press(&key_name)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let mod_refs: Vec<&str> = mods.iter().map(|s| s.as_str()).collect();
                session
                    .hotkey(&mod_refs, &key_name)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok(json!({
                "ok": true,
                "keys": keys_owned,
                "dispatch": "foreground_raise",
                "target": target_json,
            })
            .to_string())
        })
        .await;
    }

    if mods.is_empty() {
        session
            .press(&key_name)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        let mod_refs: Vec<&str> = mods.iter().map(|s| s.as_str()).collect();
        session
            .hotkey(&mod_refs, &key_name)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(json!({
        "ok": true,
        "keys": keys,
        "target_note": "no window anchored — keys went to the human foreground.",
    })
    .to_string())
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
        return Err(format!("key: malformed {keys:?} — missing key after modifiers"));
    }
    if modifiers.iter().any(|m| m.is_empty()) {
        return Err(format!("key: malformed {keys:?} — empty modifier segment"));
    }
    Ok((
        modifiers.into_iter().map(String::from).collect(),
        key.to_string(),
    ))
}

async fn scroll(session: &GhostSession, args: &Value) -> Result<String, String> {
    let (direction, amount) = if let Some(d) = args["direction"].as_str() {
        let amount = args["amount"].as_i64().unwrap_or(3) as i32;
        (d.to_string(), amount)
    } else if let Some(dy) = args["dy"].as_i64() {
        if dy >= 0 {
            ("down".into(), dy.max(1) as i32)
        } else {
            ("up".into(), (-dy).max(1) as i32)
        }
    } else if let Some(dx) = args["dx"].as_i64() {
        if dx >= 0 {
            ("right".into(), dx.max(1) as i32)
        } else {
            ("left".into(), (-dx).max(1) as i32)
        }
    } else {
        ("down".into(), args["amount"].as_i64().unwrap_or(3) as i32)
    };

    if let Some(t) = resolve_target(session, args).await {
        if args.get("until_name").is_some() || args.get("until_role").is_some() {
            let by = if let Some(n) = args["until_name"].as_str() {
                By::name(n)
            } else {
                By::role(args["until_role"].as_str().unwrap_or(""))
            };
            let max_scrolls = args["max_scrolls"].as_u64().unwrap_or(20).min(100);
            for _ in 0..max_scrolls {
                if session
                    .find_background(&t.title, by.clone(), None)
                    .await
                    .is_ok()
                {
                    return Ok(json!({
                        "ok": true,
                        "found": true,
                        "dispatch": "background",
                        "target": t.to_json(),
                    })
                    .to_string());
                }
                session
                    .scroll_background(&t, &direction, amount)
                    .await
                    .map_err(|e| e.to_string())?;
                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            }
            let found = session
                .find_background(&t.title, by, None)
                .await
                .is_ok();
            return Ok(json!({
                "ok": found,
                "found": found,
                "dispatch": "background",
                "target": t.to_json(),
            })
            .to_string());
        }
        let v = session
            .scroll_background(&t, &direction, amount)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(v.to_string());
    }

    let x = args["x"].as_i64().unwrap_or(0) as i32;
    let y = args["y"].as_i64().unwrap_or(0) as i32;
    session
        .scroll(x, y, &direction, amount)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "direction": direction,
        "amount": amount,
        "target_note": "no window anchored — scrolled the human foreground.",
    })
    .to_string())
}

async fn drag(session: &GhostSession, args: &Value) -> Result<String, String> {
    let fx = args["from_x"]
        .as_i64()
        .or_else(|| args["x"].as_i64())
        .ok_or("drag needs from_x")? as i32;
    let fy = args["from_y"]
        .as_i64()
        .or_else(|| args["y"].as_i64())
        .ok_or("drag needs from_y")? as i32;
    let tx = args["to_x"]
        .as_i64()
        .or_else(|| args["x2"].as_i64())
        .ok_or("drag needs to_x")? as i32;
    let ty = args["to_y"]
        .as_i64()
        .or_else(|| args["y2"].as_i64())
        .ok_or("drag needs to_y")? as i32;

    // Drag is SendInput-only (Paint brush, selections). No background path exists —
    // briefly raise the anchored/window= target, stroke, restore background.
    let t = resolve_target(session, args).await.ok_or_else(|| {
        "drag needs window= or a session anchor (launch/anchor first). Canvas strokes use real mouse on that window.".to_string()
    })?;
    let title = t.title.clone();
    let target_json = t.to_json();
    with_temporary_raise(session, &title, || async {
        session
            .drag(fx, fy, tx, ty)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({
            "ok": true,
            "from": { "x": fx, "y": fy },
            "to": { "x": tx, "y": ty },
            "dispatch": "foreground_raise",
            "target": target_json,
            "note": "Real mouse drag on the raised target; background policy restored.",
        })
        .to_string())
    })
    .await
}

async fn clipboard(session: &GhostSession, args: &Value) -> Result<String, String> {
    let op = args["op"].as_str().unwrap_or("get");
    match op {
        "get" => {
            let text = session.get_clipboard().await.map_err(|e| e.to_string())?;
            Ok(json!({ "text": text }).to_string())
        }
        "set" => {
            let text = args["text"].as_str().unwrap_or("");
            session
                .set_clipboard(text)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }).to_string())
        }
        "paste" => {
            let text = args["text"].as_str().unwrap_or("");
            session.paste_text(text).await.map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }).to_string())
        }
        other => Err(format!("clipboard: unknown op `{other}` (get|set|paste)")),
    }
}

async fn screenshot(
    session: &GhostSession,
    plugin_id: &str,
    args: &Value,
) -> Result<String, String> {
    let _suspend = crate::services::computer_use_hud::CaptureSuspend::enter();
    let scope = args["scope"].as_str().unwrap_or("foreground");
    let png = if let Some(title) = args["title"].as_str().or_else(|| args["window"].as_str()) {
        let t = session
            .resolve_target(Some(title))
            .await
            .map_err(|e| e.to_string())?;
        session
            .screenshot_window(t.hwnd, None, Some(1280), Some(85))
            .await
            .map_err(|e| e.to_string())?
    } else if scope == "desktop" {
        session
            .screenshot_region(None, Some(1280), Some(85))
            .await
            .map_err(|e| e.to_string())?
    } else if let Some(anchor) = session.live_anchor().await {
        session
            .screenshot_window(anchor.hwnd, None, Some(1280), Some(85))
            .await
            .map_err(|e| e.to_string())?
    } else {
        session
            .screenshot(Region::full())
            .await
            .map_err(|e| e.to_string())?
    };
    let dir = plugin_dir(plugin_id)
        .map_err(|e| e.to_string())?
        .join("data")
        .join("computer");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("shot-{now_ms}.png"));
    fs::write(&path, &png).map_err(|e| e.to_string())?;
    let href = path.to_string_lossy().replace('\\', "/");
    let tree = see(
        session,
        &json!({
            "mode": "full",
            "window": args["title"].as_str().or_else(|| args["window"].as_str()),
            "limit": 40
        }),
    )
    .await
    .unwrap_or_default();
    Ok(format!(
        "Screenshot saved. Prefer act by name/role from the control list; pixel click only for canvas.\n![image](path:{href})\n{tree}"
    ))
}

async fn screen_info(session: &GhostSession) -> Result<String, String> {
    let wins = session.list_windows().await.map_err(|e| e.to_string())?;
    Ok(json!({
        "windows": wins.len(),
        "policy": session.focus_policy(),
        "locked": session.focus_locked(),
        "note": "Anya unlocks Ghost in-process and defaults to background (Ghost MCP parity). Window-scoped act/key/scroll stay on the anchored app — do not setx GHOST_FOCUS_LOCK. policy=background is normal.",
    })
    .to_string())
}

async fn click_at(session: &GhostSession, args: &Value) -> Result<String, String> {
    let x = args["x"].as_i64().ok_or("click needs x")? as i32;
    let y = args["y"].as_i64().ok_or("click needs y")? as i32;
    // Canvas / games ignore posted WM_*: pass real=true (or input=real) for SendInput.
    let want_real = args["real"].as_bool().unwrap_or(false)
        || args["input"]
            .as_str()
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "real" | "foreground" | "sendinput"));

    if let Some(t) = resolve_target(session, args).await {
        if want_real {
            let title = t.title.clone();
            let target_json = t.to_json();
            return with_temporary_raise(session, &title, || async {
                session
                    .click_at(x, y)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({
                    "ok": true,
                    "x": x,
                    "y": y,
                    "dispatch": "foreground_raise",
                    "target": target_json,
                    "note": "Real mouse click (canvas/paint). Prefer drag for brush strokes.",
                })
                .to_string())
            })
            .await;
        }
        let hwnd = (t.hwnd != 0).then_some(t.hwnd);
        let mut v = session
            .click_at_background(x, y, hwnd)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(obj) = v.as_object_mut() {
            obj.insert("target".into(), t.to_json());
            obj.insert("dispatch".into(), json!("background"));
            obj.insert(
                "hint".into(),
                json!("Posted click — canvas apps like Paint often ignore this. Use drag or click real=true for brush marks."),
            );
        }
        return Ok(v.to_string());
    }

    if want_real {
        return Err(
            "click real=true needs window= or a session anchor so input lands on the right app"
                .into(),
        );
    }
    session
        .click_at(x, y)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "x": x,
        "y": y,
        "target_note": "no window anchored — click went to the human foreground.",
    })
    .to_string())
}

async fn browser(session: &GhostSession, args: &Value) -> Result<String, String> {
    let op = args["op"].as_str().unwrap_or("launch");
    let id = args["id"].as_str().unwrap_or("default");
    match op {
        "launch" => {
            let mode = args["mode"].as_str().unwrap_or("windowed");
            let which = args["which"].as_str();
            let v = session
                .browser_launch_with(id, mode, which)
                .await
                .map_err(|e| e.to_string())?;
            Ok(v.to_string())
        }
        "attach" => {
            let port = args["port"].as_u64().ok_or("browser attach needs port")? as u16;
            let v = session
                .browser_attach(id, port)
                .await
                .map_err(|e| e.to_string())?;
            Ok(v.to_string())
        }
        "tabs" => {
            let tabs = session
                .browser_tabs(id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&tabs).unwrap_or_default())
        }
        "close" => {
            session
                .browser_close(id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }).to_string())
        }
        other => Err(format!(
            "browser: unknown op `{other}` (launch|attach|tabs|close)"
        )),
    }
}

async fn tab(session: &GhostSession, args: &Value) -> Result<String, String> {
    let op = args["op"].as_str().unwrap_or("list");
    let browser_id = args["browser"].as_str().unwrap_or("default");
    match op {
        "open" => {
            let url = args["url"].as_str().unwrap_or("about:blank");
            let tid = session
                .tab_open(browser_id, url)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "target_id": tid, "url": url }).to_string())
        }
        "close" => {
            let tid = args["target_id"].as_str().ok_or("tab close needs target_id")?;
            session
                .tab_close(browser_id, tid)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }).to_string())
        }
        "find" => {
            let needle = args["needle"].as_str().unwrap_or("");
            let info = session
                .tab_find(browser_id, needle)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&info).unwrap_or_default())
        }
        "navigate" | "click" | "type" | "text" | "eval" | "wait" => {
            let tid = args["target_id"].as_str().ok_or("tab op needs target_id")?;
            let tab = session
                .tab(browser_id, tid)
                .await
                .map_err(|e| e.to_string())?;
            match op {
                "navigate" => {
                    let url = args["url"].as_str().ok_or("tab navigate needs url")?;
                    let timeout = args["timeout_ms"].as_u64().unwrap_or(30_000);
                    tab.navigate(url, timeout)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "ok": true, "url": url }).to_string())
                }
                "click" => {
                    let sel = args["selector"].as_str().ok_or("tab click needs selector")?;
                    let timeout = args["timeout_ms"].as_u64().unwrap_or(10_000);
                    tab.click(sel, timeout).await.map_err(|e| e.to_string())?;
                    Ok(json!({ "ok": true, "selector": sel }).to_string())
                }
                "type" => {
                    let sel = args["selector"].as_str().ok_or("tab type needs selector")?;
                    let text = args["text"].as_str().unwrap_or("");
                    let clear = args["clear"].as_bool().unwrap_or(true);
                    tab.type_text(sel, text, clear)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "ok": true }).to_string())
                }
                "text" => {
                    let sel = args["selector"].as_str().unwrap_or("");
                    let text = tab.text(sel).await.map_err(|e| e.to_string())?;
                    Ok(json!({ "text": text }).to_string())
                }
                "eval" => {
                    let expr = args["expression"]
                        .as_str()
                        .or_else(|| args["js"].as_str())
                        .ok_or("tab eval needs expression")?;
                    let v = tab.eval(expr).await.map_err(|e| e.to_string())?;
                    Ok(v.to_string())
                }
                "wait" => {
                    let sel = args["selector"].as_str().ok_or("tab wait needs selector")?;
                    let timeout = args["timeout_ms"].as_u64().unwrap_or(15_000);
                    tab.wait_for_selector(sel, timeout)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "ok": true }).to_string())
                }
                _ => unreachable!(),
            }
        }
        other => Err(format!(
            "tab: unknown op `{other}` (open|close|find|navigate|click|type|text|eval|wait)"
        )),
    }
}
