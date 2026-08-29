//! Device list and gateway preference persistence.
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::types::{GatewayPrefs, PairedDevice};

pub(super) fn load_devices(path: &PathBuf) -> HashMap<String, PairedDevice> {
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str::<Vec<PairedDevice>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|d| (d.device_id.clone(), d))
        .collect()
}

pub(super) fn save_devices(path: &PathBuf, devices: &HashMap<String, PairedDevice>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let list: Vec<_> = devices.values().cloned().collect();
    let raw = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

pub(super) fn load_prefs(path: &PathBuf) -> Option<GatewayPrefs> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub(super) fn save_prefs(path: &PathBuf, prefs: &GatewayPrefs) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}
