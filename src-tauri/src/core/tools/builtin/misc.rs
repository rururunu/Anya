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
        "Search the workspace index for symbols, file paths, decision docs (AGENTS.md / ADR), and full-text content chunks. Prefer this over mem0 for codebase knowledge. Builds/refreshes the index on use."
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
        // Retrieve a larger keyword candidate set, then re-rank semantically
        // when the embedding model is ready (retrieve-then-rerank).
        let candidate_limit = limit.max(30).min(80);
        let mut hits = index
            .search(query, candidate_limit)
            .map_err(ToolError::new)?;
        if hits.len() > 1 && crate::core::ai::embed::SemanticSearchEngine::is_ready() {
            let passages: Vec<String> = hits.iter().map(|h| h.snippet.clone()).collect();
            if let Ok(scores) =
                crate::core::ai::embed::SemanticSearchEngine::rerank(query, &passages)
            {
                for (hit, score) in hits.iter_mut().zip(scores.iter()) {
                    hit.score = (*score * 1000.0) as i32;
                }
                hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
            }
        }
        hits.truncate(limit.max(1));
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

/// Push a desktop file to the phone Companion for preview/download.
/// Emits metadata-only `file.offer`; Companion then pulls bytes via workspace.readFile download.
pub(super) struct ShareToCompanionTool;

