//! OpenCLI sidecar: site adapters + logged-in Chrome via Browser Bridge.

mod setup;

use serde_json::Value;

use super::grant::grant_has;
use crate::core::tools::error::ToolError;

pub use setup::{install_cli, probe_status, OpenCliInstallResult, OpenCliSetupStatus};

const PLUGIN_ID: &str = "opencli";
const DEFAULT_TIMEOUT_SECS: u64 = 90;
const MAX_TIMEOUT_SECS: u64 = 180;
const MAX_OUTPUT_CHARS: usize = 80_000;
const MAX_ARGS: usize = 48;

const TOOLS: &[&str] = &["doctor", "list", "run", "browser"];

/// True when this agent tool name should go through tool approval.
pub fn requires_tool_approval(full_name: &str) -> bool {
    if !full_name.starts_with("plugin_opencli__") {
        return false;
    }
    match local_name(full_name) {
        "doctor" | "list" => false,
        "run" | "browser" => true,
        other => TOOLS.contains(&other),
    }
}

pub fn is_read_only_tool(name: &str) -> bool {
    matches!(name, "doctor" | "list")
}

fn local_name(full_name: &str) -> &str {
    full_name.rsplit("__").next().unwrap_or(full_name)
}

pub fn try_execute_tool(
    plugin_id: &str,
    local_name: &str,
    args: &Value,
) -> Option<Result<String, ToolError>> {
    if plugin_id != PLUGIN_ID || !TOOLS.contains(&local_name) {
        return None;
    }
    if !grant_has(plugin_id, "opencli") {
        return Some(Err(ToolError::new(
            "plugin is not granted opencli (install OpenCLI + enable this plugin)",
        )));
    }
    Some(dispatch(local_name, args))
}

fn dispatch(action: &str, args: &Value) -> Result<String, ToolError> {
    match action {
        "doctor" => run_opencli(&["doctor".into()], timeout_secs(args)),
        "list" => {
            let mut cmd = vec!["list".to_string()];
            if args.get("json").and_then(|v| v.as_bool()).unwrap_or(true) {
                cmd.push("--json".into());
            }
            run_opencli(&cmd, timeout_secs(args))
        }
        "run" => {
            let argv = collect_argv(args)?;
            if argv.is_empty() {
                return Err(ToolError::new(
                    "run needs argv[] e.g. [\"bilibili\",\"hot\",\"--limit\",\"5\",\"--json\"]",
                ));
            }
            reject_dangerous(&argv)?;
            run_opencli(&argv, timeout_secs(args))
        }
        "browser" => {
            let session = args
                .get("session")
                .and_then(|v| v.as_str())
                .unwrap_or("work")
                .trim();
            if session.is_empty() || !is_safe_token(session) {
                return Err(ToolError::new(
                    "browser needs a simple session name (e.g. work)",
                ));
            }
            let op = args
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if op.is_empty() {
                return Err(ToolError::new(
                    "browser needs op (open|state|click|type|fill|select|keys|wait|get|find|extract|scroll|back|eval|screenshot|network|tab|close|…)",
                ));
            }
            let mut cmd = vec!["browser".into(), session.to_string()];
            if op.eq_ignore_ascii_case("tab") {
                let sub = args
                    .get("tab_op")
                    .or_else(|| args.get("sub"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("list");
                cmd.push("tab".into());
                cmd.push(sub.to_string());
                append_extra(&mut cmd, args)?;
            } else {
                cmd.push(op.to_string());
                if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                    cmd.push(url.to_string());
                }
                if let Some(sel) = args
                    .get("selector")
                    .or_else(|| args.get("target"))
                    .and_then(|v| v.as_str())
                {
                    cmd.push(sel.to_string());
                }
                if let Some(text) = args
                    .get("text")
                    .or_else(|| args.get("value"))
                    .and_then(|v| v.as_str())
                {
                    cmd.push(text.to_string());
                }
                if let Some(tab) = args.get("tab").and_then(|v| v.as_str()) {
                    cmd.push("--tab".into());
                    cmd.push(tab.to_string());
                }
                append_extra(&mut cmd, args)?;
            }
            reject_dangerous(&cmd)?;
            run_opencli(&cmd, timeout_secs(args))
        }
        other => Err(ToolError::new(format!("unknown opencli tool `{other}`"))),
    }
}

fn timeout_secs(args: &Value) -> u64 {
    args.get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(5, MAX_TIMEOUT_SECS)
}

fn collect_argv(args: &Value) -> Result<Vec<String>, ToolError> {
    if let Some(arr) = args.get("argv").and_then(|v| v.as_array()) {
        if arr.len() > MAX_ARGS {
            return Err(ToolError::new(format!("argv max {MAX_ARGS}")));
        }
        return Ok(arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect());
    }
    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
        let parts = shell_words_split(cmd);
        if parts.len() > MAX_ARGS {
            return Err(ToolError::new(format!("command max {MAX_ARGS} tokens")));
        }
        return Ok(parts);
    }
    Ok(Vec::new())
}

