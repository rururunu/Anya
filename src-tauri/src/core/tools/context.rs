use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::EventBus;
use crate::core::tools::error::ToolError;
use crate::core::tools::preview::ToolPreview;

pub use crate::core::tools::path_permission::PathPermissionStore;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn read_only(&self) -> bool {
        false
    }
    /// When false, the tool is omitted from model-facing schemas.
    fn available(&self) -> bool {
        true
    }

    /// Optional pre-execution preview for approval / checkpoints.
    fn preview(&self, _ctx: &ToolContext, _args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        Ok(None)
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError>;

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters_schema(),
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub content: String,
    pub status: String,
    pub active_form: Option<String>,
    #[serde(default)]
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskOption {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskQuestion {
    pub header: String,
    pub question: String,
    pub options: Vec<AskOption>,
    #[serde(default)]
    pub multi_select: bool,
}

pub struct PendingAsk {
    pub sender: mpsc::Sender<String>,
    pub session_id: String,
    pub questions: Vec<AskQuestion>,
}

pub struct AskStore {
    inner: Mutex<std::collections::HashMap<String, PendingAsk>>,
}

impl AskStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn insert(
        &self,
        request_id: String,
        session_id: String,
        questions: Vec<AskQuestion>,
        sender: mpsc::Sender<String>,
    ) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(
                request_id,
                PendingAsk {
                    sender,
                    session_id,
                    questions,
                },
            );
        }
    }

    pub fn complete(&self, request_id: &str, answer: String) -> bool {
        let sender = self
            .inner
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(request_id).map(|pending| pending.sender));
        if let Some(sender) = sender {
            let _ = sender.send(answer);
            return true;
        }
        false
    }

    /// Snapshot of current pending asks.
    /// Used to replay missing `ask-user` events to newly connected clients.
    pub fn pending_items(&self) -> Vec<PendingAskSnapshot> {
        let guard = self.inner.lock().ok();
        guard
            .map(|map| {
                map.iter()
                    .map(|(request_id, pending)| PendingAskSnapshot {
                        request_id: request_id.clone(),
                        session_id: pending.session_id.clone(),
                        questions: pending.questions.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for AskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAskSnapshot {
    pub request_id: String,
    pub session_id: String,
    pub questions: Vec<AskQuestion>,
}

#[derive(Clone)]
pub struct ToolContext {
    pub workspace_root: PathBuf,
    pub request_context: crate::core::runtime::RequestContext,
    pub session_id: String,
    pub assistant_message_id: String,
    pub conversation: Arc<ConversationManager>,
    pub event_bus: Arc<dyn EventBus>,
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub ask_store: Arc<AskStore>,
    pub path_permission_store: Arc<PathPermissionStore>,
    pub registry: Option<Arc<super::registry::ToolRegistry>>,
    pub provider: Option<Arc<dyn crate::core::ai::provider::AIProvider>>,
    pub subagent_depth: u32,
    pub max_subagent_depth: u32,
    pub subagent_id: Option<String>,
    /// Tool activity that owns this execution context in the parent agent.
    pub parent_activity_id: Option<String>,
    pub app_handle: Option<tauri::AppHandle>,
    pub cancelled: Arc<AtomicBool>,
}

impl ToolContext {
    pub fn child_subagent(&self, _prompt: &str) -> ToolContext {
        let subagent_id = uuid::Uuid::new_v4().to_string();
        ToolContext {
            workspace_root: self.workspace_root.clone(),
            // Minimal context: do not inherit IDE/clipboard/git from the parent turn.
            request_context: Default::default(),
            session_id: format!("{}-sub-{subagent_id}", self.session_id),
            assistant_message_id: self.assistant_message_id.clone(),
            conversation: Arc::clone(&self.conversation),
            event_bus: Arc::clone(&self.event_bus),
            tasks: Arc::clone(&self.tasks),
            ask_store: Arc::clone(&self.ask_store),
            path_permission_store: Arc::clone(&self.path_permission_store),
            registry: self.registry.clone(),
            provider: self.provider.clone(),
            subagent_depth: self.subagent_depth + 1,
            max_subagent_depth: self.max_subagent_depth,
            subagent_id: Some(subagent_id),
            parent_activity_id: self.parent_activity_id.clone(),
            app_handle: self.app_handle.clone(),
            cancelled: Arc::clone(&self.cancelled),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Child agents share the parent turn's UI, approvals, diff and checkpoint.
    pub fn root_session_id(&self) -> &str {
        self.session_id
            .split_once("-sub-")
            .map(|(root, _)| root)
            .unwrap_or(&self.session_id)
    }

    pub fn ensure_not_cancelled(&self) -> Result<(), ToolError> {
        if self.is_cancelled() {
            Err(ToolError::cancelled())
        } else {
            Ok(())
        }
    }

    /// Depth 0 is the root agent. Spawning is allowed while `subagent_depth < max_subagent_depth`.
    /// With the default `max_subagent_depth = 1`, only the root may spawn (children cannot nest).
    pub fn can_spawn_subagent(&self) -> bool {
        self.subagent_depth < self.max_subagent_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};

    struct NullEventBus;
    impl EventBus for NullEventBus {
        fn emit(&self, _event: BusEvent) {}
    }

    #[test]
    fn child_subagent_shares_cancellation_token() {
        let db_path =
            std::env::temp_dir().join(format!("peek-tool-context-{}.db", uuid::Uuid::new_v4()));
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: Default::default(),
            session_id: "parent".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NullEventBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
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
        };
        let child = context.child_subagent("test");

        context.cancelled.store(true, Ordering::Relaxed);

        assert!(child.is_cancelled());
        assert!(child.subagent_id.is_some());
        assert!(child.session_id.contains("-sub-"));
        assert_eq!(child.root_session_id(), "parent");
        assert!(context.can_spawn_subagent());
        assert!(!child.can_spawn_subagent());
        drop(context);
        drop(child);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn child_subagent_snapshots_files_in_parent_checkpoint() {
        use crate::core::checkpoint::CheckpointStore;
        use crate::core::tools::preview::{ChangeKind, ToolPreview};

        let base =
            std::env::temp_dir().join(format!("peek-child-checkpoint-{}", uuid::Uuid::new_v4()));
        let workspace = base.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("child.txt"), "before\n").unwrap();

        let db_path = base.join("chat.db");
        let context = ToolContext {
            workspace_root: workspace.clone(),
            request_context: Default::default(),
            session_id: "parent-session".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(ConversationManager::new(db_path)),
            event_bus: Arc::new(NullEventBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
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
        };
        let child = context.child_subagent("edit child.txt");
        let store = CheckpointStore::new(base.join("checkpoints"));
        store.begin_turn(
            "parent-session",
            1,
            "delegate edit",
            Some("user-1".into()),
            Some(&workspace),
        );
        store
            .snapshot_preview(
                child.root_session_id(),
                &workspace,
                &ToolPreview {
                    path: "child.txt".into(),
                    affected_paths: vec!["child.txt".into()],
                    kind: ChangeKind::Modify,
                    old_text: Some("before\n".into()),
                    new_text: Some("after\n".into()),
                    unified_diff: String::new(),
                },
            )
            .unwrap();
        store.finish_turn("parent-session").unwrap();

        std::fs::write(workspace.join("child.txt"), "after\n").unwrap();
        assert_eq!(
            store.restore_code("parent-session", 1, &workspace).unwrap(),
            1
        );
        assert_eq!(
            std::fs::read_to_string(workspace.join("child.txt")).unwrap(),
            "before\n"
        );

        drop(child);
        drop(context);
        let _ = std::fs::remove_dir_all(base);
    }
}