impl Tool for ShareToCompanionTool {
    fn name(&self) -> &str {
        "share_to_companion"
    }
    fn description(&self) -> &str {
        "Send a real file the user should open. Desktop shows a file card that reveals it in Explorer; \
the paired phone shows a card and downloads original bytes only after the user taps it. \
Call once per file (icons, SVGs, images, PDFs, docs, zip, etc.). ALWAYS pass the source file path itself — \
never invent a substitute HTML/preview page, collage, or wrapper. Prefer workspace-relative paths; \
absolute paths outside the workspace are copied into .anya/shared/ first. For a running local web app, \
use share_preview_url instead."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to workspace root, or an absolute path on this machine"
                },
                "label": {
                    "type": "string",
                    "description": "Optional display name shown on the phone"
                }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let raw = args["path"].as_str().unwrap_or("").trim();
        if raw.is_empty() {
            return Err(ToolError::new("path is required"));
        }
        let label = args["label"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let (rel_path, absolute, workspace_id) = prepare_share_path(ctx, raw)?;
        let meta = std::fs::metadata(&absolute).map_err(|e| ToolError::new(e.to_string()))?;
        if !meta.is_file() {
            return Err(ToolError::new("path is not a file"));
        }
        let size = meta.len();
        let max_share = crate::core::remote::MAX_UPLOAD_BYTES;
        if size > max_share {
            return Err(ToolError::new(format!(
                "file too large to share ({size} bytes; max {max_share})"
            )));
        }
        let name = label
            .map(str::to_string)
            .or_else(|| {
                absolute
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| rel_path.clone());
        let mime = guess_share_mime(&absolute);
        let offer_id = uuid::Uuid::new_v4().to_string();
        ctx.event_bus.emit(BusEvent::FileOffer {
            session_id: ctx.root_session_id().to_string(),
            offer_id: offer_id.clone(),
            path: rel_path.clone(),
            absolute_path: absolute.to_string_lossy().to_string(),
            name: name.clone(),
            mime: mime.clone(),
            size,
            workspace_id,
        });
        Ok(format!(
            "Offered `{name}` (path={rel_path}, mime={mime}, size={size}, offerId={offer_id}). \
The desktop shows a file card; the phone shows a card and downloads only after the user taps it."
        ))
    }
}

/// Publish a loopback HTTP preview through the Companion gateway reverse proxy.
pub(super) struct SharePreviewUrlTool;

impl Tool for SharePreviewUrlTool {
    fn name(&self) -> &str {
        "share_preview_url"
    }
    fn description(&self) -> &str {
        "Share a local web preview (http://127.0.0.1 or http://localhost) so the user can open it. \
The tool registers the origin on the existing Companion gateway and returns a proxied public/LAN \
URL under /p/{id}/ — always give the user that address, never raw localhost. Optional label. \
Use after starting a Vite/Next/static server, or whenever the deliverable is a running web app."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Local preview URL, must be http://127.0.0.1 or http://localhost with a port"
                },
                "label": {
                    "type": "string",
                    "description": "Optional display name shown on the card"
                }
            },
            "required": ["url"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let raw = args["url"].as_str().unwrap_or("").trim();
        if raw.is_empty() {
            return Err(ToolError::new("url is required"));
        }
        let label = args["label"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Preview")
            .to_string();
        let app = ctx
            .app_handle
            .as_ref()
            .ok_or_else(|| ToolError::new("app handle unavailable"))?;
        let state = crate::core::remote::remote_state(app);
        if !state.is_running() {
            crate::core::remote::start_gateway(app.clone(), None).map_err(ToolError::new)?;
        }
        let preview =
            crate::core::remote::preview::register_preview(app, raw, ctx.root_session_id())
                .map_err(ToolError::new)?;
        let public_url = crate::core::remote::preview::public_preview_url(app, &preview.id);
        let offer_id = preview.id.clone();
        ctx.event_bus.emit(BusEvent::UrlOffer {
            session_id: ctx.root_session_id().to_string(),
            offer_id: offer_id.clone(),
            label: label.clone(),
            origin_url: preview.origin_url.clone(),
            public_url: public_url.clone(),
        });
        Ok(format!(
            "Preview `{label}` is available at {public_url} (origin {raw}, offerId={offer_id}). \
Give the user this proxied address, not localhost."
        ))
    }
}

/// Prefer a registered workspace the Companion can `workspace.readFile` against.
/// Agent turns without a workspace fall back to `peek-public`, which is NOT a
/// downloadable workspace — shares must land in a real workspace root instead.
fn resolve_share_workspace(
    ctx: &ToolContext,
) -> Result<(std::path::PathBuf, Option<String>), ToolError> {
    use crate::core::tools::path::normalize_path;
    use tauri::Manager;

    if let Some(app) = ctx.app_handle.as_ref() {
        if let Some(state) = app.try_state::<crate::app_state::AppState>() {
            let manager = state.core.workspaces();
            let list = manager.list();

            if let Some(id) = ctx
                .conversation
                .workspace_for_session(ctx.root_session_id())
            {
                if let Some(ws) = list.iter().find(|w| w.id == id) {
                    return Ok((ws.root.clone(), Some(ws.id.clone())));
                }
            }

            let ctx_root = normalize_path(&ctx.workspace_root);
            if let Some(ws) = list.iter().find(|w| normalize_path(&w.root) == ctx_root) {
                return Ok((ws.root.clone(), Some(ws.id.clone())));
            }

            if let Some(ws) = manager.current().or_else(|| list.into_iter().next()) {
                return Ok((ws.root, Some(ws.id)));
            }
        }
    }

    Err(ToolError::new(
        "no workspace available to stage the file for Companion download — open a workspace on desktop first",
    ))
}

fn prepare_share_path(
    ctx: &ToolContext,
    raw: &str,
) -> Result<(String, std::path::PathBuf, Option<String>), ToolError> {
    use crate::core::tools::path::{normalize_path, resolve_path_candidate};

    // Do not call resolve_tool_path: outside-workspace reads would block on a
    // PathPermission prompt; sharing is already an explicit user/agent intent.
    let source = resolve_path_candidate(&ctx.workspace_root, raw)?;
    if !source.is_file() {
        // Absolute paths outside the agent workspace_root still resolve via
        // resolve_path_candidate when absolute; re-check raw absolute.
        let absolute = std::path::PathBuf::from(raw);
        if absolute.is_absolute() && absolute.is_file() {
            let (share_root, workspace_id) = resolve_share_workspace(ctx)?;
            return copy_into_shared(share_root, workspace_id, &absolute);
        }
        return Err(ToolError::new(format!("path is not a file: {raw}")));
    }

    let (share_root, workspace_id) = resolve_share_workspace(ctx)?;
    let share_norm = normalize_path(&share_root);
    let source_norm = normalize_path(&source);

    if source_norm.starts_with(&share_norm) {
        let rel = source_norm
            .strip_prefix(&share_norm)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| raw.replace('\\', "/"));
        return Ok((rel, source_norm, workspace_id));
    }

    copy_into_shared(share_root, workspace_id, &source_norm)
}

fn copy_into_shared(
    share_root: std::path::PathBuf,
    workspace_id: Option<String>,
    source: &std::path::Path,
) -> Result<(String, std::path::PathBuf, Option<String>), ToolError> {
    let shared_dir = share_root.join(".anya").join("shared");
    std::fs::create_dir_all(&shared_dir).map_err(|e| ToolError::new(e.to_string()))?;
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "shared.bin".into());
    let dest = shared_dir.join(&file_name);
    std::fs::copy(source, &dest).map_err(|e| ToolError::new(e.to_string()))?;
    Ok((format!(".anya/shared/{file_name}"), dest, workspace_id))
}

fn guess_share_mime(path: &std::path::Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("txt") | Some("log") | Some("md") => "text/plain",
        Some("json") => "application/json",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}
