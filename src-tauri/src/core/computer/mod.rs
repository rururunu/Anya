//! Anya's built-in computer tools, backed by a managed Ghost MCP process.

use crate::core::tools::registry::ToolRegistry;

#[cfg(windows)]
mod ghost_bridge;
#[cfg(windows)]
mod ghost_ops;
#[cfg(windows)]
mod ghost_process;
#[cfg(windows)]
mod ghost_release;
mod playbook;
#[cfg(windows)]
mod run;

pub fn requires_tool_approval(name: &str) -> bool {
    let Some(action) = name.strip_prefix("computer_") else {
        return false;
    };
    !read_only(action)
}

fn read_only(action: &str) -> bool {
    matches!(
        action,
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

pub fn register_tools(registry: &mut ToolRegistry) {
    #[cfg(windows)]
    {
        let tools: Vec<serde_json::Value> = serde_json::from_str(include_str!("tools.json"))
            .expect("built-in computer tool schemas");
        for schema in tools {
            let action = schema["name"].as_str().expect("computer tool name");
            registry.register(std::sync::Arc::new(ComputerTool {
                name: format!("computer_{action}"),
                schema,
            }));
        }
    }
    #[cfg(not(windows))]
    let _ = registry;
}

#[cfg(windows)]
struct ComputerTool {
    name: String,
    schema: serde_json::Value,
}

#[cfg(windows)]
impl crate::core::tools::Tool for ComputerTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        self.schema["description"].as_str().unwrap_or_default()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.schema["parameters"].clone()
    }

    fn read_only(&self) -> bool {
        read_only(&self.schema["name"].as_str().unwrap_or_default())
    }

    fn execute(
        &self,
        ctx: &crate::core::tools::context::ToolContext,
        args: serde_json::Value,
    ) -> Result<String, crate::core::tools::error::ToolError> {
        ctx.ensure_not_cancelled()?;
        let action = self.schema["name"].as_str().expect("computer tool name");
        if action == "playbook" {
            return playbook::dispatch(&args);
        }
        run::execute(action, &args, Some(ctx))
    }
}

pub fn prompt_context() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    let mut prompt = String::from(concat!(
        "For Windows apps, use computer_window to select a target, computer_see or ",
        "computer_find to locate controls, and computer_act to click or type. Keep ",
        "the window anchored in the current turn. Confirm the result with verified, ",
        "computer_wait, or computer_assert before retrying an action. Use screenshots ",
        "and pointer coordinates for canvas content. Use computer_browser and computer_tab ",
        "for a browser session. Consult computer_playbook before repeating learned workflows."
    ));
    if let Some(catalog) = playbook::prompt_catalog() {
        prompt.push_str("\n\n");
        prompt.push_str(&catalog);
    }
    Some(prompt)
}

pub fn finish_computer_turn(message_id: &str) {
    #[cfg(windows)]
    run::stop(Some(message_id));
    #[cfg(not(windows))]
    let _ = message_id;
}

pub fn stop_computer_turn() -> Option<String> {
    #[cfg(windows)]
    return run::stop(None);
    #[cfg(not(windows))]
    None
}

pub(crate) fn shutdown() {
    #[cfg(windows)]
    {
        run::stop(None);
        ghost_bridge::stop_session(true);
    }
}
