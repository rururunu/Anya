//! File read and search builtin tools.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::Command;

use glob::glob;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::runtime::terminal::prepare_command;

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;

use super::{office, resolve_read, run_command_cancellable, should_skip};

pub struct ReadFileTool;
pub struct ListFolderTool;
pub struct FindFilesTool;
pub struct SearchFilesTool;
pub struct ListSymbolsTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a file by path (relative to workspace root). Prefer this over shell cat/Get-Content/type. Text files return numbered lines. .docx/.xlsx/.pptx are extracted to plain text automatically — do not ask the user to open Word, and do not use Word COM just to read an on-disk Office file. Use offset/limit for large files instead of loading everything. For unknown paths, find_files or search_files first; for directory structure use list_folder."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "offset": { "type": "integer" },
                "limit": { "type": "integer" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let offset = args["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(200) as usize;
        let resolved = resolve_read(ctx, self.name(), path)?;
        if office::is_office_document(&resolved) {
            let extracted = office::extract_office_plain_text(&resolved)?;
            return Ok(numbered_slice(&extracted, offset, limit));
        }
        let file = fs::File::open(&resolved)?;
        let reader = BufReader::new(file);
        let mut out = String::new();
        for (idx, line) in reader.lines().enumerate() {
            ctx.ensure_not_cancelled()?;
            let line_no = idx + 1;
            if line_no < offset {
                continue;
            }
            if line_no >= offset + limit {
                break;
            }
            out.push_str(&format!("{line_no:>6}|{}\n", line?));
        }
        Ok(out)
    }
}

fn numbered_slice(text: &str, offset: usize, limit: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let mut out = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        if line_no < offset {
            continue;
        }
        if line_no >= offset + limit {
            break;
        }
        out.push_str(&format!("{line_no:>6}|{line}\n"));
    }
    let next = offset.saturating_add(limit);
    if next <= total {
        out.push_str(&format!(
            "… {total} lines total; pass offset={next} to continue\n"
        ));
    }
    out
}

impl Tool for ListFolderTool {
    fn name(&self) -> &str {
        "list_folder"
    }
    fn description(&self) -> &str {
        "List files and directories under a path (relative to workspace root). Use for structure/orientation. Prefer find_files for glob patterns and search_files for content; do not recursively dump large trees when a narrower search works."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path relative to workspace root" },
                "recursive": { "type": "boolean" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or(".");
        let recursive = args["recursive"].as_bool().unwrap_or(false);
        let resolved = resolve_read(ctx, self.name(), path)?;
        if !recursive {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&resolved)? {
                ctx.ensure_not_cancelled()?;
                let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let kind = if entry.file_type()?.is_dir() {
                    "dir"
                } else {
                    "file"
                };
                entries.push(format!("[{kind}] {name}"));
            }
            entries.sort();
            return Ok(entries.join("\n"));
        }
        let mut lines = Vec::new();
        for entry in WalkDir::new(&resolved)
            .into_iter()
            .filter_entry(|e| !should_skip(e.path()))
        {
            ctx.ensure_not_cancelled()?;
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            let rel = entry
                .path()
                .strip_prefix(&resolved)
                .unwrap_or(entry.path())
                .display();
            let kind = if entry.file_type().is_dir() {
                "dir"
            } else {
                "file"
            };
            lines.push(format!("[{kind}] {rel}"));
        }
        Ok(lines.join("\n"))
    }
}

impl Tool for FindFilesTool {
    fn name(&self) -> &str {
        "find_files"
    }
    fn description(&self) -> &str {
        "Find files by glob pattern (relative to workspace root). Prefer this over shell find/rg --files/Get-ChildItem for locating paths. For content inside files use search_files; for a single known path use read_file."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string", "description": "Base directory relative to workspace root (default: workspace root)" }
            },
            "required": ["pattern"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let base = args["path"].as_str().unwrap_or(".");
        let resolved = resolve_read(ctx, self.name(), base)?;
        let full_pattern = resolved.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        let mut hits = Vec::new();
        for entry in glob(&pattern_str).map_err(|e| ToolError::new(e.to_string()))? {
            ctx.ensure_not_cancelled()?;
            let path = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if should_skip(&path) {
                continue;
            }
            hits.push(path.display().to_string());
            if hits.len() >= 200 {
                break;
            }
        }
        Ok(hits.join("\n"))
    }
}

impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }
    fn description(&self) -> &str {
        "Preferred content-search tool: regex search in files (ripgrep when available, internal fallback otherwise). Prefer this over shell rg/grep/findstr. Path is relative to workspace root. Exclude generated and dependency dirs unless they are in scope. For path-only discovery use find_files; for a known file use read_file."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string", "description": "Search directory relative to workspace root (default: workspace root)" }
            },
            "required": ["pattern"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let base = args["path"].as_str().unwrap_or(".");
        let resolved = resolve_read(ctx, self.name(), base)?;
        let mut rg = Command::new("rg");
        rg.args([
            "--no-heading",
            "--line-number",
            pattern,
            resolved.to_str().unwrap_or(""),
        ]);
        prepare_command(&mut rg);
        if let Some(output) = run_command_cancellable(ctx, &mut rg)? {
            if output.status.success() || !output.stdout.is_empty() {
                let text = crate::runtime::encoding::decode_process_bytes(&output.stdout);
                let lines: Vec<_> = text.lines().take(200).collect();
                return Ok(lines.join("\n"));
            }
        }
        let re = Regex::new(pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let mut hits = Vec::new();
        for entry in WalkDir::new(&resolved)
            .into_iter()
            .filter_entry(|e| !should_skip(e.path()))
        {
            ctx.ensure_not_cancelled()?;
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(content) = fs::read_to_string(entry.path()) else {
                continue;
            };
            for (idx, line) in content.lines().enumerate() {
                ctx.ensure_not_cancelled()?;
                if re.is_match(line) {
                    hits.push(format!("{}:{}:{}", entry.path().display(), idx + 1, line));
                    if hits.len() >= 200 {
                        return Ok(hits.join("\n"));
                    }
                }
            }
        }
        Ok(hits.join("\n"))
    }
}

impl Tool for ListSymbolsTool {
    fn name(&self) -> &str {
        "list_symbols"
    }
    fn description(&self) -> &str {
        "Lightweight symbol outline for a source file (relative to workspace root). Use before a deep read when you need structure (functions/types) quickly; use lsp for precise go-to-definition/hover/diagnostics when available."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "File path relative to workspace root" } },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let resolved = resolve_read(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let re = Regex::new(
            r"(?m)^\s*(pub\s+)?(fn|struct|enum|trait|impl|class|def|func)\s+([A-Za-z0-9_]+)",
        )
        .map_err(|e| ToolError::new(e.to_string()))?;
        let mut out = Vec::new();
        for cap in re.captures_iter(&content) {
            ctx.ensure_not_cancelled()?;
            out.push(format!(
                "{} {}",
                cap.get(2).map(|m| m.as_str()).unwrap_or(""),
                cap.get(3).map(|m| m.as_str()).unwrap_or("")
            ));
        }
        Ok(out.join("\n"))
    }
}
