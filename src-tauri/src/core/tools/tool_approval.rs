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
    AllowSession,
    Deny,
}

impl ApprovalDecision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "allow_once" => Some(Self::AllowOnce),
            "allow_session" => Some(Self::AllowSession),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

struct PendingApproval {
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
    session_grants: Mutex<HashMap<String, Vec<String>>>,
    mode: Mutex<ToolApprovalMode>,
    session_modes: Mutex<HashMap<String, ToolApprovalMode>>,
}

impl ToolApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            session_grants: Mutex::new(HashMap::new()),
            mode: Mutex::new(ToolApprovalMode::Ask),
            session_modes: Mutex::new(HashMap::new()),
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

    pub fn complete(&self, request_id: &str, decision: &str) -> bool {
        let Some(decision) = ApprovalDecision::parse(decision) else {
            return false;
        };
        let sender = {
            let mut pending = match self.pending.lock() {
                Ok(guard) => guard,
                Err(_) => return false,
            };
            pending.remove(request_id).map(|p| p.sender)
        };
        if let Some(sender) = sender {
            let _ = sender.send(decision);
            true
        } else {
            false
        }
    }

    fn session_allowed(&self, session_id: &str, tool_name: &str) -> bool {
        self.session_grants
            .lock()
            .ok()
            .and_then(|g| g.get(session_id).cloned())
            .is_some_and(|tools| tools.iter().any(|t| t == tool_name || t == "*"))
    }

    fn grant_session(&self, session_id: &str, tool_name: &str) {
        if let Ok(mut grants) = self.session_grants.lock() {
            grants
                .entry(session_id.to_string())
                .or_default()
                .push(tool_name.to_string());
        }
    }

    pub fn authorize(
        &self,
        ctx: &ToolContext,
        tool: &dyn Tool,
        args: &Value,
        preview: Option<ToolPreview>,
    ) -> Result<(), ToolError> {
        if !requires_approval(tool) {
            return Ok(());
        }
        let mode = self.mode_for_session(&ctx.root_session_id().to_string());
        if mode == ToolApprovalMode::AlwaysAllow {
            return Ok(());
        }
        // Auto still asks for write tools (per plan): write ops always prompt unless AlwaysAllow
        if mode == ToolApprovalMode::Auto && tool.read_only() {
            return Ok(());
        }
        let session_id = ctx.root_session_id();
        if self.session_allowed(session_id, tool.name()) {
            return Ok(());
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();

        let title = crate::core::tools::display::build_activity_view(tool.name(), args, None).title;
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| ToolError::new("approval lock poisoned"))?;
            pending.insert(
                request_id.clone(),
                PendingApproval {
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

        let decision = rx
            .recv_timeout(Duration::from_secs(600))
            .map_err(|_| ToolError::user_denied("tool approval timed out"))?;

        match decision {
            ApprovalDecision::AllowOnce => Ok(()),
            ApprovalDecision::AllowSession => {
                self.grant_session(session_id, tool.name());
                Ok(())
            }
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

fn requires_approval(tool: &dyn Tool) -> bool {
    if tool.read_only() {
        return matches!(tool.name(), "run_shell");
    }
    matches!(
        tool.name(),
        "apply_patch"
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
