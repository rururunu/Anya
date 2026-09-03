use std::path::Path;

use crate::core::chat::agent_loop::types::{now_millis, ToolOutcome};
use crate::core::tools::context::ToolContext;
use crate::core::tools::shell_jobs::run_foreground;

/// File tools that count as a successful workspace mutation for auto-verify.
pub const MUTATION_TOOL_NAMES: &[&str] = &[
    "apply_patch",
    "write_file",
    "replace_in_file",
    "replace_many_in_file",
    "move_path",
    "delete_text_range",
    "delete_go_symbol",
    "edit_notebook_cell",
];

pub fn is_mutation_tool(name: &str) -> bool {
    MUTATION_TOOL_NAMES.contains(&name)
}

fn auto_verify_enabled(ctx: &ToolContext) -> bool {
    let Some(app) = &ctx.app_handle else {
        return true;
    };
    crate::services::settings_store::get_settings(app)
        .map(|s| s.auto_verify_after_edits)
        .unwrap_or(true)
}

fn has_successful_mutation(outcomes: &[ToolOutcome]) -> bool {
    outcomes
        .iter()
        .any(|o| o.success && is_mutation_tool(&o.tool_name))
}

fn detect_verify_command(workspace: &Path) -> Option<String> {
    let package_json = workspace.join("package.json");
    if package_json.exists() {
        let raw = std::fs::read_to_string(&package_json).ok()?;
        let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let scripts = value.get("scripts")?.as_object()?;
        if scripts.contains_key("check") {
            return Some("npm run -s check".to_string());
        }
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
    let cargo = workspace.join("Cargo.toml");
    if cargo.exists() {
        return Some("cargo check -q".to_string());
    }
    let nested_cargo = workspace.join("src-tauri").join("Cargo.toml");
    if nested_cargo.exists() {
        return Some("cargo check -q --manifest-path src-tauri/Cargo.toml".to_string());
    }
    if workspace.join("pyproject.toml").exists() || workspace.join("pytest.ini").exists() {
        return Some("python -m pytest -q".to_string());
    }
    None
}

/// Parse `exit_code: N` from shell job output. Missing / non-zero → not success.
pub fn shell_exit_code_ok(output: &str) -> bool {
    for line in output.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix("exit_code:")
            .or_else(|| trimmed.strip_prefix("exit_code: "))
        else {
            continue;
        };
        let code = rest.trim().trim_start_matches(':').trim();
        // Formats: `exit_code: 0`, `exit_code: Some(0)`, `exit_code: None`
        let digits: String = code
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        if let Ok(n) = digits.parse::<i32>() {
            return n == 0;
        }
        return false;
    }
    false
}

pub fn maybe_run_post_edit_verification(
    outcomes: &[ToolOutcome],
    ctx: &ToolContext,
) -> Option<ToolOutcome> {
    if !auto_verify_enabled(ctx) || !has_successful_mutation(outcomes) {
        return None;
    }
    let command = detect_verify_command(&ctx.workspace_root)?;
    let (result, ran_ok) = match run_foreground(
        &command,
        Some(&ctx.workspace_root),
        &ctx.cancelled,
        Some(ctx),
    ) {
        Ok(out) => {
            let ok = shell_exit_code_ok(&out);
            (out, ok)
        }
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
        result,
        success: ran_ok,
        user_denied: false,
    })
}

/// Feedback for the next model round. Must not be a `role=tool` message:
/// the verify call_id was never in the assistant `tool_calls`, and DeepSeek
/// rejects that with 400 ("Messages with role 'tool' must be a response to
/// a preceding message with 'tool_calls'").
pub fn verify_feedback_content(outcome: &ToolOutcome) -> String {
    let command = serde_json::from_str::<serde_json::Value>(&outcome.arguments)
        .ok()
        .and_then(|value| {
            value
                .get("command")
                .and_then(|v| v.as_str())
                .filter(|command| !command.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| outcome.tool_name.clone());
    format!(
        "[System] Automatic post-edit verification (`{command}`):\n{}",
        outcome.result
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::agent_loop::types::ToolOutcome;

    #[test]
    fn verify_feedback_is_system_user_text_not_tool_role() {
        let outcome = ToolOutcome {
            call_id: "auto-verify-1".into(),
            tool_name: "run_shell".into(),
            arguments: serde_json::json!({
                "command": "cargo check -q",
                "description": "Auto verify edits"
            })
            .to_string(),
            result: "exit_code: 0\nFinished `dev` profile [unoptimized + debuginfo]".into(),
            success: true,
            user_denied: false,
        };
        let content = verify_feedback_content(&outcome);
        assert!(content.starts_with("[System] Automatic post-edit verification (`cargo check -q`)"));
        assert!(content.contains("Finished `dev` profile"));
    }

    #[test]
    fn shell_exit_code_ok_requires_zero() {
        assert!(shell_exit_code_ok("exit_code: 0\nok"));
        assert!(!shell_exit_code_ok("exit_code: 1\nerror"));
        assert!(!shell_exit_code_ok("no exit line here"));
        assert!(shell_exit_code_ok("status: done\nexit_code: Some(0)\nelapsed: 1s"));
    }
}
