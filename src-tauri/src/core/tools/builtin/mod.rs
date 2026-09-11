//! Builtin tool registration and module layout.

mod chat_history;
mod files;
mod image_gen;
mod memory_tools;
mod misc;
mod plugin;
mod shell;
mod tasks;
mod theme;

use std::sync::Arc;

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::EventBus;
use crate::core::tools::memory::shared_memory_store;
use crate::core::tools::registry::ToolRegistry;
use crate::core::tools::shell_jobs::ShellJobStore;
use crate::core::tools::task_list::shared_task_list;

use chat_history::*;
use files::*;
use image_gen::*;
use memory_tools::*;
use misc::*;
use plugin::*;
use shell::*;
use tasks::*;
use theme::*;

pub fn register_all(
    registry: &mut ToolRegistry,
    conversation: Arc<ConversationManager>,
    event_bus: Arc<dyn EventBus>,
) {
    let shell_jobs = ShellJobStore::new();
    let memory = shared_memory_store();
    let tasks = shared_task_list();

    macro_rules! reg {
        ($tool:expr) => {
            registry.register(Arc::new($tool));
        };
    }

    reg!(ReadFileTool);
    reg!(ListFolderTool);
    reg!(FindFilesTool);
    reg!(SearchFilesTool);
    reg!(ListSymbolsTool);
    reg!(WriteFileTool);
    reg!(ReplaceInFileTool);
    reg!(ReplaceManyInFileTool);
    reg!(MovePathTool);
    reg!(EditNotebookCellTool);
    reg!(DeleteTextRangeTool);
    reg!(DeleteGoSymbolTool);

    registry.register(Arc::new(RunShellTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(ReadShellOutputTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(WaitForShellTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(StopShellTool { jobs: shell_jobs }));

    registry.register(Arc::new(SavePlanTool {
        event_bus: Arc::clone(&event_bus),
    }));
    registry.register(Arc::new(UpdateTasksTool {
        tasks: Arc::clone(&tasks),
        event_bus: Arc::clone(&event_bus),
    }));
    registry.register(Arc::new(AskUserTool {
        event_bus: Arc::clone(&event_bus),
    }));

    registry.register(Arc::new(SaveMemoryTool {
        memory: Arc::clone(&memory),
    }));
    registry.register(Arc::new(SearchMemoryTool { memory }));
    registry.register(Arc::new(DeleteMemoryTool {
        memory: shared_memory_store(),
    }));

    registry.register(Arc::new(ListChatsTool));
    registry.register(Arc::new(ReadChatTool {
        conversation: Arc::clone(&conversation),
    }));
    registry.register(Arc::new(SearchPastChatsTool {
        conversation: Arc::clone(&conversation),
    }));

    registry.register(Arc::new(CompletePlanStepTool {
        tasks: Arc::clone(&tasks),
        event_bus,
    }));
    registry.register(Arc::new(RunSlashCommandTool));
    registry.register(Arc::new(ConnectToolsTool));
    registry.register(Arc::new(InstallToolSourceTool));
    registry.register(Arc::new(LspTool));
    registry.register(Arc::new(SearchCodebaseTool));
    registry.register(Arc::new(RebuildCodebaseIndexTool));
    registry.register(Arc::new(ShareToCompanionTool));
    registry.register(Arc::new(SharePreviewUrlTool));
    registry.register(Arc::new(GenerateImageTool));
    registry.register(Arc::new(ManageCustomThemeTool));
    registry.register(Arc::new(ManagePluginTool));
    registry.register(Arc::new(ListFailureCandidatesTool {
        conversation: Arc::clone(&conversation),
    }));
}
