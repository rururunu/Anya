//! Agent asks the user to switch into plan mode; never flips the gate itself.

use serde_json::{json, Value};

use crate::core::event::{BusEvent, EventBus, PlanModeSource};
use crate::core::tools::context::{AskOption, AskQuestion, Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::plan_mode::shared_plan_mode_store;

pub const PLAN_SWITCH_KIND: &str = "planSwitch";
pub const PLAN_SWITCH_ACCEPT_LABEL: &str = "切换到计划";
pub const PLAN_SWITCH_DECLINE_LABEL: &str = "继续用 Agent";
const DECLINE_RESULT: &str = "User declined Plan mode. Stay in Agent mode and continue this turn. Do not call request_plan_mode again. Do not write a plan or call save_plan. Implement the original user request now with Agent tools and finish the current work.";

pub(super) struct RequestPlanModeTool {
    pub event_bus: std::sync::Arc<dyn EventBus>,
}

impl Tool for RequestPlanModeTool {
    fn name(&self) -> &str {
        "request_plan_mode"
    }

    fn description(&self) -> &str {
        "Ask the user to switch this session from Agent to Plan mode. Call this when the work is large, multi-file, or has open design choices and a reviewed plan is safer than implementing now. Do not use it as a formality for small, well-specified edits — just implement those in Agent. Waits for the user's choice. If they accept, Plan mode starts immediately this turn (writers blocked; save_plan then update_tasks, then stop). If they decline, stay in Agent, do not ask again, and finish the current work."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Short reason shown to the user (why Plan mode is better than implementing now)."
                }
            },
            "required": ["reason"]
        })
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let session_id = ctx.root_session_id();
        if shared_plan_mode_store().is_active(session_id) {
            return Ok(
                "Plan mode is already active. Continue with read-only inspection, save_plan, and update_tasks; do not implement yet."
                    .into(),
            );
        }

        let reason = args["reason"].as_str().unwrap_or("").trim();
        let question = if reason.is_empty() {
            "这项工作更适合先出方案再改。是否切换到计划模式？".to_string()
        } else {
            format!("{reason}\n\n是否切换到计划模式？")
        };

        let questions = vec![AskQuestion {
            header: PLAN_SWITCH_ACCEPT_LABEL.into(),
            question,
            options: vec![
                AskOption {
                    label: PLAN_SWITCH_ACCEPT_LABEL.into(),
                    description: Some("先出方案，你批准后再写文件和跑 Shell".into()),
                },
                AskOption {
                    label: PLAN_SWITCH_DECLINE_LABEL.into(),
                    description: Some("保持 Agent，马上改并完成当前任务".into()),
                },
            ],
            multi_select: false,
            kind: Some(PLAN_SWITCH_KIND.into()),
        }];

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        ctx.ask_store.insert(
            request_id.clone(),
            session_id.to_string(),
            questions.clone(),
            tx,
        );
        self.event_bus.emit(BusEvent::AskUser {
            session_id: session_id.to_string(),
            request_id,
            questions,
        });

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        let answer = loop {
            ctx.ensure_not_cancelled()?;
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(answer) => break answer,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                    if std::time::Instant::now() < deadline => {}
                Err(_) => {
                    return Err(ToolError::new(
                        "plan mode request timed out or disconnected",
                    ))
                }
            }
        };

        if !accepted_plan_switch(&answer) {
            return Ok(DECLINE_RESULT.into());
        }

        shared_plan_mode_store().set_active(session_id, true);
        self.event_bus.emit(BusEvent::PlanModeChanged {
            session_id: session_id.to_string(),
            active: true,
            source: PlanModeSource::Request,
        });
        Ok(
            "User accepted. Plan mode is now active this turn. Writer tools (including Shell) are blocked. Inspect with read-only tools if needed, call save_plan with a concrete proposal, then update_tasks with pending steps only, then stop and wait for Approve & execute."
                .into(),
        )
    }
}

fn accepted_plan_switch(raw: &str) -> bool {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if value["skipped"].as_bool() == Some(true) {
            return false;
        }
        return value["answers"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|item| item["selected"].as_array().into_iter().flatten())
            .filter_map(|item| item.as_str())
            .any(label_accepts_plan);
    }
    label_accepts_plan(raw)
}

fn label_accepts_plan(label: &str) -> bool {
    let trimmed = label.trim();
    trimmed.contains(PLAN_SWITCH_ACCEPT_LABEL)
        || trimmed.eq_ignore_ascii_case("Switch to Plan")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_payload_switches() {
        let raw = r#"{"skipped":false,"answers":[{"header":"切换到计划","question":"q","selected":["切换到计划"]}]}"#;
        assert!(accepted_plan_switch(raw));
    }

    #[test]
    fn decline_payload_stays_agent() {
        let raw = r#"{"skipped":false,"answers":[{"header":"切换到计划","question":"q","selected":["继续用 Agent"]}]}"#;
        assert!(!accepted_plan_switch(raw));
    }

    #[test]
    fn skipped_picker_stays_agent() {
        let raw = r#"{"skipped":true,"answers":[{"header":"切换到计划","question":"q","selected":[]}]}"#;
        assert!(!accepted_plan_switch(raw));
    }

    #[test]
    fn decline_result_tells_agent_to_finish_the_work() {
        assert!(DECLINE_RESULT.contains("Stay in Agent"));
        assert!(DECLINE_RESULT.contains("finish the current work"));
        assert!(DECLINE_RESULT.contains("Do not call request_plan_mode again"));
        assert!(!DECLINE_RESULT.contains(PLAN_SWITCH_ACCEPT_LABEL));
    }
}
