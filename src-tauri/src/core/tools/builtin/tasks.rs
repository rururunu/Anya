//! Task list and user-interaction builtin tools.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::context::{AskQuestion, TaskItem, Tool, ToolContext};
use crate::core::tools::error::ToolError;

const PLAN_MIN_STEPS: usize = 3;
const PLAN_MAX_STEPS: usize = 8;
const PLAN_MIN_STEP_CHARS: usize = 8;

pub(super) struct UpdateTasksTool {
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for UpdateTasksTool {
    fn name(&self) -> &str {
        "update_tasks"
    }
    fn description(&self) -> &str {
        "Maintain the in-session task checklist. Call before multi-step work. Each item needs content + status (pending|in_progress|completed|cancelled). Keep exactly one in_progress; mark completed as you finish; skip for trivial one-step work. In plan mode, submit 3–8 pending steps only."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string" },
                            "status": { "type": "string" },
                            "activeForm": { "type": "string" },
                            "level": { "type": "integer" }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["tasks"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let parsed: Vec<TaskItem> = serde_json::from_value(args["tasks"].clone())?;
        let plan_active = crate::core::tools::plan_mode::shared_plan_mode_store()
            .is_active(ctx.root_session_id());
        if plan_active {
            validate_plan_tasks(&parsed)?;
        }
        {
            let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
            *guard = parsed.clone();
        }
        let session_id = ctx.root_session_id();
        if parsed
            .iter()
            .any(|task| crate::core::tools::plan_mode::task_status_is_open(&task.status))
        {
            crate::core::tools::plan_mode::shared_plan_mode_store()
                .mark_awaiting_approval(session_id);
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: session_id.to_string(),
            tasks: parsed,
        });
        Ok("updated".into())
    }
}

fn validate_plan_tasks(tasks: &[TaskItem]) -> Result<(), ToolError> {
    if tasks.len() < PLAN_MIN_STEPS || tasks.len() > PLAN_MAX_STEPS {
        return Err(ToolError::new(format!(
            "plan mode requires {PLAN_MIN_STEPS}–{PLAN_MAX_STEPS} pending steps, got {}",
            tasks.len()
        )));
    }
    for task in tasks {
        if !matches!(
            task.status.trim().to_ascii_lowercase().as_str(),
            "pending" | ""
        ) {
            return Err(ToolError::new(
                "plan mode only accepts pending steps; do not mark completed while planning",
            ));
        }
        let content = task.content.trim();
        if content.chars().count() < PLAN_MIN_STEP_CHARS {
            return Err(ToolError::new(
                "each plan step must be concrete (at least 8 characters with an action and target)",
            ));
        }
        if !step_looks_concrete(content) {
            return Err(ToolError::new(format!(
                "plan step too vague: `{content}`. Include a verb and a path, command, or deliverable."
            )));
        }
    }
    Ok(())
}

fn step_looks_concrete(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let has_path = content.contains('/')
        || content.contains('\\')
        || content.contains('.')
            && content.chars().any(|c| c.is_ascii_alphanumeric());
    let has_check = [
        "test", "check", "verify", "typecheck", "lint", "build", "cargo", "pytest", "pnpm", "npm",
        "验证", "测试", "检查", "构建",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || content.contains(marker));
    let has_action = [
        "read",
        "write",
        "edit",
        "add",
        "update",
        "create",
        "implement",
        "fix",
        "run",
        "open",
        "move",
        "delete",
        "replace",
        "refactor",
        "inspect",
        "读",
        "写",
        "改",
        "加",
        "创建",
        "实现",
        "修复",
        "运行",
        "打开",
        "删除",
        "检查",
        "更新",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || content.contains(marker));
    (has_action && (has_path || has_check || content.split_whitespace().count() >= 3))
        || (has_path && content.split_whitespace().count() >= 2)
}

pub(super) struct AskUserTool {
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }
    fn description(&self) -> &str {
        "Ask the user a structured multiple-choice question and wait. Use for genuine user-owned decisions — UI style, approach, trade-offs — with 2-4 concrete options. Never substitute a plain-text option list in the chat reply when this tool is available. Do not use for routine confirmations."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "header": { "type": "string" },
                            "question": { "type": "string" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["label"]
                                }
                            },
                            "multiSelect": { "type": "boolean" }
                        },
                        "required": ["header", "question", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let questions: Vec<AskQuestion> = serde_json::from_value(args["questions"].clone())?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        ctx.ask_store.insert(
            request_id.clone(),
            ctx.root_session_id().to_string(),
            questions.clone(),
            tx,
        );
        self.event_bus.emit(BusEvent::AskUser {
            session_id: ctx.root_session_id().to_string(),
            request_id: request_id.clone(),
            questions,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        loop {
            ctx.ensure_not_cancelled()?;
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(answer) => return Ok(answer),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                    if std::time::Instant::now() < deadline => {}
                Err(_) => return Err(ToolError::new("ask_user timed out or disconnected")),
            }
        }
    }
}

