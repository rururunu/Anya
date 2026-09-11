//! Desktop computer-use primitives for plugins granted `computer`.
//! Screenshot, mouse, and keyboard stay in Anya; plugins only call them.

use serde_json::{json, Value};

use super::grant::grant_has;
use crate::core::tools::error::ToolError;

mod points;
#[cfg(windows)]
mod ops;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub(crate) const SETTLE_MS: u64 = 20;
#[cfg(not(windows))]
const WINDOWS_ONLY: &str = "computer use is currently Windows-only";

const TOOLS: &[&str] = &[
    "screenshot",
    "click",
    "drag",
    "move",
    "hover",
    "scroll",
    "wait",
    "type",
    "key",
    "screen_info",
    "list_windows",
    "focus_window",
    "find_control",
    "click_control",
    "set_value",
    "launch",
];

/// True when this agent tool name should go through tool approval.
pub fn requires_tool_approval(full_name: &str) -> bool {
    matches!(
        local_name(full_name),
        "screenshot"
            | "click"
            | "drag"
            | "move"
            | "hover"
            | "scroll"
            | "type"
            | "key"
            | "focus_window"
            | "click_control"
            | "set_value"
            | "launch"
    )
}

pub fn is_read_only_tool(local_name: &str) -> bool {
    matches!(
        local_name,
        "screenshot" | "screen_info" | "list_windows" | "find_control" | "wait"
    )
}

fn local_name(full_name: &str) -> &str {
    full_name.rsplit("__").next().unwrap_or(full_name)
}

/// Host RPC `computer.*` (workbench `ctx.host.rpc`).
pub fn dispatch_rpc(plugin_id: &str, method: &str, params: &Value) -> Result<Value, ToolError> {
    if !grant_has(plugin_id, "computer") {
        return Err(ToolError::new("plugin is not granted computer"));
    }
    let action = method.strip_prefix("computer.").unwrap_or(method);
    let text = execute(plugin_id, action, params)?;
    Ok(json!({ "content": text }))
}

/// Agent plugin tools implemented here instead of Deno.
pub fn try_execute_tool(
    plugin_id: &str,
    local_name: &str,
    args: &Value,
) -> Option<Result<String, ToolError>> {
    if !TOOLS.contains(&local_name) {
        return None;
    }
    if !grant_has(plugin_id, "computer") {
        return Some(Err(ToolError::new("plugin is not granted computer")));
    }
    Some(execute(plugin_id, local_name, args))
}

fn execute(plugin_id: &str, action: &str, args: &Value) -> Result<String, ToolError> {
    #[cfg(windows)]
    {
        ops::execute(plugin_id, action, args)
    }
    #[cfg(not(windows))]
    {
        let _ = (plugin_id, action, args);
        Err(ToolError::new(WINDOWS_ONLY))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_defaults_to_foreground() {
        #[cfg(windows)]
        {
            use ops::ShotScope;
            assert!(matches!(
                ops::parse_shot_scope(&json!({})),
                ShotScope::Foreground
            ));
            assert!(matches!(
                ops::parse_shot_scope(&json!({ "scope": "desktop" })),
                ShotScope::Desktop
            ));
            match ops::parse_shot_scope(&json!({ "title": "Antenna" })) {
                ShotScope::Title(t) => assert_eq!(t, "Antenna"),
                _ => panic!("expected title"),
            }
        }
    }

    #[test]
    fn approval_covers_mutating_actions() {
        assert!(requires_tool_approval("plugin_computer-use__hover"));
        assert!(!requires_tool_approval("plugin_computer-use__wait"));
        assert!(is_read_only_tool("wait"));
        assert!(!is_read_only_tool("hover"));
        assert!(requires_tool_approval("plugin_computer-use__drag"));
        assert!(requires_tool_approval("plugin_computer-use__screenshot"));
        assert!(requires_tool_approval("plugin_computer-use__focus_window"));
        assert!(requires_tool_approval("plugin_computer-use__click_control"));
        assert!(requires_tool_approval("plugin_computer-use__launch"));
        assert!(requires_tool_approval("plugin_computer-use__set_value"));
        assert!(!requires_tool_approval("plugin_computer-use__screen_info"));
        assert!(!requires_tool_approval("plugin_computer-use__list_windows"));
        assert!(!requires_tool_approval("plugin_computer-use__find_control"));
        assert!(!requires_tool_approval("plugin_terminal__run"));
        assert!(is_read_only_tool("list_windows"));
        assert!(is_read_only_tool("find_control"));
        assert!(!is_read_only_tool("click_control"));
        assert!(!is_read_only_tool("launch"));
        assert!(!is_read_only_tool("drag"));
        assert!(!is_read_only_tool("focus_window"));
    }

    #[test]
    fn unknown_tool_is_not_claimed() {
        assert!(try_execute_tool("computer-use", "run", &json!({})).is_none());
        assert!(try_execute_tool("computer-use", "list_windows", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "find_control", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "launch", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "set_value", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "drag", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "hover", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "wait", &json!({})).is_some());
    }
}
