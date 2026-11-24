use serde::Serialize;
use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::core::mcp::{
    clear_saved_credentials, runtime_support, shared_mcp_manager, uses_mcp_remote,
    McpRuntimeSupport, McpServerRuntimeStatus,
};

#[tauri::command]
pub fn get_mcp_runtime_support() -> McpRuntimeSupport {
    runtime_support()
}

/// Snapshot of connect / OAuth credential state for the settings UI.
#[tauri::command]
pub fn list_mcp_server_statuses() -> Vec<McpServerRuntimeStatus> {
    shared_mcp_manager().list_runtime_statuses()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConnectResult {
    pub server_id: String,
    pub tool_count: usize,
    pub status: McpServerRuntimeStatus,
}

/// Connect (or reconnect) one configured MCP server and register its tools.
/// For mcp-remote bridges, a browser window may open when credentials are missing.
#[tauri::command]
pub async fn connect_mcp_server(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<McpConnectResult, String> {
    let registry = state.core.tools().registry();
    let manager = shared_mcp_manager();
    let id = server_id.clone();
    let tool_count = tauri::async_runtime::spawn_blocking(move || {
        manager
            .reconnect_by_id(&id, registry.as_ref())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;

    let status = shared_mcp_manager()
        .runtime_status_by_id(&server_id)
        .ok_or_else(|| format!("unknown MCP server `{server_id}`"))?;

    Ok(McpConnectResult {
        server_id,
        tool_count,
        status,
    })
}

/// Clear saved mcp-remote OAuth files, then connect again (forces a fresh browser login).
#[tauri::command]
pub async fn reauthenticate_mcp_server(
    _app: AppHandle,
    state: State<'_, AppState>,
    server_id: String,
) -> Result<McpConnectResult, String> {
    let manager = shared_mcp_manager();
    let server = manager
        .find_server(&server_id)
        .ok_or_else(|| format!("unknown MCP server `{server_id}`"))?;
    if !uses_mcp_remote(&server) {
        return Err("this server does not use mcp-remote OAuth".into());
    }
    let _ = clear_saved_credentials(&server)?;

    let registry = state.core.tools().registry();
    let id = server_id.clone();
    let tool_count = tauri::async_runtime::spawn_blocking(move || {
        manager
            .reconnect_by_id(&id, registry.as_ref())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;

    let status = shared_mcp_manager()
        .runtime_status_by_id(&server_id)
        .ok_or_else(|| format!("unknown MCP server `{server_id}`"))?;

    Ok(McpConnectResult {
        server_id,
        tool_count,
        status,
    })
}