fn shell_words_split(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote: Option<char> = None;
    for ch in cmd.chars() {
        match in_quote {
            Some(q) if ch == q => in_quote = None,
            Some(_) => cur.push(ch),
            None if ch == '"' || ch == '\'' => in_quote = Some(ch),
            None if ch.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn append_extra(cmd: &mut Vec<String>, args: &Value) -> Result<(), ToolError> {
    if let Some(arr) = args.get("extra").and_then(|v| v.as_array()) {
        if cmd.len() + arr.len() > MAX_ARGS {
            return Err(ToolError::new(format!("too many args (max {MAX_ARGS})")));
        }
        for v in arr {
            if let Some(s) = v.as_str() {
                cmd.push(s.to_string());
            }
        }
    }
    Ok(())
}

fn is_safe_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn reject_dangerous(argv: &[String]) -> Result<(), ToolError> {
    for a in argv {
        if a.contains('\0') || a.contains('\n') || a.contains('\r') {
            return Err(ToolError::new("opencli args must not contain control chars"));
        }
        if a.starts_with('$') || a.starts_with('`') {
            return Err(ToolError::new("opencli args must not start with $ or `"));
        }
    }
    Ok(())
}

fn run_opencli(argv: &[String], timeout_secs: u64) -> Result<String, ToolError> {
    let bin = resolve_opencli_bin()?;
    match setup::run_capture_public(&bin, argv, timeout_secs) {
        Ok(out) => {
            let truncated = truncate(&out.text, MAX_OUTPUT_CHARS);
            if out.code == 0 {
                Ok(truncated)
            } else {
                Ok(format!("exit {}\n{truncated}", out.code))
            }
        }
        Err(e) => Err(ToolError::new(format!(
            "{e}. Install: npm i -g @jackwener/opencli && opencli doctor"
        ))),
    }
}

pub(super) fn resolve_opencli_bin() -> Result<String, ToolError> {
    if let Ok(p) = std::env::var("OPENCLI_BIN") {
        let t = p.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    if let Some(path) = setup::discover_opencli_bin() {
        return Ok(path);
    }
    Ok(if cfg!(windows) {
        "opencli.cmd".into()
    } else {
        "opencli".into()
    })
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("\n…[truncated]");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn approval_matrix() {
        assert!(!requires_tool_approval("plugin_opencli__doctor"));
        assert!(!requires_tool_approval("plugin_opencli__list"));
        assert!(requires_tool_approval("plugin_opencli__run"));
        assert!(requires_tool_approval("plugin_opencli__browser"));
        assert!(!requires_tool_approval("plugin_computer-use__act"));
        assert!(is_read_only_tool("doctor"));
        assert!(!is_read_only_tool("browser"));
    }

    #[test]
    fn splits_quoted_command() {
        let parts = shell_words_split(r#"bilibili hot --limit 5 --json"#);
        assert_eq!(parts, vec!["bilibili", "hot", "--limit", "5", "--json"]);
        let q = shell_words_split(r#"browser work open "https://example.com/a b""#);
        assert_eq!(q.last().unwrap(), "https://example.com/a b");
    }

    #[test]
    fn rejects_control_chars() {
        assert!(reject_dangerous(&["ok".into()]).is_ok());
        assert!(reject_dangerous(&["bad\n".into()]).is_err());
    }

    #[test]
    fn unknown_plugin_not_claimed() {
        assert!(try_execute_tool("computer-use", "doctor", &json!({})).is_none());
        assert!(try_execute_tool("opencli", "doctor", &json!({})).is_some());
    }
}
