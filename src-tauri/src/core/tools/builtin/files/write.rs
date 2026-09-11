//! File write and edit builtin tools.

use std::fs;

use regex::Regex;
use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::file_io::atomic_write;
use crate::core::tools::fuzzy::apply_old_string_edit;
use crate::core::tools::preview::{unified_diff, ChangeKind, ToolPreview};

use super::{
    apply_many_edits, format_match_error, guard_minimal_edit, required_string, resolve_write,
    single_edit_preview,
};

pub struct WriteFileTool;
pub struct ReplaceInFileTool;
pub struct ReplaceManyInFileTool;
pub struct MovePathTool;
pub struct EditNotebookCellTool;
pub struct DeleteTextRangeTool;
pub struct DeleteGoSymbolTool;

fn check_overwrite_permission(
    resolved: &std::path::Path,
    args: &Value,
    path: &str,
) -> Result<(), ToolError> {
    if resolved.exists() {
        let allow_overwrite = args
            .get("allow_overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || args
                .get("overwrite")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        if !allow_overwrite {
            return Err(ToolError::new(format!(
                "File '{path}' already exists. Full-file overwrites hide diffs and cause data loss. Use `replace_in_file` or `apply_patch` to edit this file, or pass `allow_overwrite: true` only if a complete rewrite is explicitly intended."
            )));
        }
    }
    Ok(())
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Create a brand-new file, or perform an explicitly requested full-file replacement. Path is relative to workspace root; parent directories are created automatically.

Usage:
- Use primarily for brand-new files.
- If the file already exists, this tool will FAIL unless `allow_overwrite: true` is explicitly passed.
- Never use for an existing file when only part of the content changes — a full rewrite hides the real diff, discards concurrent edits, and is the wrong tool.
- To modify an existing file, you MUST use `replace_in_file`, `replace_many_in_file`, or `apply_patch`."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "content": { "type": "string", "description": "Full file content to write" },
                "allow_overwrite": { "type": "boolean", "description": "Must be set to true if you explicitly intend to overwrite an existing file. If false or omitted and the file already exists, the write is rejected to prevent accidental data loss." }
            },
            "required": ["path", "content"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        check_overwrite_permission(&resolved, &args, path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, content)?;
        Ok("written".into())
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let new_text = args["content"].as_str().unwrap_or("").to_string();
        let resolved = resolve_write(ctx, self.name(), path)?;
        check_overwrite_permission(&resolved, args, path)?;
        let (kind, old_text) = if resolved.exists() {
            (
                ChangeKind::Modify,
                Some(fs::read_to_string(&resolved).unwrap_or_default()),
            )
        } else {
            (ChangeKind::Create, None)
        };
        let old = old_text.clone().unwrap_or_default();
        Ok(Some(ToolPreview {
            path: path.to_string(),
            affected_paths: vec![path.to_string()],
            kind,
            old_text,
            new_text: Some(new_text.clone()),
            unified_diff: unified_diff(path, &old, &new_text),
        }))
    }
}

impl Tool for ReplaceInFileTool {
    fn name(&self) -> &str {
        "replace_in_file"
    }
    fn description(&self) -> &str {
        "First choice for one localized change to an existing file. Path is relative to workspace root.

Usage:
- old_string and new_string must contain ONLY the lines that change, plus at most 1–2 unchanged lines as unique match context.
- Example one-line change — old: `const a = 1`, new: `const a = 2`.
- Never pass whole-file content or long unchanged blocks (rejected by the runtime).
- Prefer this over apply_patch for a single localized replacement, and over write_file for partial edits.
- Exact match first, then narrow fuzzy matching for whitespace/indentation."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "old_string": { "type": "string", "description": "ONLY the lines that change plus minimal unique context; never whole-file content" },
                "new_string": { "type": "string", "description": "The changed lines replacing old_string" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = required_string(&args, "path")?;
        let old = required_string(&args, "old_string")?;
        let new = args
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("new_string is required"))?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        guard_minimal_edit(self.name(), &[old], &content)?;
        let applied = apply_old_string_edit(&content, old, new, false);
        if applied.applied != 1 {
            return Err(format_match_error(
                "replace_in_file",
                path,
                applied.matches,
                Some(&content),
                Some(old),
            ));
        }
        atomic_write(&resolved, &applied.updated)?;
        if applied.fuzzy {
            Ok(format!(
                "replaced (fuzzy match, {} replacement)",
                applied.applied
            ))
        } else {
            Ok("replaced".into())
        }
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = required_string(args, "path")?;
        let old = required_string(args, "old_string")?;
        let new = args
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("new_string is required"))?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        guard_minimal_edit(self.name(), &[old], &content)?;
        single_edit_preview(path, content, old, new)
    }
}

