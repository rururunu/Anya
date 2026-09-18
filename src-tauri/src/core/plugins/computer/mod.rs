//! Computer-use tools: GhostSession engine behind Anya approval + HUD.

use serde_json::{json, Value};

use super::grant::grant_has;
use crate::core::tools::error::ToolError;

mod playbook;

#[cfg(windows)]
mod ghost_bridge;
#[cfg(windows)]
mod ghost_cdp;
#[cfg(windows)]
mod ghost_ops;
#[cfg(windows)]
mod ghost_scope;

#[cfg(not(windows))]
const WINDOWS_ONLY: &str = "computer use is currently Windows-only";

const TOOLS: &[&str] = &[
    // Ghost-style primary surface
    "see",
    "find",
    "act",
    "wait",
    "assert",
    "window",
    "key",
    "hotkey",
    "scroll",
    "drag",
    "clipboard",
    "screenshot",
    "screen_info",
    "browser",
    "tab",
    "playbook",
    // Thin aliases (old sessions / prompts)
    "list_windows",
    "focus_window",
    "find_control",
    "click_control",
    "set_value",
    "launch",
    "click",
    "type",
    "move",
    "hover",
];

/// True when this agent tool name should go through tool approval.
pub fn requires_tool_approval(full_name: &str) -> bool {
    if !full_name.starts_with("plugin_computer-use__") {
        return false;
    }
    match local_name(full_name) {
        "see" | "find" | "assert" | "wait" | "screenshot" | "screen_info" | "list_windows"
        | "find_control" | "playbook" => false,
        "act" | "window" | "key" | "hotkey" | "scroll" | "drag" | "clipboard" | "browser"
        | "tab" | "focus_window" | "click_control" | "set_value" | "launch" | "click"
        | "type" | "move" | "hover" => true,
        other => TOOLS.contains(&other),
    }
}

pub fn is_read_only_tool(local_name: &str) -> bool {
    matches!(
        local_name,
        "see"
            | "find"
            | "assert"
            | "wait"
            | "screenshot"
            | "screen_info"
            | "list_windows"
            | "find_control"
            | "playbook"
    )
}

/// Agent S2-style index of learned procedures for the system prompt.
pub fn playbook_prompt_catalog() -> Option<String> {
    playbook::prompt_catalog()
}

pub use playbook::{
    delete_for_ui as delete_computer_playbook, get_for_ui as get_computer_playbook,
    list_for_ui as list_computer_playbooks, Playbook as ComputerPlaybook,
};

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
    if action == "playbook" {
        return playbook::dispatch(args);
    }
    crate::services::computer_use_hud::open();
    #[cfg(windows)]
    {
        ghost_bridge::call(plugin_id, action, args)
    }
    #[cfg(not(windows))]
    {
        let _ = (plugin_id, action, args);
        Err(ToolError::new(WINDOWS_ONLY))
    }
}

/// Stop Ghost automation (HUD end / chat finished / plugin disable).
pub fn stop_engine() {
    #[cfg(windows)]
    ghost_bridge::stop_session();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_covers_mutating_actions() {
        assert!(!requires_tool_approval("plugin_computer-use__see"));
        assert!(!requires_tool_approval("plugin_computer-use__assert"));
        assert!(!requires_tool_approval("plugin_computer-use__wait"));
        assert!(!requires_tool_approval("plugin_computer-use__screenshot"));
        assert!(!requires_tool_approval("plugin_computer-use__list_windows"));
        assert!(!requires_tool_approval("plugin_computer-use__find_control"));
        assert!(!requires_tool_approval("plugin_computer-use__playbook"));
        assert!(requires_tool_approval("plugin_computer-use__act"));
        assert!(requires_tool_approval("plugin_computer-use__window"));
        assert!(requires_tool_approval("plugin_computer-use__key"));
        assert!(requires_tool_approval("plugin_computer-use__browser"));
        assert!(requires_tool_approval("plugin_computer-use__tab"));
        assert!(requires_tool_approval("plugin_computer-use__scroll"));
        assert!(requires_tool_approval("plugin_computer-use__drag"));
        assert!(requires_tool_approval("plugin_computer-use__click_control"));
        assert!(requires_tool_approval("plugin_computer-use__launch"));
        assert!(requires_tool_approval("plugin_computer-use__set_value"));
        assert!(!requires_tool_approval("plugin_terminal__run"));
        assert!(is_read_only_tool("see"));
        assert!(is_read_only_tool("wait"));
        assert!(!is_read_only_tool("act"));
        assert!(!is_read_only_tool("launch"));
    }

    #[test]
    fn unknown_tool_is_not_claimed() {
        assert!(try_execute_tool("computer-use", "run", &json!({})).is_none());
        assert!(try_execute_tool("computer-use", "see", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "act", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "window", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "browser", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "tab", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "playbook", &json!({ "op": "list" })).is_some());
        assert!(try_execute_tool("computer-use", "list_windows", &json!({})).is_some());
        assert!(try_execute_tool("computer-use", "launch", &json!({})).is_some());
    }
}

#[cfg(all(test, windows))]
mod ghost_smoke {
    use super::ghost_bridge;

    /// Manual / CI opt-in: `cargo test -p peek ghost_notepad_smoke -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn ghost_notepad_smoke() {
        let list = ghost_bridge::call("computer-use", "window", &serde_json::json!({ "op": "list" }))
            .expect("list windows");
        assert!(list.contains("Windows:"), "got: {list}");

        let launched = ghost_bridge::call(
            "computer-use",
            "window",
            &serde_json::json!({ "op": "launch", "exe": "notepad" }),
        )
        .expect("launch notepad");
        assert!(launched.contains("pid") || launched.contains("ok"), "got: {launched}");

        std::thread::sleep(std::time::Duration::from_millis(800));

        let _ = ghost_bridge::call(
            "computer-use",
            "window",
            &serde_json::json!({ "op": "anchor", "name": "Notepad" }),
        );

        let typed = ghost_bridge::call(
            "computer-use",
            "act",
            &serde_json::json!({
                "action": "type",
                "role": "edit",
                "window": "Notepad",
                "text_input": "ghost-smoke-ok",
            }),
        );
        // Soft assert: UIA name/role can vary by locale; bridge must not panic.
        let _ = typed;

        ghost_bridge::stop_session();
    }
}
