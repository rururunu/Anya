//! File read and search builtin tools.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::Command;

use glob::Pattern;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::runtime::terminal::prepare_command;

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::fs_skip;

use super::{office, resolve_read, run_command_cancellable};

/// Recursive listing without a cap walks huge trees (and still emits
/// `node_modules`-adjacent leftovers) — that stalls both the tool and the
/// next model turn. Keep the dump small; `find_files` is the glob path.
const LIST_FOLDER_MAX_ENTRIES: usize = 400;
const LIST_FOLDER_MAX_DEPTH: usize = 6;
const FIND_FILES_MAX_HITS: usize = 200;
const SEARCH_FILES_MAX_HITS: usize = 200;
const SEARCH_MAX_FILE_BYTES: u64 = 512 * 1024;
const DEFAULT_READ_LIMIT: usize = 200;
const DEFAULT_AROUND_CONTEXT: usize = 40;
const MAX_AROUND_CONTEXT: usize = 80;

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
        "Read a file by path (relative to workspace root). Prefer this over shell cat/Get-Content/type. Text files return numbered lines. When you already have a line number (search_files hit, list_symbols, compiler error), pass around_line to read that neighborhood — never omit it and start at line 1. Paginate with start_line/offset and limit or end_line. Default without a range is the first 200 lines. .docx/.xlsx/.pptx are extracted to plain text automatically — do not ask the user to open Word, and do not use Word COM just to read an on-disk Office file. For unknown paths, find_files or search_files first; for directory structure use list_folder."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "around_line": {
                    "type": "integer",
                    "description": "1-based line from search_files / an error. Reads context lines before and after (default 40). Prefer this over reading from line 1."
                },
                "context": {
                    "type": "integer",
                    "description": "Lines before and after around_line (default 40, max 80)"
                },
                "start_line": {
                    "type": "integer",
                    "description": "1-based first line (alias of offset)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "1-based last line inclusive; used with start_line/offset"
                },
                "offset": { "type": "integer", "description": "1-based first line (default 1)" },
                "limit": { "type": "integer", "description": "Max lines to return (default 200)" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let window = parse_line_window(&args);
        let resolved = resolve_read(ctx, self.name(), path)?;
        if office::is_office_document(&resolved) {
            let extracted = office::extract_office_plain_text(&resolved)?;
            return Ok(numbered_slice(&extracted, &window));
        }
        let file = fs::File::open(&resolved)?;
        let reader = BufReader::new(file);
        let mut out = String::new();
        let mut last_seen = 0usize;
        let mut truncated = false;
        for (idx, line) in reader.lines().enumerate() {
            ctx.ensure_not_cancelled()?;
            last_seen = idx + 1;
            if last_seen < window.offset {
                continue;
            }
            if last_seen >= window.offset + window.limit {
                truncated = true;
                break;
            }
            out.push_str(&format!("{last_seen:>6}|{}\n", line?));
        }
        out.push_str(&window_footer(&window, last_seen, truncated));
        Ok(out)
    }
}

/// `*` must not cross `/`, so `*.rs` stays non-recursive (old `glob()` walker).
/// `**/*.rs` still matches a file at the walk root (`top.rs`).
fn glob_matches(matcher: &Pattern, pattern: &str, rel: &str) -> bool {
    let options = glob::MatchOptions {
        case_sensitive: !cfg!(windows),
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    if matcher.matches_with(rel, options) {
        return true;
    }
    if rel.contains('/') || !pattern.contains("**") {
        return false;
    }
    matcher.matches_with(&format!("*/{rel}"), options)
}

struct LineWindow {
    offset: usize,
    limit: usize,
    around_line: Option<usize>,
}

fn positive_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key)?.as_u64().map(|n| n.max(1) as usize)
}

