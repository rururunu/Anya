mod context;
mod send;
mod sessions;

use std::sync::{Arc, Mutex};

use crate::core::agent::{AgentDebugEvent, AgentRuntime};
use crate::core::ai::provider::AIProvider;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::context::ContextResolver;
use crate::core::event::{EventBus, PlanModeSource};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem};
use crate::core::workspace::WorkspaceManager;
use crate::runtime::ToolManager;

pub struct ChatSendResult {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub agent_run_id: Option<String>,
}

pub struct ChatService {
    pub(super) provider: Arc<dyn AIProvider>,
    pub(super) event_bus: Arc<dyn EventBus>,
    pub(super) conversation: Arc<ConversationManager>,
    pub(super) workspace_manager: Arc<WorkspaceManager>,
    pub(super) context_resolver: ContextResolver,
    pub(super) agent_runtime: AgentRuntime,
    pub(super) tools: Arc<ToolManager>,
    pub(super) ask_store: Arc<AskStore>,
    pub(super) path_permission_store: Arc<PathPermissionStore>,
    pub(super) tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub(super) app_handle: Option<tauri::AppHandle>,
}

impl ChatService {
    /// Creates a chat service wired to the given provider, event bus, and workspace stack.
    pub fn new(
        provider: Arc<dyn AIProvider>,
        event_bus: Arc<dyn EventBus>,
        context_resolver: ContextResolver,
        tools: Arc<ToolManager>,
        conversation: Arc<ConversationManager>,
        workspace_manager: Arc<WorkspaceManager>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        let agent_runtime = AgentRuntime::new(Arc::clone(&event_bus), Arc::clone(&tools));
        Self {
            provider,
            event_bus,
            conversation,
            workspace_manager,
            context_resolver,
            agent_runtime,
            tools,
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            tasks: crate::core::tools::task_list::shared_task_list(),
            app_handle: Some(app_handle),
        }
    }

    /// Returns the shared conversation manager for this service.
    pub fn conversation(&self) -> Arc<ConversationManager> {
        Arc::clone(&self.conversation)
    }

    /// Returns the ask-mode clarification store shared with the agent runtime.
    pub fn ask_store(&self) -> Arc<AskStore> {
        Arc::clone(&self.ask_store)
    }

    /// Returns the path-permission store shared with the agent runtime.
    pub fn path_permission_store(&self) -> Arc<PathPermissionStore> {
        Arc::clone(&self.path_permission_store)
    }

    /// Returns a snapshot of recent agent debug events for diagnostics.
    pub fn agent_debug_snapshot(&self) -> Vec<AgentDebugEvent> {
        self.agent_runtime.debug_snapshot()
    }

    /// Emits a plan-mode changed event on the shared event bus.
    pub fn emit_plan_mode_changed(&self, session_id: &str, active: bool, source: PlanModeSource) {
        use crate::core::event::BusEvent;

        self.event_bus.emit(BusEvent::PlanModeChanged {
            session_id: session_id.to_string(),
            active,
            source,
        });
    }

    /// Rebuilds the in-memory checklist from remaining history after a rewind.
    pub fn restore_progress_after_rewind(&self, session_id: &str) {
        use crate::core::event::BusEvent;
        use crate::core::tools::plan_mode::{shared_plan_mode_store, task_status_is_open};
        use crate::core::tools::task_list::{
            history_has_save_plan, last_tasks_from_messages, replace_shared_tasks,
        };

        let messages = self.conversation.messages(session_id);
        let tasks = last_tasks_from_messages(&messages);
        replace_shared_tasks(tasks.clone());
        let plan_store = shared_plan_mode_store();
        plan_store.sync_plan_saved(session_id, history_has_save_plan(&messages));
        let has_open = tasks.iter().any(|task| task_status_is_open(&task.status));
        if plan_store.is_active(session_id) && has_open {
            plan_store.mark_awaiting_approval(session_id);
        } else if !has_open {
            plan_store.clear_awaiting_approval(session_id);
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: session_id.to_string(),
            tasks,
        });
    }
}
