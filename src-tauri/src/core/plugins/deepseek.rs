//! Built-in host RPC: DeepSeek account balance via Anya's stored API key.

use serde_json::Value;
use tauri::AppHandle;

use super::grant::grant_has;
use crate::core::ai::deepseek;
use crate::core::tools::error::ToolError;
use crate::services::settings_store::get_settings;

/// Host RPC `deepseek.*` (workbench `ctx.host.rpc`).
pub fn dispatch_rpc(app: &AppHandle, plugin_id: &str, method: &str) -> Result<Value, ToolError> {
    if !grant_has(plugin_id, "deepseek.balance") {
        return Err(ToolError::new("plugin is not granted deepseek.balance"));
    }
    match balance_action(method)? {
        "balance" => get_balance(app),
        other => Err(ToolError::new(format!("unknown deepseek method: {other}"))),
    }
}

fn balance_action(method: &str) -> Result<&str, ToolError> {
    method
        .strip_prefix("deepseek.")
        .filter(|action| !action.is_empty())
        .ok_or_else(|| ToolError::new(format!("unknown deepseek method: {method}")))
}

fn get_balance(app: &AppHandle) -> Result<Value, ToolError> {
    let settings = get_settings(app).map_err(ToolError::new)?;
    let report = tauri::async_runtime::block_on(deepseek::get_user_balance(
        &settings.deepseek_api_key,
    ))
    .map_err(|error| ToolError::new(error.to_string()))?;
    serde_json::to_value(report)
        .map_err(|error| ToolError::new(format!("serialize balance: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_balance_action() {
        assert_eq!(balance_action("deepseek.balance").unwrap(), "balance");
        assert!(balance_action("deepseek.").is_err());
        assert!(balance_action("balance").is_err());
    }

    #[test]
    fn ungrafted_plugin_is_rejected() {
        // grant_has is false for never-enabled ids; AppHandle is unused on that path.
        // Use a null-ish call only after we know grant fails — dispatch needs AppHandle
        // only for the network branch, so exercise the grant gate via grant_has directly.
        assert!(!grant_has("never-enabled-plugin", "deepseek.balance"));
    }
}
