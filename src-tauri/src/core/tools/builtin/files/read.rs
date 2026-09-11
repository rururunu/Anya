//! File read and search builtin tools.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use glob::Pattern;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::fs_skip;

use super::{office, resolve_read};

/// Recursive listing without a cap walks huge trees (and still emits
/// `node_modules`-adjacent leftovers) — that stalls both the tool and the
/// next model turn. Keep the dump small; `find_files` is the glob path.
const LIST_FOLDER_MAX_ENTRIES: usize = 400;
const LIST_FOLDER_MAX_DEPTH: usize = 6;
const FIND_FILES_MAX_HITS: usize = 200;
/// One call should cover a spec-compliant Anya module (Vue/TS ≤ 400, Rust ≤ 500).
const DEFAULT_READ_LIMIT: usize = 500;
const MAX_READ_LIMIT: usize = 500;
const DEFAULT_AROUND_CONTEXT: usize = 40;
const MAX_AROUND_CONTEXT: usize = 80;
const LARGE_FILE_PREVIEW_LINES: usize = 60;
const MAX_LINE_CHARS: usize = 400;
const MAX_READ_CONTENT_BYTES: usize = 48 * 1024;
const LARGE_FILE_BYTES: u64 = 512 * 1024;
const BINARY_PROBE_BYTES: usize = 8 * 1024;

pub struct ReadFileTool;
pub struct ListFolderTool;
pub struct FindFilesTool;
pub struct ListSymbolsTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a file by path (relative to workspace root). Prefer this over shell cat/Get-Content/type. Unknown location: Grep first. A symbol/call-site hit: around_line. A key source file you must understand or edit: read from the start (default window, hard-capped per call); if truncated, continue with offset. Do not open a file from line 1 to hunt for a symbol. Large dumps (HTML flamegraphs, JSON, logs, CSV) must not be paginated: extract with a short run_shell script that prints a compact aggregate. .docx/.xlsx/.pptx are extracted to plain text automatically."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "around_line": {
                    "type": "integer",
                    "description": "1-based line from Grep / an error. Reads context before and after (default 40). Use for a pinpoint hit; for a key file you need to understand, omit this and read from the start."
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
                "limit": { "type": "integer", "description": "Max lines to return (default 500, hard max 500)" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let mut window = parse_line_window(&args);
        let resolved = resolve_read(ctx, self.name(), path)?;
        if office::is_office_document(&resolved) {
            let extracted = office::extract_office_plain_text(&resolved)?;
            let size = extracted.len() as u64;
            shrink_window_for_large_file(&mut window, size);
            let slice = numbered_slice(&extracted, &window);
            return Ok(format_read_result(
                path,
                size,
                &window,
                &slice.body,
                slice.last_seen,
                slice.truncated,
                looks_like_dump(Path::new(path), size),
            ));
        }
        let size = fs::metadata(&resolved)?.len();
        if probe_binary(&resolved)? {
            return Ok(format!(
                "[file] {path} size={}\nThis file looks binary (NUL in the first {BINARY_PROBE_BYTES} bytes). Do not dump it with read_file or shell cat. Parse it with a format-aware script and print a compact summary.\n",
                format_bytes(size)
            ));
        }
        shrink_window_for_large_file(&mut window, size);
        let file = fs::File::open(&resolved)?;
        let reader = BufReader::new(file);
        let mut body = String::new();
        let mut used_bytes = 0usize;
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
            let row = format_numbered_line(last_seen, &line?);
            if used_bytes + row.len() > MAX_READ_CONTENT_BYTES && !body.is_empty() {
                truncated = true;
                break;
            }
            body.push_str(&row);
            used_bytes += row.len();
        }
        Ok(format_read_result(
            path,
            size,
            &window,
            &body,
            last_seen,
            truncated,
            looks_like_dump(Path::new(path), size),
        ))
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

