//! Task list and user-interaction builtin tools.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::context::{AskQuestion, TaskItem, Tool, ToolContext};
use crate::core::tools::error::ToolError;

const PLAN_MIN_STEPS: usize = 1;
const PLAN_MAX_STEPS: usize = 12;
const PLAN_MIN_STEP_CHARS: usize = 4;

pub(super) struct UpdateTasksTool {
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for UpdateTasksTool {
    fn name(&self) -> &str {
        "update_tasks"
    }
    fn description(&self) -> &str {
        "Maintain the in-session task checklist. Call before multi-step work. Each item needs content + status (pending|in_progress|completed|cancelled). Keep exactly one in_progress; mark completed as you finish; skip for trivial one-step work. In plan mode, submit 1–12 pending steps only."
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
                            "id": { "type": "string", "description": "Stable requirement ID; reuse it when updating this requirement" },
                            "acceptance_criteria": { "type": "array", "items": { "type": "string" }, "description": "Concrete observable conditions that prove this requirement is done" },
                            "evidence": { "type": "array", "items": { "type": "string" }, "description": "Successful tool call IDs that support completion, from Agent task state" },
                            "reason": { "type": "string", "description": "Why a requirement was cancelled or is blocked" },
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
        let plan_store = crate::core::tools::plan_mode::shared_plan_mode_store();
        let plan_active = plan_store.is_active(ctx.root_session_id());
        if plan_active {
            validate_plan_tasks(&parsed)?;
            if !plan_store.has_saved_plan(ctx.root_session_id()) {
                return Err(ToolError::new(
                    crate::core::tools::plan_mode::PLAN_TASKS_WITHOUT_SAVED_PLAN,
                ));
            }
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
                "each plan step must be concrete (at least 4 characters with an action and target)",
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
        || content.contains('.') && content.chars().any(|c| c.is_ascii_alphanumeric());
    let has_check = [
        "test",
        "check",
        "verify",
        "typecheck",
        "lint",
        "build",
        "cargo",
        "pytest",
        "pnpm",
        "npm",
        "验证",
        "测试",
        "检查",
        "构建",
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
        "优化",
        "重构",
        "调整",
        "配置",
        "排查",
        "设计",
        "分析",
        "处理",
        "引入",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || content.contains(marker));
    let char_count = content.chars().count();
    (has_action
        && (has_path || has_check || content.split_whitespace().count() >= 3 || char_count >= 6))
        || (has_path && (content.split_whitespace().count() >= 2 || char_count >= 6))
        || char_count >= 12
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

fn extract_plan_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix('#') {
            let title = heading.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

fn slugify_plan_title(title: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        if c.is_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if (c == ' ' || c == '-' || c == '_' || c == '/') && !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "plan".to_string()
    } else {
        let chars: Vec<char> = trimmed.chars().take(40).collect();
        chars.into_iter().collect()
    }
}

pub(super) struct SavePlanTool {
    pub event_bus: Arc<dyn EventBus>,
}

impl Tool for SavePlanTool {
    fn name(&self) -> &str {
        "save_plan"
    }

    fn description(&self) -> &str {
        "Save a temporary markdown plan proposal before submitting task checklist. In plan mode, you must write the comprehensive implementation plan proposal here first. Provide a short semantic title summarizing the plan. The user will be able to review and preview this proposal in the sidebar before approving execution."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short semantic title or slug for the plan (e.g. 'fix-scroll-position' or '用户登录鉴权'). Used to name the plan file."
                },
                "content": {
                    "type": "string",
                    "description": "Full Markdown content of the implementation proposal, including background, risks/decisions, proposed changes grouped by component, and verification plan."
                },
                "path": {
                    "type": "string",
                    "description": "Optional explicit relative path for the plan file. Defaults to .anya/plans/<timestamp>-<slug>.md."
                }
            },
            "required": ["content"]
        })
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let session_id = ctx.root_session_id();
        if !crate::core::tools::plan_mode::shared_plan_mode_store().is_active(session_id) {
            return Err(ToolError::new(
                crate::core::tools::plan_mode::SAVE_PLAN_REQUIRES_PLAN_MODE,
            ));
        }

        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::new("`content` must be a string"))?
            .trim();

        if content.is_empty() {
            return Err(ToolError::new("plan content cannot be empty"));
        }

