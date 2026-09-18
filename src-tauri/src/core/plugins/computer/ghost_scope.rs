//! Window prepare / focus helpers shared by Ghost verb dispatch (MCP parity).

use ghost_session::{GhostSession, WindowTarget};
use serde_json::{json, Value};

pub struct Prepared {
    pub args: Option<Value>,
    pub target: Option<WindowTarget>,
}

/// Explicit `window`/`title`, else live session anchor.
pub async fn resolve_target(session: &GhostSession, args: &Value) -> Option<WindowTarget> {
    let named = args["window"]
        .as_str()
        .or_else(|| args["title"].as_str())
        .filter(|s| !s.is_empty());
    if let Some(w) = named {
        session.resolve_target(Some(w)).await.ok()
    } else {
        session.live_anchor().await
    }
}

/// Ghost MCP `prepare_window_target`: rewrite window to exact title + `_target_hwnd`.
pub async fn prepare(session: &GhostSession, action: &str, args: &Value) -> Prepared {
    if !is_window_scoped(action, args) {
        return Prepared {
            args: None,
            target: None,
        };
    }
    let Some(t) = resolve_target(session, args).await else {
        return Prepared {
            args: None,
            target: None,
        };
    };
    let explicit = args
        .get("window")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("title").and_then(|v| v.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut new_args: Option<Value> = None;
    let mut set = |key: &str, v: Value| {
        let a = new_args.get_or_insert_with(|| args.clone());
        a[key] = v;
    };
    if !t.title.is_empty() && explicit.as_deref() != Some(t.title.as_str()) {
        set("window", Value::String(t.title.clone()));
    }
    if t.hwnd != 0 && !t.is_hidden() {
        set("_target_hwnd", json!(t.hwnd));
    }
    if action == "see" && args["mode"].as_str().unwrap_or("fast") == "fast" {
        set("mode", Value::String("full".into()));
    }
    Prepared {
        args: new_args,
        target: Some(t),
    }
}

fn is_window_scoped(action: &str, args: &Value) -> bool {
    match action {
        "see" | "find" | "find_control" | "findControl" | "act" | "key" | "hotkey"
        | "click" | "click_control" | "clickControl" | "set_value" | "setValue" | "type"
        | "scroll" | "drag" | "assert" | "screenshot" => true,
        "wait" => matches!(
            args["for"].as_str().unwrap_or("ms"),
            "element" | "value" | "idle" | "text"
        ),
        _ => false,
    }
}

/// Append did-you-mean names on element-not-found (Ghost MCP enrich_not_found).
pub async fn enrich_not_found(
    session: &GhostSession,
    msg: String,
    args: &Value,
    t: &WindowTarget,
    route: Option<&ghost_session::cdp_route::CdpRoute>,
) -> String {
    let lower = msg.to_lowercase();
    let is_miss = lower.contains("not found") || lower.contains("out of range");
    let query = args["name"]
        .as_str()
        .or_else(|| args["until_name"].as_str())
        .or_else(|| args["name_filter"].as_str());
    let (Some(query), true) = (query, is_miss) else {
        return msg;
    };
    let names: Vec<String> = if let Some(r) = route {
        session
            .cdp_describe(r, 400, None, None)
            .await
            .map(|els| els.into_iter().map(|e| e.name).collect())
            .unwrap_or_default()
    } else {
        session
            .describe_screen(Some(&t.title))
            .await
            .map(|els| els.into_iter().map(|e| e.name).collect())
            .unwrap_or_default()
    };
    match ghost_session::suggest::did_you_mean(query, names, 8) {
        Some(hint) => format!("{msg}. {hint}"),
        None => msg,
    }
}

pub fn needs_foreground_act_fallback(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("use foreground")
        || lower.contains("windowless")
        || lower.contains("needs a windowed control")
        || lower.contains("description targets need vision")
}

/// Temporarily unlock + raise + allow SendInput on a named window, then restore background.
pub async fn with_temporary_raise<T, F, Fut>(
    session: &GhostSession,
    title: &str,
    f: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    ghost_session::engine::focus::set_lock(false);
    let prev = session.focus_policy();
    session.set_focus_policy("foreground").map_err(|e| {
        format!(
            "could not switch to foreground policy ({e}). Anya should already unlock Ghost — check screen_info.locked; do not setx GHOST_FOCUS_LOCK."
        )
    })?;
    let raised = session.ensure_window_foreground(title).await;
    let out = match raised {
        Ok(()) => f().await,
        Err(e) => Err(format!(
            "need real input on '{title}' but could not raise it (Windows may refuse while Anya holds focus): {e}. Click the target app once, then retry."
        )),
    };
    let restore = if prev == "foreground" || prev == "prefer_background" {
        prev
    } else {
        "background"
    };
    let _ = session.set_focus_policy(restore);
    out
}

/// Run a window-scoped verb then hand back the human's foreground if the target stole it.
pub async fn with_focus_guard(
    session: &GhostSession,
    target: Option<&WindowTarget>,
    action: &str,
    run: impl std::future::Future<Output = Result<String, String>>,
) -> Result<String, String> {
    let guard = target
        .filter(|t| !t.is_hidden())
        .map(|t| (t.hwnd, t.pid, session.foreground_guard_begin()));
    let result = run.await;
    let Ok(body) = result else {
        return result;
    };
    let Some((hwnd, pid, fg_before)) = guard else {
        return Ok(body);
    };
    let settle = match action {
        "click" | "scroll" | "key" | "hotkey" => 100,
        _ => 0,
    };
    let g = session
        .foreground_guard_end_within(hwnd, pid, fg_before, 0, settle)
        .await;
    let Some(g) = g else {
        return Ok(body);
    };
    match serde_json::from_str::<Value>(&body) {
        Ok(mut v) => {
            if let Some(m) = v.as_object_mut() {
                m.entry("focus_guard").or_insert(g);
                m.insert("focus_preserved".into(), json!(false));
            }
            Ok(v.to_string())
        }
        Err(_) => Ok(body),
    }
}

pub fn attach_target_json(mut body: Value, target: Option<&WindowTarget>) -> Value {
    let Some(t) = target else {
        return body;
    };
    match body {
        Value::Object(ref mut m) => {
            m.entry("target".to_string())
                .or_insert_with(|| t.to_json());
            body
        }
        other => json!({ "result": other, "target": t.to_json() }),
    }
}
