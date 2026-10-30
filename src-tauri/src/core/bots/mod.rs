//! User-created bots, each with a private memory file and its own schedules.

mod model;
mod runner;
mod store;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter};

pub use model::{Bot, ScheduleSpec, ScheduledTask};
pub use runner::{note_run_finished, run_now, start_scheduler};
pub use store::{
    bot_for_session, create_bot, create_task, delete_bot, delete_task, list_bots, list_tasks,
    set_task_enabled, update_bot,
};

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn bind_app(app: AppHandle) {
    let _ = APP.set(app);
}

pub fn notify_changed() {
    if let Some(app) = APP.get() {
        let _ = app.emit("bots-changed", ());
    }
}

/// System note injected only while the active session belongs to a bot.
pub fn persona_block(session_id: &str) -> Option<String> {
    let bot = bot_for_session(session_id)?;
    let duty = if bot.persona.trim().is_empty() {
        "The user explains this bot's responsibilities in the conversation. Treat those instructions as its job, and save durable duties with save_memory.".to_string()
    } else {
        format!("Standing instructions:\n{}", bot.persona.trim())
    };
    Some(format!(
        "You are the bot \"{}\". This conversation belongs to this bot alone.\n{duty}\n\nsave_memory, search_memory, and delete_memory use this bot's private memory, not the user's other chats.\nUse manage_schedule to create, list, or cancel this bot's scheduled tasks. When a user message says a scheduled task is due, do that task and reply with the result they asked for.\nThis bot is a general assistant with its own memory, not only a scheduler.",
        bot.name
    ))
}

/// The shared user memory, or this bot's own file when the session is bound to one.
pub fn memory_for_session(session_id: &str) -> Arc<crate::core::tools::memory::MemoryStore> {
    let Some(bot) = bot_for_session(session_id) else {
        return crate::core::tools::memory::shared_memory_store();
    };
    bot_memory_store(&bot.id)
}

pub fn forget_bot_memory(bot_id: &str) {
    if let Some(stores) = memory_stores().get() {
        stores
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(bot_id);
    }
}

fn bot_memory_store(bot_id: &str) -> Arc<crate::core::tools::memory::MemoryStore> {
    let stores = memory_stores().get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = stores.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(store) = guard.get(bot_id) {
        return Arc::clone(store);
    }
    let path = crate::core::tools::memory::user_data_dir()
        .join("bot-memory")
        .join(format!("{}.json", safe_id(bot_id)));
    let store = Arc::new(crate::core::tools::memory::MemoryStore::new(path));
    guard.insert(bot_id.to_string(), Arc::clone(&store));
    store
}

fn memory_stores() -> &'static OnceLock<Mutex<HashMap<String, Arc<crate::core::tools::memory::MemoryStore>>>>
{
    static STORES: OnceLock<
        Mutex<HashMap<String, Arc<crate::core::tools::memory::MemoryStore>>>,
    > = OnceLock::new();
    &STORES
}

fn safe_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
