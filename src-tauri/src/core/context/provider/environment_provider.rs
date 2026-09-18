use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use crate::core::context::platform::WindowDetector;
use crate::core::runtime::RequestContext;
use crate::runtime::terminal::prepare_command;

static LAST_SHELL_EXECUTION: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn shell_store() -> &'static Mutex<Option<String>> {
    LAST_SHELL_EXECUTION.get_or_init(|| Mutex::new(None))
}

/// Enrich a request using independent, best-effort environment providers.
pub fn collect(context: &mut RequestContext) {
    collect_fast(context);
    collect_deferred(context, None);
}

/// Overlay show path: in-memory only. No git subprocess and no Office COM.
pub fn collect_fast(context: &mut RequestContext) {
    if context.active_window.is_none() {
        context.active_window = foreground_window();
    }
    context.last_shell_execution = last_shell_execution();
}

/// After the overlay is visible: git status and optional Office COM for a known process.
pub fn collect_deferred(context: &mut RequestContext, process_name: Option<&str>) {
    context.git_status = context
        .workspace
        .as_ref()
        .and_then(|workspace| git_status(Path::new(&workspace.root)));
    let office = match process_name {
        Some(process) => crate::core::office::collect_office_context_for_process(process),
        None => crate::core::office::collect_office_context(),
    };
    if let Some(office) = office {
        crate::core::office::enrich_request_context(context, office);
    }
}

pub fn record_shell_execution(command: &str, cwd: Option<&Path>, result: &str) {
    let cwd = cwd
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<default>".to_string());
    let value = format!("Command: {command}\nWorking Directory: {cwd}\nResult:\n{result}");
    match shell_store().lock() {
        Ok(mut stored) => *stored = Some(value),
        Err(error) => {
            tracing::warn!(provider = "last_shell_execution", error = %error, "context provider failed")
        }
    }
}

fn last_shell_execution() -> Option<String> {
    match shell_store().lock() {
        Ok(value) => value.clone(),
        Err(error) => {
            tracing::warn!(provider = "last_shell_execution", error = %error, "context provider failed");
            None
        }
    }
}

fn foreground_window() -> Option<String> {
    let window = match WindowDetector::detect() {
        Ok(window) => window,
        Err(error) => {
            tracing::warn!(provider = "foreground_window", error = %error, "context provider failed");
            return None;
        }
    };
    let title = window.title.trim();
    Some(if title.is_empty() {
        format!("{} (pid {})", window.process_name, window.pid)
    } else {
        format!("{} - {} (pid {})", window.process_name, title, window.pid)
    })
}

fn git_status(root: &Path) -> Option<String> {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "core.quotepath=false",
            "status",
            "--short",
            "--branch",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    prepare_command(&mut command);
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(provider = "git_status", error = %error, "context provider failed");
            return None;
        }
    };
    if !output.status.success() {
        return None;
    }
    let status = crate::runtime::encoding::decode_process_bytes(&output.stdout)
        .trim()
        .to_string();
    (!status.is_empty()).then_some(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_result_round_trips_through_optional_provider() {
        record_shell_execution(
            "Write-Output ok",
            Some(Path::new("C:\\work")),
            "exit_code: 0\nok",
        );
        let value = last_shell_execution().expect("shell result");
        assert!(value.contains("Write-Output ok"));
        assert!(value.contains("exit_code: 0"));
    }

    #[test]
    fn git_provider_ignores_non_repository() {
        assert_eq!(git_status(&std::env::temp_dir()), None);
    }
}
