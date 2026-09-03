//! Per-session staged message queue shared by desktop UI and Companion.
//!
//! While a turn is running, new user messages land here instead of soft-injecting.
//! They flush one-by-one after `ChatFinished` (or via explicit "guide" → soft inject).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

struct StagedStore {
    by_session: HashMap<String, Vec<String>>,
}

fn store() -> &'static Mutex<StagedStore> {
    static STORE: OnceLock<Mutex<StagedStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(StagedStore {
        by_session: HashMap::new(),
    }))
}

pub fn list(session_id: &str) -> Vec<String> {
    store()
        .lock()
        .ok()
        .and_then(|g| g.by_session.get(session_id).cloned())
        .unwrap_or_default()
}

pub fn push(session_id: &str, content: &str) -> Vec<String> {
    let trimmed = content.trim();
    if session_id.trim().is_empty() || trimmed.is_empty() {
        return list(session_id);
    }
    if let Ok(mut guard) = store().lock() {
        guard
            .by_session
            .entry(session_id.to_string())
            .or_default()
            .push(trimmed.to_string());
        return guard
            .by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default();
    }
    Vec::new()
}

pub fn insert(session_id: &str, index: usize, content: &str) -> Vec<String> {
    let trimmed = content.trim();
    if session_id.trim().is_empty() || trimmed.is_empty() {
        return list(session_id);
    }
    if let Ok(mut guard) = store().lock() {
        let queue = guard.by_session.entry(session_id.to_string()).or_default();
        let at = index.min(queue.len());
        queue.insert(at, trimmed.to_string());
        return queue.clone();
    }
    Vec::new()
}

pub fn remove(session_id: &str, index: usize) -> Vec<String> {
    if let Ok(mut guard) = store().lock() {
        let Some(queue) = guard.by_session.get_mut(session_id) else {
            return Vec::new();
        };
        if index < queue.len() {
            queue.remove(index);
        }
        if queue.is_empty() {
            guard.by_session.remove(session_id);
            return Vec::new();
        }
        return queue.clone();
    }
    Vec::new()
}

pub fn clear(session_id: &str) {
    if let Ok(mut guard) = store().lock() {
        guard.by_session.remove(session_id);
    }
}

/// Take and remove the message at `index` (for guide → soft inject).
pub fn take_at(session_id: &str, index: usize) -> Option<String> {
    if let Ok(mut guard) = store().lock() {
        let queue = guard.by_session.get_mut(session_id)?;
        if index >= queue.len() {
            return None;
        }
        let content = queue.remove(index);
        if queue.is_empty() {
            guard.by_session.remove(session_id);
        }
        return Some(content);
    }
    None
}

/// Take the front message for post-turn auto-send.
pub fn pop_front(session_id: &str) -> Option<String> {
    take_at(session_id, 0)
}

pub fn event_payload(session_id: &str, messages: &[String]) -> serde_json::Map<String, Value> {
    json!({
        "sessionId": session_id,
        "messages": messages,
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}
