use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::challenge::extract_paths_from_args;

use crate::core::chat::agent_loop::types::ToolOutcome;
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

/// Pending edits are drained at the completion boundary, not after every patch.
/// A new edit invalidates verification even if the path was checked earlier.
#[derive(Default)]
pub struct VerificationQueue {
    paths: BTreeSet<String>,
    completed_checks: BTreeSet<(PathBuf, String)>,
}

impl VerificationQueue {
    pub fn record(&mut self, outcomes: &[ToolOutcome], workspace: &Path) {
        for outcome in outcomes.iter().filter(|o| o.success) {
            if is_mutation_tool(&outcome.tool_name) {
                let changed: BTreeSet<_> = extract_paths_from_args(&outcome.arguments)
                    .into_iter()
                    .collect();
                for check in verification_plan(workspace, &changed) {
                    self.completed_checks.retain(|(dir, _)| dir != &check.cwd);
                }
                self.paths.extend(changed);
            } else if matches!(
                outcome.tool_name.as_str(),
                "run_subagent" | "run_parallel_subagents" | "run_skill"
            ) {
                self.paths.extend(
                    crate::core::tools::agent::extract_touched_paths_from_subagent_result(
                        &outcome.result,
                    ),
                );
                self.completed_checks.clear();
            } else if outcome.tool_name == "run_shell" && shell_exit_code_ok(&outcome.result) {
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&outcome.arguments) {
                    if let Some(command) = args["command"].as_str() {
                        for check in verification_plan(workspace, &self.paths) {
                            if check.cwd == workspace
                                && normalized_check(command) == normalized_check(&check.command)
                            {
                                self.completed_checks.insert((check.cwd, check.command));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn run_pending(&mut self, ctx: &ToolContext) -> VerificationReport {
        let paths = std::mem::take(&mut self.paths);
        if !auto_verify_enabled(ctx) || paths.is_empty() {
            return VerificationReport::default();
        }
        let mut checked = BTreeSet::new();
        let mut failed = BTreeSet::new();
        let outcomes = verification_plan(&ctx.workspace_root, &paths).into_iter().filter_map(|check| {
            if self.completed_checks.contains(&(check.cwd.clone(), check.command.clone())) {
                checked.insert(check.cwd);
                return None;
            }
            let (result, success) = match run_foreground(&check.command, Some(&check.cwd), &ctx.cancelled, Some(ctx)) {
                Ok(result) => {
                    let success = shell_exit_code_ok(&result);
                    (result, success)
                }
                Err(error) => (format!("auto verification failed: {error}"), false),
            };
            checked.insert(check.cwd.clone());
            if !success { failed.insert(check.cwd.clone()); }
            Some(ToolOutcome {
                call_id: format!("auto-verify-{}", uuid::Uuid::new_v4()),
                tool_name: "run_shell".into(),
                arguments: serde_json::json!({"command": check.command, "cwd": check.cwd, "description": "Verify affected project"}).to_string(),
                result,
                success,
                user_denied: false,
            })
        }).collect();
        let verified_paths = paths
            .into_iter()
            .filter(|path| {
                let required =
                    verification_plan(&ctx.workspace_root, &BTreeSet::from([path.clone()]));
                !required.is_empty()
                    && required
                        .iter()
                        .all(|check| checked.contains(&check.cwd) && !failed.contains(&check.cwd))
            })
            .collect();
        VerificationReport {
            outcomes,
            verified_paths,
        }
    }
}

#[derive(Default)]
pub struct VerificationReport {
    pub outcomes: Vec<ToolOutcome>,
    pub verified_paths: Vec<String>,
}

fn normalized_check(command: &str) -> String {
    command
        .split_whitespace()
        .filter(|word| !matches!(*word, "-q" | "--quiet" | "-s" | "--silent" | "run"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shell tools run in workspace_root. A check only proves something about the
/// project it actually targeted, not every same-language file in a monorepo.
pub(crate) fn check_targets_path(workspace: &Path, command: &str, path: &str) -> bool {
    let required = verification_plan(workspace, &BTreeSet::from([path.to_string()]));
    if required.is_empty() {
        return false;
    }
    let target = command
        .split_once("--manifest-path ")
        .and_then(|(_, tail)| {
            let tail = tail.trim();
            if let Some(rest) = tail.strip_prefix('\'') {
                rest.split_once('\'').map(|(path, _)| path)
            } else if let Some(rest) = tail.strip_prefix('"') {
                rest.split_once('"').map(|(path, _)| path)
            } else {
                tail.split_whitespace().next()
            }
        })
        .map(|manifest| workspace.join(manifest))
        .and_then(|manifest| manifest.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| workspace.to_path_buf());
    required.iter().any(|check| check.cwd == target)
}

#[derive(Debug, PartialEq, Eq)]
struct VerificationCommand {
    cwd: PathBuf,
    command: String,
}

pub(crate) fn needs_executable_check(path: &str) -> bool {
    !matches!(
        Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "md" | "txt" | "rst" | "adoc" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
    )
}

fn verification_plan(workspace: &Path, paths: &BTreeSet<String>) -> Vec<VerificationCommand> {
    let mut projects = BTreeSet::new();
    for path in paths.iter().filter(|p| needs_executable_check(p)) {
        let path = PathBuf::from(path.replace('\\', "/"));
        let absolute = if path.is_absolute() {
            path
        } else {
            workspace.join(path)
        };
        // Do not turn an out-of-workspace edit into an implicit command there.
        if !absolute.starts_with(workspace)
            || absolute
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            continue;
        }
        for dir in absolute.parent().into_iter().flat_map(Path::ancestors) {
            if !dir.starts_with(workspace) {
                break;
            }
            let rust = dir.join("Cargo.toml").is_file();
            let js = dir.join("package.json").is_file();
            let python = dir.join("pyproject.toml").is_file() || dir.join("pytest.ini").is_file();
            if rust || js || python {
                // A mixed project needs both checks; nested manifests take precedence.
                if rust {
                    projects.insert((dir.to_path_buf(), "rust"));
                }
                if js {
                    projects.insert((dir.to_path_buf(), "js"));
                }
                if python {
                    projects.insert((dir.to_path_buf(), "python"));
                }
                break;
            }
        }
    }
    let mut commands = Vec::new();
    for (cwd, kind) in projects {
        let checks: Vec<String> = match kind {
            "rust" => vec!["cargo check -q".into(), "cargo test -q".into()],
            "python" => vec!["python -m pytest -q".into()],
            _ => javascript_checks(&cwd),
        };
        commands.extend(checks.into_iter().map(|command| VerificationCommand {
            cwd: cwd.clone(),
            command,
        }));
    }
    commands
}

fn javascript_checks(dir: &Path) -> Vec<String> {
    let Some(value) = std::fs::read_to_string(dir.join("package.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let manager = if dir.ancestors().any(|p| p.join("pnpm-lock.yaml").is_file()) {
        "pnpm"
    } else if dir.ancestors().any(|p| p.join("yarn.lock").is_file()) {
        "yarn"
    } else if dir
        .ancestors()
        .any(|p| p.join("bun.lock").is_file() || p.join("bun.lockb").is_file())
    {
        "bun"
    } else {
        "npm"
    };
    if let Some(check) = scripts.get("check").and_then(|value| value.as_str()) {
        let mut checks = vec![format!("{manager} run check")];
        if scripts.contains_key("test")
            && ![" test", "vitest", "jest", "--test"]
                .iter()
                .any(|marker| check.contains(marker))
        {
            checks.push(format!("{manager} run test"));
        }
        return checks;
    }
    ["typecheck", "lint", "test"]
        .into_iter()
        .filter(|name| scripts.contains_key(*name))
        .map(|name| format!("{manager} run {name}"))
        .collect()
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
    fn passing_manual_check_is_cached_until_the_next_edit() {
        let root = std::env::temp_dir().join(format!("anya-check-cache-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"check":"tsc --noEmit"}}"#,
        )
        .unwrap();
        let mut queue = VerificationQueue::default();
        let edit = || ToolOutcome {
            call_id: "edit".into(),
            tool_name: "write_file".into(),
            arguments: r#"{"path":"a.ts"}"#.into(),
            result: "ok".into(),
            success: true,
            user_denied: false,
        };
        queue.record(&[edit()], &root);
        queue.record(
            &[ToolOutcome {
                call_id: "check".into(),
                tool_name: "run_shell".into(),
                arguments: r#"{"command":"npm run check"}"#.into(),
                result: "exit_code: 0\npassed".into(),
                success: true,
                user_denied: false,
            }],
            &root,
        );
        assert_eq!(queue.completed_checks.len(), 1);
        queue.record(&[edit()], &root);
        assert!(queue.completed_checks.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mixed_workspace_checks_only_affected_projects_and_respects_package_manager() {
        let root = std::env::temp_dir().join(format!("anya-verify-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src-tauri/src")).unwrap();
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"check":"vue-tsc","test":"vitest run"}}"#,
        )
        .unwrap();
        std::fs::write(root.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(root.join("src-tauri/Cargo.toml"), "").unwrap();
        let rust = verification_plan(&root, &BTreeSet::from(["src-tauri/src/lib.rs".into()]));
        assert_eq!(rust.len(), 2);
        assert!(rust
            .iter()
            .all(|c| c.cwd == root.join("src-tauri") && c.command.starts_with("cargo ")));
        let both = verification_plan(
            &root,
            &BTreeSet::from(["src-tauri/src/lib.rs".into(), "src/App.vue".into()]),
        );
        assert_eq!(both.len(), 4);
        assert!(both
            .iter()
            .any(|c| c.command == "pnpm run check" && c.cwd == root));
        assert!(verification_plan(&root, &BTreeSet::from(["README.md".into()])).is_empty());
        assert!(verification_plan(&root, &BTreeSet::from(["../escape.rs".into()])).is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pending_edits_coalesce_and_patch_moves_include_destination() {
        let mut queue = VerificationQueue::default();
        let outcome = ToolOutcome {
            call_id: "1".into(),
            tool_name: "apply_patch".into(),
            arguments: serde_json::json!({"input":"*** Update File: a.rs\n*** Move to: b.rs\n"})
                .to_string(),
            result: "ok".into(),
            success: true,
            user_denied: false,
        };
        queue.record(&[outcome], &std::env::temp_dir());
        assert_eq!(queue.paths, BTreeSet::from(["a.rs".into(), "b.rs".into()]));
    }

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
        assert!(shell_exit_code_ok(
            "status: done\nexit_code: Some(0)\nelapsed: 1s"
        ));
    }
}
