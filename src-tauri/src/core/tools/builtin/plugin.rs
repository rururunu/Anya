//! Agent-facing plugin lifecycle — same pattern as `manage_custom_theme`.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::plugins::diagnostics::list_plugin_errors;
use crate::core::plugins::{
    create_plugin, delete_plugin, describe_contract, ensure_plugin_icon, export_plugin_zip,
    get_grant, import_plugin_from_path, is_official_plugin_id, list_plugin_files, list_plugins,
    load_manifest, open_plugin_window, peek_plugin_id, plugin_dir, put_plugin_file,
    read_plugin_file, sanitize_plugin_id, shared_runtime,
};
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;
use crate::core::tools::path::resolve_path_candidate;

/// How long `enable`/`reload` wait for the webview to report activate() success/failure
/// after a rebundle, before returning "no runtime signal" instead of blocking forever.
const RELOAD_CHECK_TIMEOUT: Duration = Duration::from_millis(3000);

pub struct ManagePluginTool;

impl Tool for ManagePluginTool {
    fn name(&self) -> &str {
        "manage_plugin"
    }
    fn description(&self) -> &str {
        "Read/create Anya user plugins. Always manage_plugin — never read_file/list_folder/Grep/run_shell on %APPDATA% or guessed plugin.json paths. After create/put_file you enable/reload yourself — never ask the user to Enable, Reload, or restart Anya. import/export pack a zip or folder (does not auto-enable). For surfaces, UI entry, ctx.*, theming, or 'path not allowed', load_skill plugin_creator or describe_contract; do not grep Anya src/src-tauri. Inspect with list → list_files → get_file. In plan mode only list/list_files/get_file/describe_contract/errors; create/put_file/enable/import/export wait until the plan is approved. Pass files:[{path,contents}] on create or put_file. Local file pick is ctx.fs.pick (permission fs.pick)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "put_file", "get_file", "list", "list_files", "open", "enable", "disable", "reload", "delete", "errors", "describe_contract", "import", "export"],
                    "description": "create scaffold, write a file, read a file, list plugins, list files in one plugin, open window, enable/disable/reload host+UI, delete (user plugins only, not official), errors, describe_contract, import a zip/folder, or export a zip"
                },
                "id": {
                    "type": "string",
                    "description": "Plugin id (kebab-case). Required except list."
                },
                "name": { "type": "string", "description": "Display name. Required for create." },
                "description": { "type": "string", "description": "One-line purpose. For create." },
                "path": {
                    "type": "string",
                    "description": "Relative path inside the plugin for put_file/get_file, e.g. ui/src/activate.js or plugin.json. For import/export: workspace or absolute path to a zip/folder (import) or dest zip/folder (export)."
                },
                "contents": {
                    "type": "string",
                    "description": "File text for a single put_file. Max 4MB per file. JS/TS/HTML/CSS/JSON/images only — no exe, cdylib, or Cargo."
                },
                "files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "contents": { "type": "string" }
                        },
                        "required": ["path", "contents"]
                    },
                    "description": "Batch write for create/put_file: write several files (e.g. plugin.json, ui/src/activate.js, host/main.ts) in this one call instead of one tool call per file."
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "For import: replace an existing plugin with the same id (never official ids). Imported plugins are not enabled."
                },
                "permissions": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Permission ids to grant on enable (must be declared in plugin.json)."
                }
            },
            "required": ["action"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let action = args["action"].as_str().unwrap_or("").trim();
        reject_plan_mode_plugin_write(ctx, action)?;
        match action {
            "list" => {
                let items = list_plugins()?;
                if items.is_empty() {
                    return Ok(format!(
                        "No user plugins yet. Directory: {}",
                        plugins_dir().display()
                    ));
                }
                Ok(serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into()))
            }
            "create" => {
                let id = args["id"].as_str().unwrap_or("");
                let name = args["name"].as_str().unwrap_or("");
                let description = args["description"].as_str().unwrap_or("");
                let manifest = create_plugin(id, name, description)?;
                let written = write_batch_files(&manifest.id, &args["files"])?;
                if !written.is_empty() {
                    let _ = load_manifest(&manifest.id)?;
                }
                ensure_plugin_icon(&manifest.id)?;
                let extra = if written.is_empty() {
                    String::new()
                } else {
                    format!(" Also wrote: {}.", written.join(", "))
                };
                let live = apply_live_plugin(ctx, &manifest.id, true);
                Ok(format!(
                    "Created plugin `{}` ({}). Scaffold: ui/src/activate.js, ui/icon.svg, ui/index.html, host/main.ts.{extra} Do not edit Anya source. {live}",
                    manifest.id, manifest.name
                ))
            }
            "get_file" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let path = args["path"].as_str().unwrap_or("").trim();
                if path.is_empty() {
                    return Err(ToolError::new(
                        "get_file needs path (relative inside the plugin). Call list_files first if you don't know it.",
                    ));
                }
                read_plugin_file(&id, path)
            }
            "list_files" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let files = list_plugin_files(&id)?;
                if files.is_empty() {
                    return Ok(format!("Plugin `{id}` has no listed source files."));
                }
                Ok(files.join("\n"))
            }
            "put_file" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let mut written = write_batch_files(&id, &args["files"])?;
                let path = args["path"].as_str().unwrap_or("");
                let contents = args["contents"].as_str().unwrap_or("");
                if !path.is_empty() || !contents.is_empty() {
                    if path.is_empty() {
                        return Err(ToolError::new("path is required for put_file"));
                    }
                    if contents.is_empty() {
                        return Err(ToolError::new("contents is required for put_file"));
                    }
                    let dest = put_plugin_file(&id, path, contents)?;
                    written.push(dest.display().to_string());
                }
                if written.is_empty() {
                    return Err(ToolError::new(
                        "put_file needs either path+contents or a files:[...] array",
                    ));
                }
                let _ = load_manifest(&id)?;
                ensure_plugin_icon(&id)?;
                let live = apply_live_plugin(ctx, &id, false);
                Ok(format!("Wrote {}. {live}", written.join(", ")))
            }
            "open" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let Some(app) = &ctx.app_handle else {
                    return Err(ToolError::new("app context unavailable"));
                };
                open_plugin_window(app, &id)?;
                Ok(format!("Opened plugin `{id}`"))
            }
            "enable" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let manifest = load_manifest(&id)?;
                let permissions = args["permissions"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| manifest.permissions.clone());
                let since = now_ms();
                shared_runtime().enable(&id, permissions)?;
                let signal = wait_for_runtime_signal(ctx, &id, since);
                Ok(format!(
                    "Enabled plugin `{id}` (UI bundled and loaded into the open workbench). {signal}"
                ))
            }
            "disable" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                shared_runtime().disable(&id)?;
                Ok(format!("Disabled plugin `{id}`"))
            }
            "reload" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let since = now_ms();
                shared_runtime().reload(&id)?;
                let signal = wait_for_runtime_signal(ctx, &id, since);
                Ok(format!("Reloaded plugin `{id}` (UI rebundled OK). {signal}"))
            }
            "errors" => {
                let id = args["id"].as_str().filter(|s| !s.is_empty());
                let id = id.map(sanitize_plugin_id).transpose()?;
                let records: Vec<_> = list_plugin_errors(id.as_deref(), None)
                    .into_iter()
                    .filter(|r| r.phase != "reload-ok")
                    .collect();
                if records.is_empty() {
                    return Ok("No runtime errors recorded (only faults reported by an open workbench webview show up here; bundling errors are already returned inline by enable/reload).".into());
                }
                Ok(serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".into()))
            }
            "describe_contract" => Ok(serde_json::to_string_pretty(&describe_contract())
                .unwrap_or_else(|_| "{}".into())),
            "delete" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                if is_official_plugin_id(&id) {
                    return Err(ToolError::new(format!(
                        "cannot uninstall official plugin `{id}`"
                    )));
                }
                shared_runtime().uninstall_cleanup(&id)?;
                delete_plugin(&id)?;
                Ok(format!("Deleted plugin `{id}`"))
            }
            "import" => {
                let raw = args["path"].as_str().unwrap_or("").trim();
                if raw.is_empty() {
                    return Err(ToolError::new("import needs path (zip or folder with plugin.json)"));
                }
                let path = resolve_path_candidate(&ctx.workspace_root, raw)?;
                let overwrite = args["overwrite"].as_bool().unwrap_or(false);
                if overwrite {
                    if let Ok(id) = peek_plugin_id(&path) {
                        if is_official_plugin_id(&id) {
                            return Err(ToolError::new(format!(
                                "cannot import over official plugin `{id}`"
                            )));
                        }
                        let dest = plugin_dir(&id)?;
                        if dest.join("plugin.json").is_file() {
                            shared_runtime().uninstall_cleanup(&id)?;
                        }
                    }
                }
                let id = import_plugin_from_path(&path, overwrite)?;
                Ok(format!(
                    "Imported plugin `{id}` (not enabled). Call action:enable, or let the user Enable it. Do not auto-trust packs."
                ))
            }
            "export" => {
                let id = sanitize_plugin_id(args["id"].as_str().unwrap_or(""))?;
                let raw = args["path"].as_str().unwrap_or("").trim();
                let dest = if raw.is_empty() {
                    ctx.workspace_root.clone()
                } else {
                    resolve_path_candidate(&ctx.workspace_root, raw)?
                };
                let out = export_plugin_zip(&id, &dest)?;
                Ok(format!("Exported plugin `{id}` to {}", out.display()))
            }
            _ => Err(ToolError::new(
                "action must be create, put_file, get_file, list, list_files, open, enable, disable, reload, delete, errors, describe_contract, import, or export",
            )),
        }
    }
}