        let custom_path = args["path"]
            .as_str()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        let (clean_path, is_default_derived) = if let Some(p) = custom_path {
            let clean = p.trim_start_matches(['/', '\\']);
            if clean.contains("..") {
                return Err(ToolError::new("path cannot contain `..`"));
            }
            (clean.to_string(), false)
        } else {
            let title = args["title"]
                .as_str()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| extract_plan_title(content))
                .unwrap_or_else(|| "plan".to_string());
            let slug = slugify_plan_title(&title);
            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            (format!(".anya/plans/{timestamp}-{slug}.md"), true)
        };

        let full_path = if !ctx.workspace_root.as_os_str().is_empty() {
            ctx.workspace_root.join(&clean_path)
        } else {
            std::env::temp_dir().join(&clean_path)
        };

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::new(format!("failed to create plan directory: {e}")))?;
        }

        std::fs::write(&full_path, content)
            .map_err(|e| ToolError::new(format!("failed to write plan file: {e}")))?;

        if is_default_derived {
            let active_ptr = if !ctx.workspace_root.as_os_str().is_empty() {
                ctx.workspace_root.join(".anya").join("plan.md")
            } else {
                std::env::temp_dir().join(".anya").join("plan.md")
            };
            if let Some(parent) = active_ptr.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&active_ptr, content);
        }

        crate::core::tools::plan_mode::shared_plan_mode_store().mark_plan_saved(session_id);
        self.event_bus.emit(BusEvent::PlanUpdated {
            session_id: session_id.to_string(),
            path: clean_path.clone(),
            content: content.to_string(),
        });

        Ok(format!(
            "Saved plan proposal to `{clean_path}` ({} bytes). You may now call `update_tasks` with pending execution steps.",
            content.len()
        ))
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
        let too_few = validate_plan_tasks(&[]).unwrap_err();
        assert!(too_few.to_string().contains("1–12"));

        let too_short = validate_plan_tasks(&[TaskItem {
            content: "ok".into(),
            status: "pending".into(),
            active_form: None,
            level: 0,
        }])
        .unwrap_err();
        assert!(too_short.to_string().contains("at least 4 characters"));

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
                content: "优化滚动定位逻辑".into(),
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

    fn make_test_ctx(root: std::path::PathBuf) -> ToolContext {
        use crate::core::chat::conversation_manager::ConversationManager;
        use std::sync::atomic::AtomicBool;
        let db_path = std::env::temp_dir().join(format!("peek-test-{}.db", uuid::Uuid::new_v4()));
        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }
        ToolContext {
            workspace_root: root,
            request_context: Default::default(),
            session_id: "test-session".into(),
            assistant_message_id: "asst-1".into(),
            conversation: Arc::new(ConversationManager::new(db_path)),
            event_bus: Arc::new(NullBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(crate::core::tools::context::AskStore::new()),
            path_permission_store: Arc::new(crate::core::tools::context::PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 1,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn save_plan_validates_input_and_writes_file() {
        struct MockBus;
        impl EventBus for MockBus {
            fn emit(&self, _event: BusEvent) {}
        }
        let tool = SavePlanTool {
            event_bus: Arc::new(MockBus),
        };
        let tmp = std::env::temp_dir().join(format!("save-plan-test-{}", uuid::Uuid::new_v4()));
        let mut ctx = make_test_ctx(tmp.clone());
        ctx.session_id = format!("save-plan-ok-{}", uuid::Uuid::new_v4());
        crate::core::tools::plan_mode::shared_plan_mode_store().set_active(&ctx.session_id, true);

        let empty_args = json!({ "content": "   " });
        let err = tool.execute(&ctx, empty_args).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));

        let traversal_args = json!({
            "content": "# Test Plan",
            "path": "../evil.md"
        });
        let err2 = tool.execute(&ctx, traversal_args).unwrap_err();
        assert!(err2.to_string().contains("cannot contain `..`"));

        let ok_args = json!({
            "content": "# Implementation Plan\n\n- Step 1: Do something\n"
        });
        let res = tool.execute(&ctx, ok_args).unwrap();
        assert!(res.contains(".anya/plans/"));
        assert!(res.contains("Implementation-Plan.md"));
        assert!(tmp.join(".anya").join("plan.md").exists());

        let custom_title_args = json!({
            "title": "用户登录鉴权改造",
            "content": "实现登录鉴权\n"
        });
        let res2 = tool.execute(&ctx, custom_title_args).unwrap();
        assert!(res2.contains("用户登录鉴权改造.md"));

        let _ = std::fs::remove_dir_all(&tmp);
        crate::core::tools::plan_mode::shared_plan_mode_store().set_active(&ctx.session_id, false);
    }

    #[test]
    fn save_plan_blocked_when_plan_mode_is_off() {
        struct MockBus;
        impl EventBus for MockBus {
            fn emit(&self, _event: BusEvent) {}
        }
        let tool = SavePlanTool {
            event_bus: Arc::new(MockBus),
        };
        let tmp = std::env::temp_dir().join(format!("save-plan-exec-{}", uuid::Uuid::new_v4()));
        let mut ctx = make_test_ctx(tmp.clone());
        ctx.session_id = format!("save-plan-exec-{}", uuid::Uuid::new_v4());
        crate::core::tools::plan_mode::shared_plan_mode_store().set_active(&ctx.session_id, false);

        let err = tool
            .execute(
                &ctx,
                json!({ "content": "# Should not write during execution\n" }),
            )
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("only available while plan mode is active"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn update_tasks_in_plan_mode_requires_prior_save_plan() {
        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let tmp = std::env::temp_dir().join(format!("update-tasks-test-{}", uuid::Uuid::new_v4()));
        let mut ctx = make_test_ctx(tmp.clone());
        ctx.session_id = session_id.clone();

        let plan_store = crate::core::tools::plan_mode::shared_plan_mode_store();
        plan_store.set_active(&session_id, true);

        let tool = UpdateTasksTool {
            tasks: Arc::new(Mutex::new(Vec::new())),
            event_bus: Arc::new(NullBus),
        };
        let args = json!({
            "tasks": [
                { "content": "Read src/main.rs for entry", "status": "pending" },
                { "content": "Run cargo check to verify", "status": "pending" }
            ]
        });

        let err = tool.execute(&ctx, args.clone()).unwrap_err();
        assert!(err
            .to_string()
            .contains("plan mode requires save_plan before update_tasks"));

        let save_tool = SavePlanTool {
            event_bus: Arc::new(NullBus),
        };
        save_tool
            .execute(
                &ctx,
                json!({ "content": "# Implementation Plan\n\n- step" }),
            )
            .unwrap();

        assert!(tool.execute(&ctx, args).is_ok());

        plan_store.set_active(&session_id, false);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