fn parse_line_window(args: &Value) -> LineWindow {
    if let Some(around) = positive_usize(args, "around_line") {
        let context = positive_usize(args, "context")
            .unwrap_or(DEFAULT_AROUND_CONTEXT)
            .min(MAX_AROUND_CONTEXT);
        let start = around.saturating_sub(context).max(1);
        let end = around.saturating_add(context);
        return LineWindow {
            offset: start,
            limit: end.saturating_sub(start).saturating_add(1),
            around_line: Some(around),
        };
    }
    let offset = positive_usize(args, "start_line")
        .or_else(|| positive_usize(args, "offset"))
        .unwrap_or(1);
    if let Some(end) = positive_usize(args, "end_line") {
        let end = end.max(offset);
        return LineWindow {
            offset,
            limit: end.saturating_sub(offset).saturating_add(1),
            around_line: None,
        };
    }
    LineWindow {
        offset,
        limit: positive_usize(args, "limit").unwrap_or(DEFAULT_READ_LIMIT),
        around_line: None,
    }
}

fn window_footer(window: &LineWindow, last_seen: usize, truncated: bool) -> String {
    if let Some(around) = window.around_line {
        if around > last_seen && !truncated {
            return format!("… around_line={around} is past end of file ({last_seen} lines)\n");
        }
    }
    if truncated {
        let next = window.offset.saturating_add(window.limit);
        return format!("… more lines follow; pass offset={next} to continue\n");
    }
    String::new()
}

fn numbered_slice(text: &str, window: &LineWindow) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let mut out = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        if line_no < window.offset {
            continue;
        }
        if line_no >= window.offset + window.limit {
            break;
        }
        out.push_str(&format!("{line_no:>6}|{line}\n"));
    }
    let truncated = window.offset.saturating_add(window.limit) <= total;
    out.push_str(&window_footer(window, total, truncated));
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
                if entries.len() >= LIST_FOLDER_MAX_ENTRIES {
                    break;
                }
            }
            entries.sort();
            let mut out = entries.join("\n");
            if entries.len() >= LIST_FOLDER_MAX_ENTRIES {
                out.push_str(&format!(
                    "\n… truncated after {LIST_FOLDER_MAX_ENTRIES} entries. Use a narrower path or find_files."
                ));
            }
            return Ok(out);
        }
        let mut lines = Vec::new();
        let mut truncated = false;
        for entry in WalkDir::new(&resolved)
            .max_depth(LIST_FOLDER_MAX_DEPTH)
            .into_iter()
            .filter_entry(|e| !fs_skip::should_skip_walk_entry(e))
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
            if lines.len() >= LIST_FOLDER_MAX_ENTRIES {
                truncated = true;
                break;
            }
        }
        let mut out = lines.join("\n");
        if truncated {
            out.push_str(&format!(
                "\n… truncated after {LIST_FOLDER_MAX_ENTRIES} entries (max depth {LIST_FOLDER_MAX_DEPTH}). Use a narrower path or find_files."
            ));
        }
        Ok(out)
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
        let matcher = Pattern::new(pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let mut hits = Vec::new();
        let mut truncated = false;
        for (index, entry) in WalkDir::new(&resolved)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !fs_skip::should_skip_walk_entry(e))
            .enumerate()
        {
            if index % 32 == 0 {
                ctx.ensure_not_cancelled()?;
            }
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&resolved)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if !glob_matches(&matcher, pattern, &rel) {
                continue;
            }
            hits.push(entry.path().display().to_string());
            if hits.len() >= FIND_FILES_MAX_HITS {
                truncated = true;
                break;
            }
        }
        let mut out = hits.join("\n");
        if truncated {
            out.push_str(&format!(
                "\n… truncated after {FIND_FILES_MAX_HITS} entries. Use a narrower pattern or path."
            ));
        }
        Ok(out)
    }
}

impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }
    fn description(&self) -> &str {
        "Preferred content-search tool: regex search in files (ripgrep when available, internal fallback otherwise). Prefer this over shell rg/grep/findstr. Hits are path:line:text — follow a hit with read_file around_line=<line>, do not read the file from line 1. Path is relative to workspace root. Exclude generated and dependency dirs unless they are in scope. For path-only discovery use find_files; for a known file use read_file."
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
        rg.args(["--no-heading", "--line-number", "--max-filesize", "512K"]);
        for glob in fs_skip::rg_exclude_globs() {
            rg.arg("-g").arg(glob);
        }
        rg.arg(pattern);
        rg.arg(resolved.to_str().unwrap_or(""));
        prepare_command(&mut rg);
        if let Some(output) = run_command_cancellable(ctx, &mut rg)? {
            if output.status.success() || !output.stdout.is_empty() {
                let text = crate::runtime::encoding::decode_process_bytes(&output.stdout);
                let lines: Vec<_> = text.lines().take(SEARCH_FILES_MAX_HITS).collect();
                return Ok(lines.join("\n"));
            }
        }
        let re = Regex::new(pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let mut hits = Vec::new();
        for (index, entry) in WalkDir::new(&resolved)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !fs_skip::should_skip_walk_entry(e))
            .enumerate()
        {
            if index % 32 == 0 {
                ctx.ensure_not_cancelled()?;
            }
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.len() > SEARCH_MAX_FILE_BYTES {
                continue;
            }
            let Ok(content) = fs::read_to_string(entry.path()) else {
                continue;
            };
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    hits.push(format!("{}:{}:{}", entry.path().display(), idx + 1, line));
                    if hits.len() >= SEARCH_FILES_MAX_HITS {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
    use std::sync::{atomic::AtomicBool, Arc, Mutex};

    struct NullBus;
    impl EventBus for NullBus {
        fn emit(&self, _event: BusEvent) {}
    }

    fn make_ctx(workspace: std::path::PathBuf) -> (ToolContext, std::path::PathBuf) {
        let db = std::env::temp_dir().join(format!("peek-list-{}.db", uuid::Uuid::new_v4()));
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
    fn glob_matches_star_is_not_recursive_but_globstar_is() {
        let star = Pattern::new("*.rs").unwrap();
        assert!(glob_matches(&star, "*.rs", "top.rs"));
        assert!(!glob_matches(&star, "*.rs", "src/lib.rs"));

        let globstar = Pattern::new("**/*.rs").unwrap();
        assert!(glob_matches(&globstar, "**/*.rs", "src/lib.rs"));
        assert!(glob_matches(&globstar, "**/*.rs", "top.rs"));

        let nested = Pattern::new("src/*.rs").unwrap();
        assert!(glob_matches(&nested, "src/*.rs", "src/lib.rs"));
        assert!(!glob_matches(&nested, "src/*.rs", "top.rs"));
    }

    #[test]
    fn recursive_list_caps_depth_and_count() {
        let root = std::env::temp_dir().join(format!("peek-list-ws-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut deep = root.clone();
        for level in 0..8 {
            deep.push(format!("d{level}"));
            std::fs::create_dir_all(&deep).unwrap();
        }
        std::fs::write(deep.join("hidden.txt"), "x").unwrap();
        for index in 0..(LIST_FOLDER_MAX_ENTRIES + 20) {
            std::fs::write(root.join(format!("f{index}.txt")), "x").unwrap();
        }

        let (ctx, db) = make_ctx(root.clone());
        let out = ListFolderTool
            .execute(&ctx, json!({ "path": ".", "recursive": true }))
            .unwrap();
        assert!(out.contains("truncated"));
        assert!(!out.contains("hidden.txt"));
        let entry_lines = out.lines().filter(|line| line.starts_with('[')).count();
        assert!(entry_lines <= LIST_FOLDER_MAX_ENTRIES);

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn recursive_list_skips_target_dir() {
        let root = std::env::temp_dir().join(format!("peek-list-skip-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target").join("debug")).unwrap();
        std::fs::write(root.join("src").join("lib.rs"), "x").unwrap();
        std::fs::write(root.join("target").join("debug").join("out.rs"), "x").unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = ListFolderTool
            .execute(&ctx, json!({ "path": ".", "recursive": true }))
            .unwrap();
        assert!(out.contains("lib.rs"));
        assert!(!out.contains("out.rs"));
        assert!(!out.to_lowercase().contains("target"));

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn find_files_skips_target_and_star_is_not_recursive() {
        let root = std::env::temp_dir().join(format!("peek-find-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target").join("debug")).unwrap();
        std::fs::write(root.join("top.rs"), "x").unwrap();
        std::fs::write(root.join("src").join("lib.rs"), "x").unwrap();
        for index in 0..80 {
            std::fs::write(
                root.join("target")
                    .join("debug")
                    .join(format!("gen{index}.rs")),
                "x",
            )
            .unwrap();
        }

        let (ctx, db) = make_ctx(root.clone());
        let nested = FindFilesTool
            .execute(&ctx, json!({ "pattern": "**/*.rs", "path": "." }))
            .unwrap();
        assert!(nested.contains("lib.rs"), "{nested}");
        assert!(nested.contains("top.rs"), "{nested}");
        assert!(!nested.contains("gen0.rs"), "{nested}");
        assert!(!nested.contains("target"), "{nested}");

        let star = FindFilesTool
            .execute(&ctx, json!({ "pattern": "*.rs", "path": "." }))
            .unwrap();
        assert!(star.contains("top.rs"), "{star}");
        assert!(!star.contains("lib.rs"), "{star}");

        let src_only = FindFilesTool
            .execute(&ctx, json!({ "pattern": "src/*.rs", "path": "." }))
            .unwrap();
        assert!(src_only.contains("lib.rs"), "{src_only}");
        assert!(!src_only.contains("top.rs"), "{src_only}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn parse_line_window_prefers_around_line() {
        let around = parse_line_window(&json!({ "around_line": 100, "context": 5 }));
        assert_eq!(around.offset, 95);
        assert_eq!(around.limit, 11);
        assert_eq!(around.around_line, Some(100));

        let start = parse_line_window(&json!({ "start_line": 10, "end_line": 12 }));
        assert_eq!(start.offset, 10);
        assert_eq!(start.limit, 3);

        let offset = parse_line_window(&json!({ "offset": 3, "limit": 4 }));
        assert_eq!(offset.offset, 3);
        assert_eq!(offset.limit, 4);

        let default = parse_line_window(&json!({ "path": "x" }));
        assert_eq!(default.offset, 1);
        assert_eq!(default.limit, DEFAULT_READ_LIMIT);
    }

    #[test]
    fn read_file_around_line_skips_file_head() {
        let root = std::env::temp_dir().join(format!("peek-read-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let body: String = (1..=80).map(|n| format!("line-{n}\n")).collect();
        std::fs::write(root.join("notes.txt"), &body).unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = ReadFileTool
            .execute(
                &ctx,
                json!({ "path": "notes.txt", "around_line": 50, "context": 2 }),
            )
            .unwrap();
        assert!(out.contains("line-50"), "{out}");
        assert!(out.contains("    48|"), "{out}");
        assert!(out.contains("    52|"), "{out}");
        assert!(
            !out.contains("line-1\n") && !out.contains("     1|"),
            "{out}"
        );

        let ranged = ReadFileTool
            .execute(
                &ctx,
                json!({ "path": "notes.txt", "start_line": 10, "end_line": 11 }),
            )
            .unwrap();
        assert!(ranged.contains("    10|line-10"), "{ranged}");
        assert!(ranged.contains("    11|line-11"), "{ranged}");
        assert!(!ranged.contains("    12|"), "{ranged}");

        let past = ReadFileTool
            .execute(&ctx, json!({ "path": "notes.txt", "around_line": 500 }))
            .unwrap();
        assert!(past.contains("past end of file"), "{past}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }
}
