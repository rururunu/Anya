//! Slash commands, MCP, LSP, and codebase index builtin tools.

use serde_json::{json, Value};

use crate::core::event::BusEvent;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;

pub(super) struct RunSlashCommandTool;

impl Tool for RunSlashCommandTool {
    fn name(&self) -> &str {
        "run_slash_command"
    }
    fn description(&self) -> &str {
        "Run a slash command. Known commands: history, model, plan, settings, work, exit, compact, clear, context. Emits a UI event for frontend-handled commands."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Command name without leading slash" },
                "args": { "type": "string" }
            },
            "required": ["command"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let raw = args["command"].as_str().unwrap_or("").trim();
        let command = raw.trim_start_matches('/').to_ascii_lowercase();
        let extra = args["args"].as_str().unwrap_or("").trim().to_string();
        if command.is_empty() {
            return Err(ToolError::new("command is required"));
        }

        let known = [
            "history", "model", "plan", "settings", "work", "exit", "compact", "clear", "context",
        ];
        if !known.contains(&command.as_str()) {
            return Err(ToolError::new(format!(
                "unknown slash command `/{command}`. Known: {}",
                known.map(|c| format!("/{c}")).join(", ")
            )));
        }

        match command.as_str() {
            "context" => serde_json::to_string_pretty(&ctx.request_context)
                .map_err(|error| ToolError::new(error.to_string())),
            "compact" => Ok(
                "Slash /compact acknowledged — context compaction runs automatically when history exceeds the size threshold."
                    .into(),
            ),
            "clear" => {
                ctx.event_bus.emit(BusEvent::SlashCommand {
                    session_id: ctx.root_session_id().to_string(),
                    command: command.clone(),
                    args: extra,
                });
                Ok("Slash /clear requested — frontend should clear the visible conversation.".into())
            }
            other => {
                ctx.event_bus.emit(BusEvent::SlashCommand {
                    session_id: ctx.root_session_id().to_string(),
                    command: other.to_string(),
                    args: extra.clone(),
                });
                Ok(format!(
                    "Slash /{other}{} dispatched to UI",
                    if extra.is_empty() {
                        String::new()
                    } else {
                        format!(" {extra}")
                    }
                ))
            }
        }
    }
}

pub(super) struct ConnectToolsTool;

impl Tool for ConnectToolsTool {
    fn name(&self) -> &str {
        "connect_tools"
    }
    fn description(&self) -> &str {
        "Connect a configured MCP server by id and register its tools as mcp__{id}__{tool}. With reconnect=true, disconnect first and reconnect to refresh the server's tools."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "description": "MCP server id from Settings" },
                "reconnect": { "type": "boolean", "default": false, "description": "Disconnect first, then reconnect to refresh the server's tools" }
            },
            "required": ["source"]
        })
    }
    fn available(&self) -> bool {
        crate::core::mcp::shared_mcp_manager().has_enabled_servers()
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let source = args["source"].as_str().unwrap_or("");
        let registry = ctx
            .registry
            .as_ref()
            .ok_or_else(|| ToolError::new("tool registry unavailable"))?;
        let count = if args["reconnect"].as_bool().unwrap_or(false) {
            crate::core::mcp::shared_mcp_manager().reconnect_by_id(source, registry)?
        } else {
            crate::core::mcp::shared_mcp_manager().connect_by_id(source, registry)?
        };
        Ok(format!("connected MCP `{source}` with {count} tools"))
    }
}

pub(super) struct InstallToolSourceTool;

impl Tool for InstallToolSourceTool {
    fn name(&self) -> &str {
        "install_tool_source"
    }
    fn description(&self) -> &str {
        "Install an MCP or tool source package."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "url": { "type": "string" } },
            "required": ["url"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        Ok(format!(
            "install queued for {}",
            args["url"].as_str().unwrap_or("")
        ))
    }
}

pub(super) struct LspTool;

impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }
    fn description(&self) -> &str {
        "Language-aware navigation via LSP: hover, definition, diagnostics, references, workspace symbols, rename preview, and code actions. Prefer when available for precise symbol navigation; fall back to search_files / list_symbols / read_file when LSP is off or unavailable. Requires LSP enabled in Settings."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["hover", "definition", "diagnostics", "references", "workspace_symbol", "rename", "code_action"],
                    "description": "hover/definition/diagnostics/references/workspace_symbol/rename/code_action"
                },
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "query": { "type": "string", "description": "Query for workspace_symbol action" },
                "line": { "type": "integer" },
                "character": { "type": "integer" },
                "new_name": { "type": "string", "description": "New symbol name for rename action" }
            },
            "required": ["action", "path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn available(&self) -> bool {
        crate::core::lsp::shared_lsp_manager().is_enabled()
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let manager = crate::core::lsp::shared_lsp_manager();
        let path = args["path"].as_str().unwrap_or("");
        match args["action"].as_str().unwrap_or("") {
            "hover" => manager.hover(
                &ctx.workspace_root,
                path,
                args["line"].as_u64().unwrap_or(0),
                args["character"].as_u64().unwrap_or(0),
            ),
            "definition" => manager.definition(
                &ctx.workspace_root,
                path,
                args["line"].as_u64().unwrap_or(0),
                args["character"].as_u64().unwrap_or(0),
            ),
            "diagnostics" => manager.diagnostics(&ctx.workspace_root, path),
            "references" => manager.references(
                &ctx.workspace_root,
                path,
                args["line"].as_u64().unwrap_or(0),
                args["character"].as_u64().unwrap_or(0),
            ),
            "workspace_symbol" => {
                manager.workspace_symbol(&ctx.workspace_root, args["query"].as_str().unwrap_or(""))
            }
            "rename" => manager.rename(
                &ctx.workspace_root,
                path,
                args["line"].as_u64().unwrap_or(0),
                args["character"].as_u64().unwrap_or(0),
                args["new_name"].as_str().unwrap_or(""),
            ),
            "code_action" => manager.code_action(
                &ctx.workspace_root,
                path,
                args["line"].as_u64().unwrap_or(0),
                args["character"].as_u64().unwrap_or(0),
            ),
            other => Err(ToolError::new(format!("unsupported lsp action: {other}"))),
        }
    }
}

pub(super) struct SearchCodebaseTool;

impl Tool for SearchCodebaseTool {
    fn name(&self) -> &str {
        "search_codebase"
    }
    fn description(&self) -> &str {
        "Search the workspace index for symbols, file paths, and decision docs (AGENTS.md / ADR). Prefer this over mem0 for codebase knowledge. Builds the index on first use."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Symbol name, path fragment, or short topic" },
                "limit": { "type": "integer", "minimum": 1, "default": 12 }
            },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let query = args["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return Err(ToolError::new("query is required"));
        }
        let limit = args["limit"].as_u64().unwrap_or(12) as usize;
        let index = crate::core::tools::workspace_index::WorkspaceIndex::open(&ctx.workspace_root)
            .map_err(ToolError::new)?;
        let hits = index.search(query, limit).map_err(ToolError::new)?;
        if hits.is_empty() {
            return Ok("No index hits.".into());
        }
        let mut lines = Vec::new();
        for hit in hits {
            let symbol = hit.symbol.unwrap_or_default();
            lines.push(format!(
                "[{}] {}{} — {}",
                hit.kind,
                hit.path,
                if symbol.is_empty() {
                    String::new()
                } else {
                    format!("::{symbol}")
                },
                hit.snippet
            ));
        }
        Ok(lines.join("\n"))
    }
}

pub(super) struct RebuildCodebaseIndexTool;

impl Tool for RebuildCodebaseIndexTool {
    fn name(&self) -> &str {
        "rebuild_codebase_index"
    }
    fn description(&self) -> &str {
        "Rebuild the workspace symbol/path index under .anya/index. Use after large refactors when search_codebase looks stale."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let index = crate::core::tools::workspace_index::WorkspaceIndex::open(&ctx.workspace_root)
            .map_err(ToolError::new)?;
        let count = index.rebuild().map_err(ToolError::new)?;
        Ok(format!("indexed {count} records into .anya/index"))
    }
}
