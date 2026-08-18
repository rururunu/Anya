//! Shell job builtin tools.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::shell_jobs::{background_allowed, run_foreground, ShellJobStore};

pub(super) struct RunShellTool {
    pub jobs: Arc<ShellJobStore>,
}

impl Tool for RunShellTool {
    fn name(&self) -> &str {
        "run_shell"
    }
    fn description(&self) -> &str {
        "Run a PowerShell command in the project workspace directory. Prefer dedicated file/search/git tools when they cover the task.\n\nUsage:\n- Always pass a short description (3-8 words) for the UI header.\n- Keep finite commands (Git, file reads, status checks, tests, builds, Docker inspection) in the foreground. Long builds are fine there: a command that keeps making progress is allowed to run to completion, and the result reports how long it took, so never estimate elapsed time yourself.\n- A command is only cut short when it hits the absolute ceiling, or when it makes no progress at all (no output, no CPU) and completion cannot be confirmed; both cases say so explicitly in the result.\n- Background mode only for persistent processes: Get-Content -Wait, log following, watchers, dev servers, or foreground container services — never to avoid waiting.\n- Logs: prefer bounded reads (Get-Content -Tail N, docker logs --tail N). Use follow mode only when live monitoring is requested, then background + read_shell_output.\n- Docker: start with read-only inspection (docker ps / compose ps / inspect / bounded logs). Build/start/restart/stop only when required. Never run destructive cleanup (docker system prune, compose down -v, volume/image deletion) without explicit user authorization.\n- When rtk is installed, use it to compact large output and fall back to the native command when needed."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "description": {
                    "type": "string",
                    "description": "Short human-readable label for the UI (e.g. 'Run unit tests'). Prefer 3–8 words."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Only for persistent processes (follow/watch/dev server/foreground service). Finite commands are forced to foreground."
                }
            },
            "required": ["command"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let command = args["command"].as_str().unwrap_or("");
        crate::core::tools::sandbox::reject_workspace_escape_writes(
            command,
            Some(&ctx.workspace_root),
        )?;
        if args["run_in_background"].as_bool().unwrap_or(false) && background_allowed(command) {
            return self.jobs.spawn_background(
                command.to_string(),
                Some(&ctx.workspace_root),
                Arc::clone(&ctx.cancelled),
                Some(ctx.clone()),
            );
        }
        run_foreground(
            command,
            Some(&ctx.workspace_root),
            &ctx.cancelled,
            Some(ctx),
        )
    }
}

pub(super) struct ReadShellOutputTool {
    pub jobs: Arc<ShellJobStore>,
}

impl Tool for ReadShellOutputTool {
    fn name(&self) -> &str {
        "read_shell_output"
    }
    fn description(&self) -> &str {
        "Read the latest stdout/stderr from a background shell job while it runs or after it exits. Prefer bounded tail_lines/max_chars — search or tail large logs instead of loading them whole. Stop the job with stop_shell when live monitoring is no longer needed."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string" },
                "tail_lines": { "type": "integer", "minimum": 1, "description": "Return only the last N lines" },
                "max_chars": { "type": "integer", "minimum": 1, "description": "Maximum output characters, taken from the tail" }
            },
            "required": ["job_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let job_id = args["job_id"].as_str().unwrap_or("");
        self.jobs.read_output_limited(
            job_id,
            args["tail_lines"].as_u64().map(|value| value as usize),
            args["max_chars"].as_u64().map(|value| value as usize),
        )
    }
}

pub(super) struct WaitForShellTool {
    pub jobs: Arc<ShellJobStore>,
}

impl Tool for WaitForShellTool {
    fn name(&self) -> &str {
        "wait_for_shell"
    }
    fn description(&self) -> &str {
        "Wait for a background shell job to finish. Prefer read_shell_output for incremental log checks while it is still running."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "job_id": { "type": "string" } },
            "required": ["job_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.jobs
            .wait_job(args["job_id"].as_str().unwrap_or(""), ctx)
    }
}

pub(super) struct StopShellTool {
    pub jobs: Arc<ShellJobStore>,
}

impl Tool for StopShellTool {
    fn name(&self) -> &str {
        "stop_shell"
    }
    fn description(&self) -> &str {
        "Stop a background shell job. Call when a persistent process (watcher, follow logs, dev server) is no longer needed."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "job_id": { "type": "string" } },
            "required": ["job_id"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.jobs.kill(args["job_id"].as_str().unwrap_or(""))
    }
}