impl Tool for ReplaceManyInFileTool {
    fn name(&self) -> &str {
        "replace_many_in_file"
    }
    fn description(&self) -> &str {
        "First choice for several independent localized changes in one existing file. Path is relative to workspace root.

Usage:
- Apply multiple unique replacements atomically (exact then narrow fuzzy matching), preserving all other content.
- Each old_string/new_string pair must contain ONLY the lines that change; whole-file edits are rejected.
- Prefer this over multiple replace_in_file calls when edits are independent and in the same file.
- Use apply_patch only when changes form a structural block rewrite or need contextual insertion/deletion across hunks."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_string": { "type": "string", "description": "ONLY the lines that change plus minimal unique context; never whole-file content" },
                            "new_string": { "type": "string", "description": "The changed lines replacing old_string" }
                        },
                        "required": ["old_string", "new_string"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = required_string(&args, "path")?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let olds = args["edits"]
            .as_array()
            .map(|edits| {
                edits
                    .iter()
                    .filter_map(|e| e.get("old_string").and_then(Value::as_str))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        guard_minimal_edit(self.name(), &olds, &content)?;
        let (updated, fuzzy_count) = apply_many_edits(&content, &args)?;
        let total = args["edits"].as_array().map_or(0, Vec::len);
        atomic_write(&resolved, updated)?;
        if fuzzy_count > 0 {
            Ok(format!(
                "replaced ({total} total, {fuzzy_count} fuzzy match)"
            ))
        } else {
            Ok("replaced".into())
        }
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = required_string(args, "path")?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let original = fs::read_to_string(&resolved)?;
        let olds = args["edits"]
            .as_array()
            .map(|edits| {
                edits
                    .iter()
                    .filter_map(|e| e.get("old_string").and_then(Value::as_str))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        guard_minimal_edit(self.name(), &olds, &original)?;
        let (content, _) = apply_many_edits(&original, args)?;
        Ok(Some(ToolPreview {
            path: path.to_string(),
            affected_paths: vec![path.to_string()],
            kind: ChangeKind::Modify,
            old_text: Some(original.clone()),
            new_text: Some(content.clone()),
            unified_diff: unified_diff(path, &original, &content),
        }))
    }
}

impl Tool for MovePathTool {
    fn name(&self) -> &str {
        "move_path"
    }
    fn description(&self) -> &str {
        "Move or rename a file or directory (paths relative to workspace root). Prefer this dedicated tool over shell Move-Item/mv for workspace paths."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "from": { "type": "string", "description": "Source path relative to workspace root" },
                "to": { "type": "string", "description": "Destination path relative to workspace root" }
            },
            "required": ["from", "to"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let from = resolve_write(ctx, self.name(), args["from"].as_str().unwrap_or(""))?;
        let to = resolve_write(ctx, self.name(), args["to"].as_str().unwrap_or(""))?;
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, to)?;
        Ok("moved".into())
    }
}

impl Tool for EditNotebookCellTool {
    fn name(&self) -> &str {
        "edit_notebook_cell"
    }
    fn description(&self) -> &str {
        "Replace a Jupyter notebook cell by index (path relative to workspace root). Prefer this over rewriting the whole .ipynb with write_file."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "cell_index": { "type": "integer" },
                "new_source": { "type": "string" }
            },
            "required": ["path", "cell_index", "new_source"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let index = args["cell_index"].as_u64().unwrap_or(0) as usize;
        let new_source = args["new_source"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let raw = fs::read_to_string(&resolved)?;
        let mut notebook: Value = serde_json::from_str(&raw)?;
        let cells = notebook["cells"]
            .as_array_mut()
            .ok_or_else(|| ToolError::new("invalid notebook"))?;
        let cell = cells
            .get_mut(index)
            .ok_or_else(|| ToolError::new("cell index out of range"))?;
        cell["source"] = json!(new_source.lines().collect::<Vec<_>>());
        fs::write(&resolved, serde_json::to_string_pretty(&notebook)?)?;
        Ok("updated".into())
    }
}

impl Tool for DeleteTextRangeTool {
    fn name(&self) -> &str {
        "delete_text_range"
    }
    fn description(&self) -> &str {
        "Delete text between exact start and end anchors in a file (path relative to workspace root). Prefer this for a contiguous deletion when replace_in_file would need a large old_string; still keep anchors minimal and unique."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "start_anchor": { "type": "string" },
                "end_anchor": { "type": "string" }
            },
            "required": ["path", "start_anchor", "end_anchor"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let start = args["start_anchor"].as_str().unwrap_or("");
        let end = args["end_anchor"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let start_idx = content
            .find(start)
            .ok_or_else(|| ToolError::new("start_anchor not found"))?;
        let end_idx = content[start_idx + start.len()..]
            .find(end)
            .map(|idx| start_idx + start.len() + idx + end.len())
            .ok_or_else(|| ToolError::new("end_anchor not found"))?;
        let updated = format!("{}{}", &content[..start_idx], &content[end_idx..]);
        fs::write(&resolved, updated)?;
        Ok("deleted".into())
    }
}

impl Tool for DeleteGoSymbolTool {
    fn name(&self) -> &str {
        "delete_go_symbol"
    }
    fn description(&self) -> &str {
        "Delete a Go symbol by name using gofmt-compatible heuristics (path relative to workspace root). Prefer this over hand-editing large Go declarations with replace_in_file when the symbol boundary is the unit of change."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "symbol": { "type": "string" }
            },
            "required": ["path", "symbol"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let symbol = args["symbol"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let pattern =
            format!(r"(?ms)^func\s+(\([^)]*\)\s+)?{symbol}\b.*?(?:\n\}}|\n\)\s*\{{.*?\n\}})");
        let re = Regex::new(&pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let updated = re.replace(&content, "").trim().to_string() + "\n";
        fs::write(&resolved, updated)?;
        Ok("deleted".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::tools::context::{AskStore, PathPermissionStore};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    struct NullBus;
    impl EventBus for NullBus {
        fn emit(&self, _event: BusEvent) {}
    }

    fn test_ctx(workspace: std::path::PathBuf) -> (ToolContext, std::path::PathBuf) {
        let db = std::env::temp_dir().join(format!("peek-test-{}.db", uuid::Uuid::new_v4()));
        let ctx = ToolContext {
            workspace_root: workspace,
            request_context: Default::default(),
            session_id: "s".into(),
            assistant_message_id: "a".into(),
            conversation: Arc::new(ConversationManager::new(db.clone())),
            event_bus: Arc::new(NullBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 1,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        (ctx, db)
    }

    #[test]
    fn write_file_blocks_existing_file_without_allow_overwrite() {
        let dir = std::env::temp_dir().join(format!("peek-wf-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("existing.txt"), "hello").unwrap();
        let (ctx, db) = test_ctx(dir.clone());

        let tool = WriteFileTool;
        let args = json!({ "path": "existing.txt", "content": "world" });
        let err_preview = tool.preview(&ctx, &args).unwrap_err();
        assert!(err_preview.to_string().contains("already exists"));
        assert!(err_preview.to_string().contains("allow_overwrite: true"));

        let err_exec = tool.execute(&ctx, args).unwrap_err();
        assert!(err_exec.to_string().contains("already exists"));

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn write_file_overwrites_existing_file_with_allow_overwrite() {
        let dir = std::env::temp_dir().join(format!("peek-wf-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("existing.txt"), "hello").unwrap();
        let (ctx, db) = test_ctx(dir.clone());

        let tool = WriteFileTool;
        let args = json!({ "path": "existing.txt", "content": "world", "allow_overwrite": true });
        assert!(tool.preview(&ctx, &args).unwrap().is_some());
        assert_eq!(tool.execute(&ctx, args).unwrap(), "written");
        assert_eq!(
            fs::read_to_string(dir.join("existing.txt")).unwrap(),
            "world"
        );

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn replace_in_file_fails_with_guided_error_on_mismatch() {
        let dir = std::env::temp_dir().join(format!("peek-rif-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), "line1\nline2\n").unwrap();
        let (ctx, db) = test_ctx(dir.clone());

        let tool = ReplaceInFileTool;
        let args =
            json!({ "path": "file.txt", "old_string": "missing", "new_string": "replacement" });
        let err_preview = tool.preview(&ctx, &args).unwrap_err();
        assert!(err_preview
            .to_string()
            .contains("target `old_string` was not found in `file.txt`"));
        assert!(err_preview
            .to_string()
            .contains("Current content around the target area in `file.txt`"));
        assert!(err_preview.to_string().contains("1 | line1"));

        let err_exec = tool.execute(&ctx, args).unwrap_err();
        assert!(err_exec
            .to_string()
            .contains("target `old_string` was not found in `file.txt`"));
        assert!(err_exec
            .to_string()
            .contains("Current content around the target area in `file.txt`"));

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_file(db);
    }
}