struct NumberedSlice {
    body: String,
    last_seen: usize,
    truncated: bool,
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
            limit: end
                .saturating_sub(offset)
                .saturating_add(1)
                .min(MAX_READ_LIMIT),
            around_line: None,
        };
    }
    LineWindow {
        offset,
        limit: positive_usize(args, "limit")
            .unwrap_or(DEFAULT_READ_LIMIT)
            .min(MAX_READ_LIMIT),
        around_line: None,
    }
}

fn shrink_window_for_large_file(window: &mut LineWindow, size: u64) {
    if size >= LARGE_FILE_BYTES && window.around_line.is_none() {
        window.limit = window.limit.min(LARGE_FILE_PREVIEW_LINES);
    }
}

fn probe_binary(path: &Path) -> Result<bool, ToolError> {
    let mut file = fs::File::open(path)?;
    let mut buf = vec![0u8; BINARY_PROBE_BYTES];
    let n = file.read(&mut buf)?;
    Ok(buf[..n].contains(&0))
}

fn looks_like_dump(path: &Path, size: u64) -> bool {
    if size >= 2 * 1024 * 1024 {
        return true;
    }
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some(
            "html"
                | "htm"
                | "svg"
                | "json"
                | "jsonl"
                | "ndjson"
                | "log"
                | "csv"
                | "xml"
                | "prof"
                | "folded"
        )
    ) && size >= LARGE_FILE_BYTES
}

fn format_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if n as f64 >= MB {
        format!("{:.1}MB", n as f64 / MB)
    } else if n as f64 >= KB {
        format!("{:.1}KB", n as f64 / KB)
    } else {
        format!("{n}B")
    }
}

fn clamp_line(line: &str) -> String {
    let count = line.chars().count();
    if count <= MAX_LINE_CHARS {
        return line.to_string();
    }
    let head: String = line.chars().take(MAX_LINE_CHARS).collect();
    format!("{head}…[{} chars truncated]", count - MAX_LINE_CHARS)
}

fn format_numbered_line(line_no: usize, line: &str) -> String {
    format!("{line_no:>6}|{}\n", clamp_line(line))
}

fn window_footer(
    window: &LineWindow,
    last_seen: usize,
    truncated: bool,
    large_dump: bool,
) -> String {
    if let Some(around) = window.around_line {
        if around > last_seen && !truncated {
            return format!("… around_line={around} is past end of file ({last_seen} lines)\n");
        }
    }
    if truncated && large_dump {
        return concat!(
            "… truncated. This file is too large to read in full. ",
            "Do not paginate with offset. Write a short run_shell script that extracts ",
            "the needed structure and prints a compact aggregate (counts, top-N, totals), ",
            "or call Grep for a specific pattern.\n"
        )
        .to_string();
    }
    if truncated {
        let next = window.offset.saturating_add(window.limit);
        return format!("… more lines follow; pass offset={next} to continue\n");
    }
    String::new()
}

fn format_read_result(
    path: &str,
    size: u64,
    window: &LineWindow,
    body: &str,
    last_seen: usize,
    truncated: bool,
    large_dump: bool,
) -> String {
    let mut out = format!("[file] {path} size={}\n", format_bytes(size));
    out.push_str(body);
    out.push_str(&window_footer(window, last_seen, truncated, large_dump));
    out
}

fn numbered_slice(text: &str, window: &LineWindow) -> NumberedSlice {
    let mut body = String::new();
    let mut used_bytes = 0usize;
    let mut last_seen = 0usize;
    let mut truncated = false;
    for (idx, line) in text.lines().enumerate() {
        last_seen = idx + 1;
        if last_seen < window.offset {
            continue;
        }
        if last_seen >= window.offset + window.limit {
            truncated = true;
            break;
        }
        let row = format_numbered_line(last_seen, line);
        if used_bytes + row.len() > MAX_READ_CONTENT_BYTES && !body.is_empty() {
            truncated = true;
            break;
        }
        body.push_str(&row);
        used_bytes += row.len();
    }
    NumberedSlice {
        body,
        last_seen,
        truncated,
    }
}