pub(super) struct CompletePlanStepTool {
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for CompletePlanStepTool {
    fn name(&self) -> &str {
        "complete_plan_step"
    }
    fn description(&self) -> &str {
        "Mark a plan step complete with evidence (file path or check command). Only after plan approval during execution."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "step": { "type": "string" },
                "evidence": { "type": "string" }
            },
            "required": ["step", "evidence"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let session_id = ctx.root_session_id();
        if crate::core::tools::plan_mode::shared_plan_mode_store().is_active(session_id) {
            return Err(ToolError::new(
                "complete_plan_step is blocked while plan mode is active; wait for approval",
            ));
        }
        let step = args["step"].as_str().unwrap_or("").trim();
        let evidence = args["evidence"].as_str().unwrap_or("").trim();
        if evidence.is_empty() {
            return Err(ToolError::new("evidence is required"));
        }
        if !evidence_looks_valid(evidence) {
            return Err(ToolError::new(
                "evidence must cite a file path or a check/test/build command",
            ));
        }
        let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
        let mut matched = false;
        let step_norm = normalize_step(step);
        for task in guard.iter_mut() {
            let content_norm = normalize_step(&task.content);
            if content_norm == step_norm
                || content_norm.contains(&step_norm)
                || step_norm.contains(&content_norm)
            {
                task.status = "completed".into();
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(ToolError::new(format!(
                "no matching plan step for `{step}`"
            )));
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: session_id.to_string(),
            tasks: guard.clone(),
        });
        Ok(format!("completed step with evidence: {evidence}"))
    }
}

fn evidence_looks_valid(evidence: &str) -> bool {
    let lower = evidence.to_ascii_lowercase();
    evidence.contains('/')
        || evidence.contains('\\')
        || evidence.contains('.')
        || [
            "cargo",
            "npm",
            "pnpm",
            "pytest",
            "test",
            "check",
            "lint",
            "typecheck",
            "build",
            "git diff",
            "read",
            "验证",
            "测试",
            "检查",
        ]
        .iter()
        .any(|marker| lower.contains(marker) || evidence.contains(marker))
}

fn normalize_step(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_tasks_reject_completed_and_shallow() {
        let too_few = validate_plan_tasks(&[TaskItem {
            content: "do it now please".into(),
            status: "pending".into(),
            active_form: None,
            level: 0,
        }])
        .unwrap_err();
        assert!(too_few.to_string().contains("3–8"));

        let completed = validate_plan_tasks(&[
            TaskItem {
                content: "Read src/a.rs carefully".into(),
                status: "completed".into(),
                active_form: None,
                level: 0,
            },
            TaskItem {
                content: "Edit src/a.rs timeout".into(),
                status: "pending".into(),
                active_form: None,
                level: 0,
            },
            TaskItem {
                content: "Run cargo check to verify".into(),
                status: "pending".into(),
                active_form: None,
                level: 0,
            },
        ])
        .unwrap_err();
        assert!(completed.to_string().contains("pending"));
    }

    #[test]
    fn plan_tasks_accept_concrete_pending() {
        let tasks = vec![
            TaskItem {
                content: "Read src/main.rs for entry".into(),
                status: "pending".into(),
                active_form: None,
                level: 0,
            },
            TaskItem {
                content: "Edit src/main.rs timeout".into(),
                status: "pending".into(),
                active_form: None,
                level: 0,
            },
            TaskItem {
                content: "Run cargo check to verify".into(),
                status: "pending".into(),
                active_form: None,
                level: 0,
            },
        ];
        assert!(validate_plan_tasks(&tasks).is_ok());
    }

    #[test]
    fn evidence_requires_path_or_check() {
        assert!(evidence_looks_valid("src/main.rs"));
        assert!(evidence_looks_valid("cargo check -q"));
        assert!(!evidence_looks_valid("ok"));
    }
}
