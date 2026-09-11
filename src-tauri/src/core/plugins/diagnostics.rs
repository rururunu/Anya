//! In-memory bridge for plugin runtime faults reported by the workbench webview.
//!
//! Bundling errors (esbuild) already return through `ToolError` on the same
//! `manage_plugin` call. Errors thrown *inside* `activate()`/`deactivate()` happen
//! in the frontend JS heap, invisible to the agent unless the frontend reports
//! them back here — this is that bridge, so `manage_plugin errors` can answer
//! "did the plugin actually run" instead of only "did it bundle".

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_RECORDS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginErrorRecord {
    pub plugin_id: String,
    pub phase: String,
    pub message: String,
    pub at_ms: u64,
}

fn store() -> &'static Mutex<VecDeque<PluginErrorRecord>> {
    static STORE: std::sync::OnceLock<Mutex<VecDeque<PluginErrorRecord>>> =
        std::sync::OnceLock::new();
    STORE.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECORDS)))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn record_plugin_error(plugin_id: &str, phase: &str, message: &str) {
    let Ok(mut records) = store().lock() else {
        return;
    };
    if records.len() >= MAX_RECORDS {
        records.pop_front();
    }
    records.push_back(PluginErrorRecord {
        plugin_id: plugin_id.to_string(),
        phase: phase.to_string(),
        message: message.chars().take(2000).collect(),
        at_ms: now_ms(),
    });
}

pub fn list_plugin_errors(
    plugin_id: Option<&str>,
    since_ms: Option<u64>,
) -> Vec<PluginErrorRecord> {
    let Ok(records) = store().lock() else {
        return Vec::new();
    };
    records
        .iter()
        .filter(|r| plugin_id.map(|id| r.plugin_id == id).unwrap_or(true))
        .filter(|r| since_ms.map(|since| r.at_ms >= since).unwrap_or(true))
        .cloned()
        .collect()
}

pub fn clear_plugin_errors(plugin_id: Option<&str>) {
    let Ok(mut records) = store().lock() else {
        return;
    };
    match plugin_id {
        Some(id) => records.retain(|r| r.plugin_id != id),
        None => records.clear(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn records_and_filters_by_plugin() {
        let _guard = test_lock();
        clear_plugin_errors(None);
        record_plugin_error("a", "activate", "boom");
        record_plugin_error("b", "mount", "bang");
        assert_eq!(list_plugin_errors(Some("a"), None).len(), 1);
        assert_eq!(list_plugin_errors(None, None).len(), 2);
        clear_plugin_errors(Some("a"));
        assert_eq!(list_plugin_errors(None, None).len(), 1);
        clear_plugin_errors(None);
        assert!(list_plugin_errors(None, None).is_empty());
    }
}