impl Tool for ListFolderTool {
    fn name(&self) -> &str {
        "list_folder"
    }
    fn description(&self) -> &str {
        "List files and directories under a path (relative to workspace root). Use for structure/orientation. Prefer find_files for glob patterns and Grep for content; do not recursively dump large trees when a narrower search works."
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
        "Find files by glob pattern (relative to workspace root). Prefer this over shell find/rg --files/Get-ChildItem for locating paths. For content inside files use Grep; for a single known path use read_file."
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

impl Tool for ListSymbolsTool {
    fn name(&self) -> &str {
        "list_symbols"
    }
    fn description(&self) -> &str {
        "Lightweight symbol outline for a source file (relative to workspace root). Use after you already know the path and need structure (functions/types) quickly; do not use this to discover which file to open — call Grep first. Use lsp for go-to-definition/hover when available."
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

        let huge = parse_line_window(&json!({ "limit": 999_999 }));
        assert_eq!(huge.limit, MAX_READ_LIMIT);
        let huge_end = parse_line_window(&json!({ "start_line": 1, "end_line": 50_000 }));
        assert_eq!(huge_end.limit, MAX_READ_LIMIT);
    }

    #[test]
    fn read_file_covers_a_medium_source_module_in_one_call() {
        let root = std::env::temp_dir().join(format!("peek-read-mod-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let body: String = (1..=360).map(|n| format!("line-{n}\n")).collect();
        std::fs::write(root.join("win.rs"), &body).unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = ReadFileTool
            .execute(&ctx, json!({ "path": "win.rs" }))
            .unwrap();
        assert!(out.contains("line-1"), "{out}");
        assert!(out.contains("line-360"), "{out}");
        assert!(!out.contains("more lines follow"), "{out}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
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

    #[test]
    fn read_file_reports_size_and_rejects_binary() {
        let root = std::env::temp_dir().join(format!("peek-read-bin-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("notes.txt"), "hello\nworld\n").unwrap();
        std::fs::write(root.join("blob.bin"), [0u8, 1, 2, 3]).unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let text = ReadFileTool
            .execute(&ctx, json!({ "path": "notes.txt" }))
            .unwrap();
        assert!(text.contains("[file] notes.txt size="), "{text}");
        assert!(text.contains("hello"), "{text}");

        let binary = ReadFileTool
            .execute(&ctx, json!({ "path": "blob.bin" }))
            .unwrap();
        assert!(binary.contains("looks binary"), "{binary}");
        assert!(!binary.contains('\0'), "{binary}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn read_file_large_dump_asks_for_script_not_pagination() {
        let root = std::env::temp_dir().join(format!("peek-read-dump-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let chunk = "<div class=\"frame\">fn;main;work 12</div>\n".repeat(40_000);
        std::fs::write(root.join("cpu.html"), chunk.as_bytes()).unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = ReadFileTool
            .execute(&ctx, json!({ "path": "cpu.html", "limit": 999_999 }))
            .unwrap();
        assert!(out.contains("[file] cpu.html size="), "{out}");
        assert!(out.contains("too large to read in full"), "{out}");
        assert!(out.contains("run_shell script"), "{out}");
        assert!(!out.contains("pass offset="), "{out}");
        let data_lines = out.lines().filter(|line| line.contains('|')).count();
        assert!(data_lines <= LARGE_FILE_PREVIEW_LINES + 2, "{data_lines}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn read_file_clamps_very_long_lines() {
        let root = std::env::temp_dir().join(format!("peek-read-long-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let long = "x".repeat(2_000);
        std::fs::write(root.join("one.json"), format!("{long}\n")).unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = ReadFileTool
            .execute(&ctx, json!({ "path": "one.json" }))
            .unwrap();
        assert!(out.contains("chars truncated"), "{out}");
        assert!(out.len() < 2_000, "{}", out.len());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }
}
