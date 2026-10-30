//! JSON catalog of bots and their scheduled tasks.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::{
    next_run_after, now_ms, truncate_chars, validate_spec, Bot, ScheduleSpec, ScheduledTask,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CatalogFile {
    bots: Vec<Bot>,
    tasks: Vec<ScheduledTask>,
}

fn catalog_path() -> PathBuf {
    crate::core::tools::memory::user_data_dir().join("bots.json")
}

fn with_catalog<T>(persist: bool, f: impl FnOnce(&mut CatalogFile) -> T) -> Result<T, String> {
    static CATALOG: OnceLock<Mutex<CatalogFile>> = OnceLock::new();
    let catalog = CATALOG.get_or_init(|| Mutex::new(load_catalog()));
    let mut guard = catalog.lock().unwrap_or_else(|error| error.into_inner());
    let value = f(&mut guard);
    if persist {
        save_catalog(&guard)?;
    }
    Ok(value)
}

fn load_catalog() -> CatalogFile {
    fs::read_to_string(catalog_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_catalog(catalog: &CatalogFile) -> Result<(), String> {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let raw = serde_json::to_string_pretty(catalog).map_err(|error| error.to_string())?;
    fs::write(path, raw).map_err(|error| error.to_string())
}

pub fn list_bots() -> Result<Vec<Bot>, String> {
    with_catalog(false, |catalog| catalog.bots.clone())
}

pub fn bot_for_session(session_id: &str) -> Option<Bot> {
    with_catalog(false, |catalog| {
        catalog
            .bots
            .iter()
            .find(|bot| bot.session_id == session_id)
            .cloned()
    })
    .ok()
    .flatten()
}

pub fn create_bot(name: String, avatar: String, session_id: String) -> Result<Bot, String> {
    let name = clean_name(&name)?;
    let session_id = session_id.trim().to_string();
    if session_id.is_empty() {
        return Err("session id is required".into());
    }
    let bot = Bot {
        id: Uuid::new_v4().to_string(),
        name,
        persona: String::new(),
        avatar: clean_avatar(&avatar)?,
        session_id,
        created_at: now_ms(),
    };
    let created = bot.clone();
    with_catalog(true, |catalog| {
        catalog.bots.push(bot);
    })?;
    crate::core::bots::notify_changed();
    Ok(created)
}

pub fn update_bot(id: &str, name: String, avatar: String) -> Result<Bot, String> {
    let name = clean_name(&name)?;
    let avatar = clean_avatar(&avatar)?;
    let updated = with_catalog(true, |catalog| {
        let bot = catalog
            .bots
            .iter_mut()
            .find(|bot| bot.id == id)
            .ok_or_else(|| format!("bot not found: {id}"))?;
        bot.name = name;
        bot.avatar = avatar;
        Ok(bot.clone())
    })?;
    crate::core::bots::notify_changed();
    updated
}

pub fn delete_bot(id: &str) -> Result<(), String> {
    with_catalog(true, |catalog| {
        catalog.bots.retain(|bot| bot.id != id);
        catalog.tasks.retain(|task| task.bot_id != id);
    })?;
    let path = crate::core::tools::memory::user_data_dir()
        .join("bot-memory")
        .join(format!("{id}.json"));
    let _ = fs::remove_file(path);
    crate::core::bots::forget_bot_memory(id);
    crate::core::bots::notify_changed();
    Ok(())
}

pub fn list_tasks() -> Result<Vec<ScheduledTask>, String> {
    with_catalog(false, |catalog| catalog.tasks.clone())
}

pub fn create_task(
    bot_id: &str,
    title: String,
    instruction: String,
    spec: ScheduleSpec,
) -> Result<ScheduledTask, String> {
    validate_spec(&spec)?;
    let title = clean_name(&title)?;
    let instruction = instruction.trim().to_string();
    if instruction.is_empty() {
        return Err("instruction is required".into());
    }
    let now = now_ms();
    let task = ScheduledTask {
        id: Uuid::new_v4().to_string(),
        bot_id: bot_id.to_string(),
        title,
        instruction: truncate_chars(&instruction, 8_000),
        next_run_ms: next_run_after(&spec, now.saturating_sub(1))?,
        spec,
        enabled: true,
        last_run_ms: None,
        last_status: "idle".into(),
        last_result: String::new(),
        created_at: now,
    };
    let created = task.clone();
    with_catalog(true, |catalog| {
        if !catalog.bots.iter().any(|bot| bot.id == bot_id) {
            return Err(format!("bot not found: {bot_id}"));
        }
        catalog.tasks.push(task);
        Ok(())
    })??;
    crate::core::bots::notify_changed();
    Ok(created)
}

pub fn set_task_enabled(id: &str, enabled: bool) -> Result<ScheduledTask, String> {
    let updated = with_catalog(true, |catalog| {
        let task = catalog
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or_else(|| format!("schedule not found: {id}"))?;
        task.enabled = enabled;
        if enabled && task.next_run_ms < now_ms() {
            if let Ok(next) = next_run_after(&task.spec, now_ms()) {
                task.next_run_ms = next;
            }
        }
        Ok(task.clone())
    })?;
    crate::core::bots::notify_changed();
    updated
}

pub fn delete_task(id: &str) -> Result<(), String> {
    with_catalog(true, |catalog| {
        catalog.tasks.retain(|task| task.id != id);
    })?;
    crate::core::bots::notify_changed();
    Ok(())
}

pub fn due_tasks(now: i64) -> Result<Vec<(ScheduledTask, Bot)>, String> {
    with_catalog(false, |catalog| {
        catalog
            .tasks
            .iter()
            .filter(|task| task.enabled && task.next_run_ms <= now)
            .filter_map(|task| {
                catalog
                    .bots
                    .iter()
                    .find(|bot| bot.id == task.bot_id)
                    .map(|bot| (task.clone(), bot.clone()))
            })
            .collect()
    })
}

pub fn task_with_bot(id: &str) -> Result<(ScheduledTask, Bot), String> {
    with_catalog(false, |catalog| {
        let task = catalog
            .tasks
            .iter()
            .find(|task| task.id == id)
            .cloned()
            .ok_or_else(|| format!("schedule not found: {id}"))?;
        let bot = catalog
            .bots
            .iter()
            .find(|bot| bot.id == task.bot_id)
            .cloned()
            .ok_or_else(|| format!("bot not found: {}", task.bot_id))?;
        Ok((task, bot))
    })?
}

pub fn postpone_task(id: &str, until_ms: i64, error: &str) -> Result<(), String> {
    with_catalog(true, |catalog| {
        if let Some(task) = catalog.tasks.iter_mut().find(|task| task.id == id) {
            task.next_run_ms = until_ms;
            task.last_status = "error".into();
            task.last_result = truncate_chars(error, 2_000);
        }
    })?;
    crate::core::bots::notify_changed();
    Ok(())
}

/// Record the outcome of a run and move a repeating task to its next slot.
pub fn complete_task(id: &str, result: &str, ok: bool) -> Result<(), String> {
    let now = now_ms();
    with_catalog(true, |catalog| {
        let Some(task) = catalog.tasks.iter_mut().find(|task| task.id == id) else {
            return;
        };
        task.last_run_ms = Some(now);
        task.last_status = if ok { "done" } else { "error" }.into();
        task.last_result = truncate_chars(result, 2_000);
        match &task.spec {
            ScheduleSpec::Once { .. } => task.enabled = false,
            spec => {
                if let Ok(next) = next_run_after(spec, now) {
                    task.next_run_ms = next;
                }
            }
        }
    })?;
    crate::core::bots::notify_changed();
    Ok(())
}

fn clean_avatar(value: &str) -> Result<String, String> {
    let avatar = value.trim();
    if avatar.is_empty() {
        return Ok(String::new());
    }
    let allowed = avatar.starts_with("data:image/svg+xml")
        || avatar.starts_with("data:image/png")
        || avatar.starts_with("data:image/jpeg")
        || avatar.starts_with("data:image/webp");
    if !allowed || avatar.len() > 1_500_000 {
        return Err("avatar must be a png, jpeg, webp, or svg image under 1.5 MB".into());
    }
    Ok(avatar.to_string())
}

fn clean_name(value: &str) -> Result<String, String> {
    let name = value.trim();
    if name.is_empty() {
        return Err("name is required".into());
    }
    Ok(truncate_chars(name, 80))
}
