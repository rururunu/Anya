use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::event::BusEvent;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::preview::ToolPreview;
use crate::models::settings::ToolApprovalMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalDecision {
    AllowOnce,
    Deny,
}

impl ApprovalDecision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "allow_once" => Some(Self::AllowOnce),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

struct PendingApproval {
    sequence: u64,
    sender: mpsc::Sender<ApprovalDecision>,
    session_id: String,
    request_id: String,
    tool_name: String,
    title: String,
    arguments: Value,
    preview: Option<ToolPreview>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingToolApprovalSnapshot {
    pub sequence: u64,
    pub request_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub title: String,
    pub arguments: Value,
    pub preview: Option<ToolPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ToolApprovalRequestPayload {
    pub session_id: String,
    pub request_id: String,
    pub tool_name: String,
    pub title: String,
    pub arguments: Value,
    pub preview: Option<ToolPreview>,
}

pub struct ToolApprovalStore {
    pending: Mutex<HashMap<String, PendingApproval>>,
    mode: Mutex<ToolApprovalMode>,
    session_modes: Mutex<HashMap<String, ToolApprovalMode>>,
    recorded_modes: Mutex<HashMap<String, ToolApprovalMode>>,
}

impl ToolApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            mode: Mutex::new(ToolApprovalMode::Ask),
            session_modes: Mutex::new(HashMap::new()),
            recorded_modes: Mutex::new(HashMap::new()),
        }
    }

    pub fn configure(&self, mode: ToolApprovalMode) {
        if let Ok(mut guard) = self.mode.lock() {
            *guard = mode;
        }
    }

    pub fn mode(&self) -> ToolApprovalMode {
        self.mode
            .lock()
            .map(|g| *g)
            .unwrap_or(ToolApprovalMode::Ask)
    }

    /// Register (Some) or clear (None) a per-conversation approval mode override.
    /// Each conversation keeps its own approval choice; a cleared entry falls
    /// back to the global mode.
    pub fn set_session_mode(&self, session_id: &str, mode: Option<ToolApprovalMode>) {
        let mut session_modes = match self.session_modes.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        match mode {
            Some(mode) => {
                session_modes.insert(session_id.to_string(), mode);
            }
            None => {
                session_modes.remove(session_id);
            }
        }
    }

    pub fn mode_for_session(&self, session_id: &str) -> ToolApprovalMode {
        self.session_modes
            .lock()
            .ok()
            .and_then(|modes| modes.get(session_id).copied())
            .unwrap_or_else(|| self.mode())
    }

    pub fn complete(&self, request_id: &str, decision: &str) -> Option<String> {
        let Some(decision) = ApprovalDecision::parse(decision) else {
            return None;
        };
        let pending = {
            let mut pending = match self.pending.lock() {
                Ok(guard) => guard,
                Err(_) => return None,
            };
            pending.remove(request_id)
        };
        let Some(pending) = pending else {
            return None;
        };
        let session_id = pending.session_id.clone();
        let _ = pending.sender.send(decision);
        Some(session_id)
    }

    pub fn authorize(
        &self,
        ctx: &ToolContext,
        tool: &dyn Tool,
        args: &Value,
        preview: Option<ToolPreview>,
    ) -> Result<(), ToolError> {
        ctx.ensure_not_cancelled()?;
        let mode = self.mode_for_session(ctx.root_session_id());
        {
            let mut recorded = self
                .recorded_modes
                .lock()
                .map_err(|_| ToolError::new("permission policy lock poisoned"))?;
            if recorded.get(ctx.root_session_id()) != Some(&mode) {
                let sandbox = match mode {
                    ToolApprovalMode::Ask => "read-only",
                    ToolApprovalMode::Auto => "workspace-write",
                    ToolApprovalMode::AlwaysAllow => "danger-full-access",
                };
                ctx.conversation.record_permission_event(
                    ctx.root_session_id(),
                    &ctx.assistant_message_id,
                    &uuid::Uuid::new_v4().to_string(),
                    "permission/policy",
                    serde_json::json!({"sandbox":sandbox}),
                )?;
                recorded.insert(ctx.root_session_id().into(), mode);
            }
        }
        // Legacy persisted `ask` now means read-only. No approval can elevate it.
        if mode == ToolApprovalMode::Ask && !read_only_effect(tool, args) {
            return Err(ToolError::policy_denied(
                "read-only mode denies this tool; switch to Workspace-write or Full-access",
            ));
        }
        if !requires_approval(tool) {
            return Ok(());
        }
        // Auto: 工作区内自动批准 — skip interactive tool prompts (workspace ops).
        // AlwaysAllow: 最高级 — same for tools; also auto-passes outside-workspace
        // path/shell gates in path.rs / shell.rs.
        if mode == ToolApprovalMode::AlwaysAllow
            || (mode == ToolApprovalMode::Auto
                && !tool.name().starts_with("mcp__")
                && !matches!(tool.name(), "run_shell" | "pwsh" | "bash")
                && !crate::core::plugins::requires_tool_approval(tool.name()))
        {
            return Ok(());
        }
        let session_id = ctx.root_session_id();

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();

        let mut title =
            crate::core::tools::display::build_activity_view(tool.name(), args, None).title;
        if matches!(tool.name(), "run_shell" | "pwsh" | "bash") {
            title.push_str(" — unsandboxed process / 非隔离进程");
        }
        ctx.conversation.record_permission_event(
            session_id,
            &ctx.assistant_message_id,
            &request_id,
            "approval/asked",
            serde_json::json!({"toolName":tool.name(),"arguments":args}),
        )?;
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| ToolError::new("approval lock poisoned"))?;
            pending.insert(
                request_id.clone(),
                PendingApproval {
                    sequence: super::context::next_interaction_sequence(),
                    sender: tx,
                    session_id: session_id.to_string(),
                    request_id: request_id.clone(),
                    tool_name: tool.name().to_string(),
                    title: title.clone(),
                    arguments: args.clone(),
                    preview: preview.clone(),
                },
            );
        }

        ctx.event_bus.emit(BusEvent::ToolApprovalRequest {
            session_id: session_id.to_string(),
            request_id: request_id.clone(),
            tool_name: tool.name().to_string(),
            title,
            arguments: args.clone(),
            preview,
        });

        let decision =
            super::approval_wait::wait(&rx, Some(&ctx.cancelled), Duration::from_secs(600));
        // Every terminal outcome retires the pending card, including cancellation
        // and timeout. A late reply must not grant permission to another turn.
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&request_id);
        }
        ctx.event_bus.emit(BusEvent::InteractionResolved {
            session_id: session_id.into(),
            request_id: request_id.clone(),
            kind: "tool_approval".into(),
        });
        let outcome = match &decision {
            Ok(ApprovalDecision::AllowOnce) => "allowed-once",
            Ok(ApprovalDecision::Deny) => "rejected",
            Err(_) if ctx.is_cancelled() => "cancelled",
            Err(_) => "unavailable",
        };
        ctx.conversation.record_permission_event(
            session_id,
            &ctx.assistant_message_id,
            &request_id,
            "approval/decided",
            serde_json::json!({"outcome":outcome}),
        )?;
        let decision = decision?;

        match decision {
            ApprovalDecision::AllowOnce => Ok(()),
            ApprovalDecision::Deny => Err(ToolError::user_denied("user denied tool execution")),
        }
    }

    /// Snapshot of current pending tool approval requests.
    /// Used to replay missing `tool-approval` events to newly connected clients.
    pub fn pending_items(&self) -> Vec<PendingToolApprovalSnapshot> {
        let guard = self.pending.lock().ok();
        guard
            .map(|map| {
                map.values()
                    .map(|p| PendingToolApprovalSnapshot {
                        sequence: p.sequence,
                        request_id: p.request_id.clone(),
                        session_id: p.session_id.clone(),
                        tool_name: p.tool_name.clone(),
                        title: p.title.clone(),
                        arguments: p.arguments.clone(),
                        preview: p.preview.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Session bookkeeping does not grant filesystem or subprocess authority.
fn read_only_effect(tool: &dyn Tool, args: &Value) -> bool {
    if matches!(tool.name(), "run_shell" | "pwsh" | "bash") {
        return false;
    }
    tool.read_only()
        || matches!(
            tool.name(),
            "ask_user"
                | "ask_user_question"
                | "update_tasks"
                | "todo_write"
                | "run_subagent"
                | "subagent"
                | "request_plan_mode"
                | "enter_plan_mode"
                | "exit_plan_mode"
        )
        || (tool.name() == "manage_plugin"
            && matches!(
                args["action"].as_str(),
                Some("list" | "list_files" | "get_file")
            ))
}

fn requires_approval(tool: &dyn Tool) -> bool {
    if crate::core::plugins::requires_tool_approval(tool.name()) {
        return true;
    }
    if tool.read_only() {
        return matches!(tool.name(), "run_shell");
    }
    matches!(
        tool.name(),
        "write"
            | "edit"
            | "pwsh"
            | "bash"
            | "apply_patch"
            | "write_file"
            | "replace_in_file"
            | "replace_many_in_file"
            | "move_path"
            | "delete_text_range"
            | "delete_go_symbol"
            | "edit_notebook_cell"
            | "run_shell"
    ) || tool.name().starts_with("mcp__")
}

pub fn shared_tool_approval_store() -> Arc<ToolApprovalStore> {
    static STORE: OnceLock<Arc<ToolApprovalStore>> = OnceLock::new();
    Arc::clone(STORE.get_or_init(|| Arc::new(ToolApprovalStore::new())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::EventBus;
    use crate::core::tools::context::{AskStore, PathPermissionStore};
    use std::sync::atomic::AtomicBool;

    struct Capture(mpsc::Sender<String>);
    impl EventBus for Capture {
        fn emit(&self, event: BusEvent) {
            if let BusEvent::ToolApprovalRequest { request_id, .. } = event {
                let _ = self.0.send(request_id);
            }
        }
    }
    struct TestTool(&'static str, bool);
    impl Tool for TestTool {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "test"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }
        fn read_only(&self) -> bool {
            self.1
        }
        fn execute(&self, _: &ToolContext, _: Value) -> Result<String, ToolError> {
            Ok(String::new())
        }
    }
    fn context() -> (ToolContext, mpsc::Receiver<String>) {
        let root = std::env::temp_dir().join(format!("anya-permission-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let (sender, receiver) = mpsc::channel();
        (
            ToolContext {
                workspace_root: root.clone(),
                request_context: Default::default(),
                session_id: uuid::Uuid::new_v4().to_string(),
                assistant_message_id: "test".into(),
                conversation: Arc::new(ConversationManager::new(root.join("chat.db"))),
                event_bus: Arc::new(Capture(sender)),
                tasks: Arc::new(Mutex::new(vec![])),
                ask_store: Arc::new(AskStore::new()),
                path_permission_store: Arc::new(PathPermissionStore::new()),
                registry: None,
                provider: None,
                subagent_depth: 0,
                max_subagent_depth: 1,
                subagent_id: None,
                parent_activity_id: None,
                app_handle: None,
                cancelled: Arc::new(AtomicBool::new(false)),
            },
            receiver,
        )
    }

    #[test]
    fn readonly_denies_native_and_dsh_writes_and_shell_without_asking() {
        let (ctx, requests) = context();
        let store = ToolApprovalStore::new();
        for name in [
            "write_file",
            "write",
            "edit",
            "pwsh",
            "bash",
            "run_shell",
            "mcp__mutate",
        ] {
            assert!(store
                .authorize(&ctx, &TestTool(name, false), &serde_json::json!({}), None)
                .is_err());
        }
        assert!(requests.try_recv().is_err());
        assert!(store
            .authorize(&ctx, &TestTool("read", true), &serde_json::json!({}), None)
            .is_ok());
        assert!(store
            .authorize(
                &ctx,
                &TestTool("todo_write", false),
                &serde_json::json!({}),
                None
            )
            .is_ok());
    }

    #[test]
    fn workspace_mode_allows_file_tools_and_full_access_allows_shell() {
        let (ctx, requests) = context();
        let store = ToolApprovalStore::new();
        store.configure(ToolApprovalMode::Auto);
        assert!(store
            .authorize(
                &ctx,
                &TestTool("write", false),
                &serde_json::json!({}),
                None
            )
            .is_ok());
        store.configure(ToolApprovalMode::AlwaysAllow);
        assert!(store
            .authorize(&ctx, &TestTool("pwsh", false), &serde_json::json!({}), None)
            .is_ok());
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn shell_grant_is_one_shot_and_audited() {
        let (ctx, requests) = context();
        let store = Arc::new(ToolApprovalStore::new());
        store.configure(ToolApprovalMode::Auto);
        for _ in 0..2 {
            let worker_store = store.clone();
            let worker_ctx = ctx.clone();
            let worker = std::thread::spawn(move || {
                worker_store.authorize(
                    &worker_ctx,
                    &TestTool("pwsh", false),
                    &serde_json::json!({"command":"Get-Date"}),
                    None,
                )
            });
            let id = requests.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(store.complete(&id, "allow_session").is_none());
            assert!(store.complete(&id, "allow_once").is_some());
            assert!(worker.join().unwrap().is_ok());
            assert!(store.pending_items().is_empty());
        }
        let pool = ctx.conversation.db_pool();
        let count: i64 = tauri::async_runtime::block_on(async {
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_permission_events WHERE kind IN ('approval/asked','approval/decided')")
                .fetch_one(&pool).await.unwrap()
        });
        assert_eq!(count, 4);
    }

    #[test]
    fn cancel_retires_pending_request_and_refuses_late_answer() {
        let (mut ctx, requests) = context();
        struct Recorder {
            original: Arc<dyn EventBus>,
            notices: Arc<Mutex<Vec<String>>>,
        }
        impl EventBus for Recorder {
            fn emit(&self, event: BusEvent) {
                if let BusEvent::InteractionResolved { request_id, .. } = &event {
                    self.notices.lock().unwrap().push(request_id.clone());
                }
                self.original.emit(event);
            }
        }
        let notices = Arc::new(Mutex::new(Vec::new()));
        ctx.event_bus = Arc::new(Recorder {
            original: ctx.event_bus.clone(),
            notices: notices.clone(),
        });
        let store = Arc::new(ToolApprovalStore::new());
        store.configure(ToolApprovalMode::Auto);
        let worker_store = store.clone();
        let worker_ctx = ctx.clone();
        let worker = std::thread::spawn(move || {
            worker_store.authorize(
                &worker_ctx,
                &TestTool("pwsh", false),
                &serde_json::json!({}),
                None,
            )
        });
        let id = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        ctx.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(worker.join().unwrap().is_err());
        assert!(store.pending_items().is_empty());
        assert!(store.complete(&id, "allow_once").is_none());
        assert_eq!(*notices.lock().unwrap(), vec![id]);
    }
}
