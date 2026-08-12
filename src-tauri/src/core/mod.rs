//! Domain core: chat, agent orchestration, AI providers, tools, context, office.
//!
//! Request flow overview: `docs/architecture-overview.md`.
//! Chat protocol types live in [`runtime`]; pluggable tool adapters live in
//! the crate-root `crate::runtime` module (different meaning — see that overview).

pub mod agent;
pub mod ai;
pub mod chat;
pub mod checkpoint;
pub mod context;
pub mod event;
pub mod lsp;
pub mod mcp;
pub mod office;
pub mod remote;
pub mod rules;
pub mod runtime;
pub mod token;
pub mod tools;
pub mod workspace;

pub use crate::runtime::ToolManager;
pub use chat::ChatService;
pub use event::EventBus;
pub use workspace::WorkspaceManager;

use std::sync::Arc;

use ai::resolve_provider;
use context::ContextResolver;
use tauri::{AppHandle, Manager};
use tools::default_registry;

pub struct PeekCore {
    chat: ChatService,
    workspace_manager: Arc<WorkspaceManager>,
    tools: Arc<ToolManager>,
}

impl PeekCore {
    pub fn new(app: AppHandle, event_bus: Arc<dyn EventBus>) -> Self {
        let provider = resolve_provider(app.clone());
        let config_dir = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let db_path = config_dir.join("history.db");
        let conversation = Arc::new(
            crate::core::chat::conversation_manager::ConversationManager::new(db_path.clone()),
        );
        let workspace_manager = Arc::new(WorkspaceManager::new(db_path));
        let tools = Arc::new(ToolManager::new(default_registry(
            Arc::clone(&conversation),
            Arc::clone(&event_bus),
        )));
        let chat = ChatService::new(
            provider,
            event_bus,
            ContextResolver::new(),
            Arc::clone(&tools),
            conversation,
            Arc::clone(&workspace_manager),
            app,
        );

        Self {
            chat,
            workspace_manager,
            tools,
        }
    }

    pub fn chat(&self) -> &ChatService {
        &self.chat
    }

    pub fn workspaces(&self) -> Arc<WorkspaceManager> {
        Arc::clone(&self.workspace_manager)
    }

    pub fn tools(&self) -> &ToolManager {
        &self.tools
    }
}