const PLAN_MODE_PLUGIN_ACTIONS: &[&str] = &[
    "list",
    "list_files",
    "get_file",
    "describe_contract",
    "errors",
];

fn reject_plan_mode_plugin_write(ctx: &ToolContext, action: &str) -> Result<(), ToolError> {
    if !crate::core::tools::plan_mode::shared_plan_mode_store().is_active(ctx.root_session_id()) {
        return Ok(());
    }
    if PLAN_MODE_PLUGIN_ACTIONS.contains(&action) {
        return Ok(());
    }
    Err(ToolError::new(format!(
        "plan mode: manage_plugin `{action}` writes. Allowed now: {}. After the user approves the plan, use create/put_file/enable/disable/reload/delete/open/import/export.",
        PLAN_MODE_PLUGIN_ACTIONS.join(", ")
    )))
}

/// Writes every `{path, contents}` entry in `files` (a JSON array, or anything
/// else which is treated as "no batch files"). Lets `create`/`put_file` write
/// a whole plugin's sources in one tool call instead of one call per file.
fn write_batch_files(id: &str, files: &Value) -> Result<Vec<String>, ToolError> {
    let Some(entries) = files.as_array() else {
        return Ok(Vec::new());
    };
    let mut written = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry["path"].as_str().unwrap_or("").trim();
        let contents = entry["contents"].as_str().unwrap_or("");
        if path.is_empty() {
            return Err(ToolError::new("files[].path is required"));
        }
        let dest = put_plugin_file(id, path, contents)?;
        written.push(dest.display().to_string());
    }
    Ok(written)
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Enable a newly created plugin, or reload one that is already enabled.
fn apply_live_plugin(ctx: &ToolContext, id: &str, created: bool) -> String {
    let since = now_ms();
    let result = if created {
        let permissions = load_manifest(id).map(|m| m.permissions).unwrap_or_default();
        shared_runtime().enable(id, permissions)
    } else if get_grant(id).enabled {
        shared_runtime().reload(id)
    } else {
        return "Plugin is not enabled — call action:enable so the workbench loads it. Do not ask the user to click Enable or restart Anya.".into();
    };
    match result {
        Ok(()) => {
            let signal = wait_for_runtime_signal(ctx, id, since);
            if created {
                format!("Enabled and loaded into the open workbench. {signal}")
            } else {
                format!("Reloaded live UI. {signal}")
            }
        }
        Err(err) => format!(
            "Live load failed ({err}). Call action:enable (or reload if already enabled). Do not ask the user to click Enable, Reload, or restart Anya."
        ),
    }
}

/// Asks an open workbench webview to re-activate `id` (the runtime already
/// emitted `plugin-reload-request`) and waits briefly for activate() to report
/// back. Best-effort: if no webview is open, times out with a caveat instead of
/// hanging or claiming false confidence.
fn wait_for_runtime_signal(ctx: &ToolContext, id: &str, since_ms: u64) -> String {
    if ctx.app_handle.is_none() {
        return "Runtime check skipped: no app handle in this context.".into();
    }
    let deadline = Instant::now() + RELOAD_CHECK_TIMEOUT;
    while Instant::now() < deadline {
        let records = list_plugin_errors(Some(id), Some(since_ms));
        if let Some(latest) = records.last() {
            return if latest.phase == "reload-ok" {
                "Runtime check: workbench re-activated the plugin with no error.".into()
            } else {
                format!(
                    "Runtime check: activate() reported an error after reload — [{}] {}",
                    latest.phase, latest.message
                )
            };
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    "Runtime check: no signal within 3s — workbench may be closed. Host/UI bundle succeeded; the plugin will load the next time Workbench opens. Do not ask the user to restart Anya.".into()
}
