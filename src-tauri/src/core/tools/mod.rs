pub mod agent;
pub mod apply_patch;
pub mod builtin;
pub mod display;
pub mod error;
pub mod file_io;
pub mod fs_skip;
pub mod fuzzy;
pub mod image_mode;
pub mod memory;
pub mod path;
pub mod path_permission;
pub mod plan_mode;
pub mod preview;
pub mod process_stats;
pub mod registry;
pub mod sandbox;
pub mod shell_jobs;
pub mod shell_judge;
pub mod skills;
pub mod tool_approval;
pub mod workspace_index;

pub mod context;

pub use context::Tool;
pub use registry::ToolRegistry;

use std::sync::Arc;

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::EventBus;

pub fn default_registry(
    conversation: Arc<ConversationManager>,
    event_bus: Arc<dyn EventBus>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    builtin::register_all(&mut registry, conversation, event_bus);
    apply_patch::register(&mut registry);
    skills::register_all(&mut registry);
    agent::register_all(&mut registry);
    crate::core::office::register_tools(&mut registry);
    registry.register(Arc::new(crate::runtime::context::ContextTool));
    registry.register(Arc::new(crate::runtime::workspace::WorkspaceTool));
    registry.register(Arc::new(crate::runtime::git::GitTool));
    registry.register(Arc::new(crate::runtime::git::GitCommitTool));
    registry.register(Arc::new(crate::runtime::search::SearchTool::new(
        crate::runtime::search::shared_search_runtime(),
    )));
    let config = crate::runtime::config::RuntimeConfig::default();
    if let Ok(provider) = crate::runtime::browser::JinaReaderProvider::new(
        config.jina_reader_base_url,
        config.jina_api_key,
    ) {
        registry.register(Arc::new(crate::runtime::browser::BrowserTool::new(
            Arc::new(provider),
        )));
    }
    registry
}
