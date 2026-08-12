use std::path::Path;

use crate::core::chat::agent_loop::types::{now_millis, ToolOutcome};
use crate::core::tools::context::ToolContext;
use crate::core::tools::shell_jobs::run_foreground;

fn auto_verify_enabled(ctx: &ToolContext) -> bool {
    let Some(app) = &ctx.app_handle else {
        return true;
    };
    crate::services::settings_store::get_settings(app)
        .map(|s| s.auto_verify_after_edits)
        .unwrap_or(true)
}

fn has_successful_mutation(outcomes: &[ToolOutcome]) -> bool {
    outcomes.iter().any(|o| {
        o.success
            && matches!(
                o.tool_name.as_str(),
                "apply_patch"
                    | "write_file"
                    | "replace_in_file"
                    | "replace_many_in_file"
                    | "move_path"
                    | "delete_text_range"
                    | "delete_go_symbol"
                    | "edit_notebook_cell"
            )
    })
}

fn detect_verify_command(workspace: &Path) -> Option<String> {
    let cargo = workspace.join("Cargo.toml");
    if cargo.exists() {
        return Some("cargo check -q".to_string());
    }
    let package_json = workspace.join("package.json");
    if package_json.exists() {
        let raw = std::fs::read_to_string(&package_json).ok()?;
        let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let scripts = value.get("scripts")?.as_object()?;
        if scripts.contains_key("typecheck") {
            return Some("npm run -s typecheck".to_string());
        }
        if scripts.contains_key("lint") {
            return Some("npm run -s lint".to_string());
        }
        if scripts.contains_key("test") {
            return Some("npm test --silent".to_string());
        }
    }
    if workspace.join("pyproject.toml").exists() || workspace.join("pytest.ini").exists() {
        return Some("python -m pytest -q".to_string());
    }
    None
}

pub fn maybe_run_post_edit_verification(
    outcomes: &[ToolOutcome],
    ctx: &ToolContext,
) -> Option<ToolOutcome> {
    if !auto_verify_enabled(ctx) || !has_successful_mutation(outcomes) {
        return None;
    }
    let command = detect_verify_command(&ctx.workspace_root)?;
    let result = match run_foreground(&command, Some(&ctx.workspace_root), &ctx.cancelled, Some(ctx)) {
        Ok(out) => (out, true),
        Err(err) => (format!("auto verification failed: {err}"), false),
    };
    Some(ToolOutcome {
        call_id: format!("auto-verify-{}", now_millis()),
        tool_name: "run_shell".to_string(),
        arguments: serde_json::json!({
            "command": command,
            "description": "Auto verify edits"
        })
        .to_string(),
        result: result.0,
        success: result.1,
        user_denied: false,
    })
}

