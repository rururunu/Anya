use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::models::settings::{AppSettings, McpServerConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::process::McpProcess;
use super::remote_auth;

struct NamedMcpTool {
    full_name: String,
    server_id: String,
    local_name: String,
    description: String,
    input_schema: Value,
}

impl Tool for NamedMcpTool {
    fn name(&self) -> &str {
        &self.full_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }
    /// Hidden from model-facing schemas when the owning server is disconnected.
    fn available(&self) -> bool {
        shared_mcp_manager().is_server_connected(&self.server_id)
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        shared_mcp_manager().call(&self.server_id, &self.local_name, args)
    }
}
pub struct McpManager {
    servers: Mutex<Vec<McpServerConfig>>,
    processes: Mutex<HashMap<String, McpProcess>>,
    /// Serialize connect/reconnect so settings-save registration and UI actions
    /// cannot open two OAuth browser flows for the same server at once.
    connect_lock: Mutex<()>,
}

impl McpManager {
    /// Creates an MCP manager with no configured servers.
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(Vec::new()),
            processes: Mutex::new(HashMap::new()),
            connect_lock: Mutex::new(()),
        }
    }

    /// Reloads server configs from application settings.
    pub fn configure(&self, settings: &AppSettings) {
        remote_auth::configure_smithery_api_key(&settings.smithery_api_key);
        let mut servers = settings.mcp_servers.clone();
        let _ = remote_auth::normalize_mcp_servers(&mut servers);
        if let Ok(mut s) = self.servers.lock() {
            *s = servers;
        }
        if let Ok(mut p) = self.processes.lock() {
            p.clear();
        }
    }

    /// Returns runtime connection status for every configured MCP server.
    pub fn list_runtime_statuses(&self) -> Vec<remote_auth::McpServerRuntimeStatus> {
        let servers = self
            .servers
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        servers
            .iter()
            .map(|server| {
                remote_auth::runtime_status_for(server, self.is_server_connected(&server.id))
            })
            .collect()
    }

    /// Returns runtime status for one MCP server by id.
    pub fn runtime_status_by_id(&self, id: &str) -> Option<remote_auth::McpServerRuntimeStatus> {
        let servers = self.servers.lock().ok()?;
        let server = servers.iter().find(|s| s.id == id)?;
        Some(remote_auth::runtime_status_for(
            server,
            self.is_server_connected(&server.id),
        ))
    }

    /// Looks up a configured MCP server by id.
    pub fn find_server(&self, id: &str) -> Option<McpServerConfig> {
        self.servers
            .lock()
            .ok()
            .and_then(|servers| servers.iter().find(|s| s.id == id).cloned())
    }

    /// Whether at least one enabled MCP server is configured; the `connect_tools`
    /// tool hides itself when there is nothing to connect.
    pub fn has_enabled_servers(&self) -> bool {
        self.servers
            .lock()
            .ok()
            .is_some_and(|servers| servers.iter().any(|server| server.enabled))
    }

    /// Whether a server's process is currently alive (its tools are usable).
    pub fn is_server_connected(&self, server_id: &str) -> bool {
        self.processes
            .lock()
            .ok()
            .is_some_and(|processes| processes.contains_key(server_id))
    }

    /// Connects all enabled local MCP servers and registers their tools.
    pub fn register_enabled(&self, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;

        registry.unregister_dynamic_prefix("mcp__");
        if let Ok(mut procs) = self.processes.lock() {
            procs.clear();
        }

        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let mut count = 0usize;
        let mut budget = crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS;
        for server in servers.into_iter().filter(|s| s.enabled) {
            if remote_auth::uses_mcp_remote(&server) {
                eprintln!(
                    "MCP server `{}` skipped auto-connect (mcp-remote; use Connect in Settings)",
                    server.id
                );
                continue;
            }
            if budget == 0 {
                eprintln!(
                    "MCP registration stopped: reached MCP_MAX_TOTAL_TOOLS ({})",
                    crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS
                );
                break;
            }
            match self.connect_server_with_budget(&server, registry, budget) {
                Ok(n) => {
                    count += n;
                    budget = budget.saturating_sub(n);
                }
                Err(error) => {
                    eprintln!("MCP server `{}` failed to connect: {error}", server.id);
                }
            }
        }
        Ok(count)
    }

    /// Spawns one MCP server process and registers its tools.
    pub fn connect_server(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
    ) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;
        self.connect_server_with_budget(
            server,
            registry,
            crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS,
        )
    }

    fn connect_server_with_budget(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
        remaining_budget: usize,
    ) -> Result<usize, ToolError> {
        let mut proc = McpProcess::spawn(server)?;
        let tools = proc.list_tools()?;
        {
            let mut procs = self
                .processes
                .lock()
                .map_err(|_| ToolError::new("mcp lock"))?;
            procs.insert(server.id.clone(), proc);
        }
        let per_server_cap =
            crate::core::chat::limits::MCP_MAX_TOOLS_PER_SERVER.min(remaining_budget);
        let mut registered = 0usize;
        let mut skipped = 0usize;
        for tool in tools {
            if registered >= per_server_cap {
                skipped += 1;
                continue;
            }
            let name = tool
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let description = truncate_mcp_text(
                tool.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("MCP tool"),
                crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS / 4,
            );
            let input_schema = truncate_mcp_schema(
                tool.get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            );
            let full_name = format!("mcp__{}__{name}", server.id);
            registry.register_dynamic(Arc::new(NamedMcpTool {
                full_name,
                server_id: server.id.clone(),
                local_name: name,
                description,
                input_schema,
            }));
            registered += 1;
        }
        if skipped > 0 {
            eprintln!(
                "MCP server `{}`: registered {registered} tools, skipped {skipped} (cap {})",
                server.id, per_server_cap
            );
        }
        Ok(registered)
    }

    /// Connects a configured MCP server by id.
    pub fn connect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ToolError::new(format!("unknown MCP server `{id}`")))?;
        self.connect_server(&server, registry)
    }

    /// Stops an MCP server and unregisters its tools.
    pub fn disconnect_by_id(&self, id: &str, registry: &ToolRegistry) {
        if let Ok(mut procs) = self.processes.lock() {
            procs.remove(id);
        }
        registry.unregister_dynamic_prefix(&format!("mcp__{id}__"));
    }

    /// Restarts an MCP server process and re-registers its tools.
    pub fn reconnect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;
        self.disconnect_by_id(id, registry);
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ToolError::new(format!("unknown MCP server `{id}`")))?;
        self.connect_server_with_budget(
            &server,
            registry,
            crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS,
        )
    }

    /// Invokes a tool on a connected MCP server.
    pub fn call(&self, server_id: &str, tool_name: &str, args: Value) -> Result<String, ToolError> {
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?;
        let proc = procs
            .get_mut(server_id)
            .ok_or_else(|| ToolError::new(format!("MCP server `{server_id}` is not connected")))?;
        proc.call_tool(tool_name, args)
    }
}

fn truncate_mcp_text(text: &str, max_chars: usize) -> String {
    crate::core::chat::limits::truncate_chars(text, max_chars)
}

fn truncate_mcp_schema(schema: Value) -> Value {
    let serialized = schema.to_string();
    if serialized.chars().count() <= crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS {
        return schema;
    }
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true,
        "description": format!(
            "Original inputSchema truncated ({} chars > {}).",
            serialized.chars().count(),
            crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS
        )
    })
}

/// Returns the process-wide shared MCP manager singleton.
pub fn shared_mcp_manager() -> Arc<McpManager> {
    static MANAGER: OnceLock<Arc<McpManager>> = OnceLock::new();
    Arc::clone(MANAGER.get_or_init(|| Arc::new(McpManager::new())))
}
