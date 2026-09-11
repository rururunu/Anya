//! Skip every plugin when the user (or a crashed restore) enters safe mode.

use std::fs;

use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;

pub fn safe_mode_marker() -> std::path::PathBuf {
    plugins_dir().join(".safe-mode")
}

pub fn is_plugin_safe_mode() -> bool {
    env_flag("ANYA_PLUGIN_SAFE_MODE") || env_flag("ANYA_SAFE_MODE") || safe_mode_marker().is_file()
}

pub fn plugin_safe_mode_reason() -> String {
    if env_flag("ANYA_PLUGIN_SAFE_MODE") {
        return "ANYA_PLUGIN_SAFE_MODE".into();
    }
    if env_flag("ANYA_SAFE_MODE") {
        return "ANYA_SAFE_MODE".into();
    }
    fs::read_to_string(safe_mode_marker())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "safe-mode marker".into())
}

pub fn enter_plugin_safe_mode(reason: &str) -> Result<(), ToolError> {
    let dir = plugins_dir();
    fs::create_dir_all(&dir).map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(safe_mode_marker(), reason).map_err(|e| ToolError::new(e.to_string()))?;
    tracing::warn!(reason, "plugin safe mode on: skipping all plugins");
    Ok(())
}

pub fn clear_plugin_safe_mode() -> Result<(), ToolError> {
    let path = safe_mode_marker();
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| ToolError::new(e.to_string()))?;
    }
    Ok(())
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref().map(str::trim),
        Some("1") | Some("true") | Some("TRUE") | Some("yes")
    )
}
