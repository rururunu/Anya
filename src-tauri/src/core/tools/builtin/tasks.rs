//! Task list and user-interaction builtin tools.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::context::{AskQuestion, TaskItem, Tool, ToolContext};
use crate::core::tools::error::ToolError;

pub(super) struct UpdateTasksTool {
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for UpdateTasksTool {
    fn name(&self) -> &str {
        "update_tasks"
    }
    fn description(&self) -> &str {
        "Maintain the in-session task checklist. Call before multi-step work. Each item needs content + status (pending|in_progress|completed|cancelled). Keep exactly one in_progress; mark completed as you finish; skip for trivial one-step work."
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
        {
            let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
            *guard = parsed.clone();
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: ctx.root_session_id().to_string(),
            tasks: parsed,
        });
        Ok("updated".into())
    }
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
        "Mark a plan step complete with evidence. Available in Plan mode; call when a step is done so the plan UI can advance."
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
        let step = args["step"].as_str().unwrap_or("");
        let evidence = args["evidence"].as_str().unwrap_or("");
        if evidence.trim().is_empty() {
            return Err(ToolError::new("evidence is required"));
        }
        let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
        for task in guard.iter_mut() {
            if task.content == step {
                task.status = "completed".into();
            }
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: ctx.root_session_id().to_string(),
            tasks: guard.clone(),
        });
        Ok(format!("completed step with evidence: {evidence}"))
    }
}
