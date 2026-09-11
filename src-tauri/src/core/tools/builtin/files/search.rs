//! Content search (`Grep`, internal name `search_files`): bundled/PATH ripgrep, then a WalkDir fallback.

use std::fs;
use std::process::Command;

use regex::RegexBuilder;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::fs_skip;
use crate::core::tools::rg::{display_hit_path, rg_command_candidates};
use crate::runtime::terminal::prepare_command;

use super::{resolve_read, run_command_cancellable};

const SEARCH_FILES_MAX_HITS: usize = 200;
const SEARCH_MAX_FILE_BYTES: u64 = 512 * 1024;
const SEARCH_HIT_LINE_CHARS: usize = 240;
const SEARCH_MAX_COUNT_PER_FILE: usize = 15;
const MAX_CONTEXT_LINES: u64 = 40;

pub struct SearchFilesTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Content,
    FilesWithMatches,
    Count,
}

#[derive(Debug)]
struct SearchArgs {
    pattern: String,
    case_insensitive: bool,
    before: Option<u64>,
    after: Option<u64>,
    output_mode: OutputMode,
    head_limit: Option<usize>,
}

fn parse_search_args(args: &Value) -> Result<SearchArgs, ToolError> {
    let pattern = args["pattern"]
        .as_str()
        .or_else(|| args["query"].as_str())
        .unwrap_or("")
        .to_string();
    if pattern.is_empty() {
        return Err(ToolError::new("pattern (or query) is required"));
    }
    let case_insensitive = args["-i"].as_bool().unwrap_or(false);
    let context = args["-C"].as_u64().map(|n| n.min(MAX_CONTEXT_LINES));
    let before = args["-B"]
        .as_u64()
        .map(|n| n.min(MAX_CONTEXT_LINES))
        .or(context);
    let after = args["-A"]
        .as_u64()
        .map(|n| n.min(MAX_CONTEXT_LINES))
        .or(context);
    let output_mode = match args["output_mode"].as_str().unwrap_or("content") {
        "files_with_matches" => OutputMode::FilesWithMatches,
        "count" => OutputMode::Count,
        _ => OutputMode::Content,
    };
    let head_limit = args["head_limit"].as_u64().map(|n| (n as usize).max(1));
    Ok(SearchArgs {
        pattern,
        case_insensitive,
        before,
        after,
        output_mode,
        head_limit,
    })
}

impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }
    fn description(&self) -> &str {
        "Locate code by regex (bundled ripgrep, PATH rg, then an internal walk). Prefer this over shell rg/grep/findstr and over opening files from line 1. Pattern can also be passed as `query`. Default output is `path:line:text` (context lines use `path-line-text`); `output_mode` can switch to `files_with_matches` (unique paths only) or `count` (matches per file). `-i` for case-insensitive, `-A`/`-B`/`-C` for lines of context after/before/both, `head_limit` to cap results. Optional glob narrows by filename (e.g. *.rs). Skips generated/dependency dirs and files over 512K. For path-only discovery use find_files; for a known path use read_file."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Regex to match inside files" },
                "query": { "type": "string", "description": "Alias for `pattern`" },
                "path": { "type": "string", "description": "Search directory relative to workspace root (default: workspace root)" },
                "glob": { "type": "string", "description": "Optional filename glob, e.g. *.rs or *.{ts,vue}" },
                "-i": { "type": "boolean", "description": "Case insensitive search" },
                "-A": { "type": "number", "description": "Lines of context to show after each match" },
                "-B": { "type": "number", "description": "Lines of context to show before each match" },
                "-C": { "type": "number", "description": "Lines of context before and after each match (overridden by -A/-B if both given)" },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "\"content\" (default) shows matching lines; \"files_with_matches\" lists unique file paths; \"count\" shows match counts per file"
                },
                "head_limit": { "type": "number", "description": "Cap the number of results (lines for content, files for files_with_matches/count)" }
            },
            "required": ["pattern"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    /// Exposed to the model as `Grep` — matches the shape most coding-model
    /// priors already expect, so it gets reached for and used correctly
    /// without extra prompting. `search_files`/`grep`/`rg` remain aliases
    /// (see `normalize_tool_name`) for any caller still using the old name.
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "Grep",
                "description": self.description(),
                "parameters": self.parameters_schema(),
            }
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let search = parse_search_args(&args)?;
        let base = args["path"].as_str().unwrap_or(".");
        let glob = args["glob"].as_str().filter(|value| !value.is_empty());
        let resolved = resolve_read(ctx, self.name(), base)?;

        for binary in rg_command_candidates() {
            ctx.ensure_not_cancelled()?;
            let mut rg = Command::new(&binary);
            rg.current_dir(&resolved);
            rg.args([
                "--no-heading",
                "--line-number",
                "--max-filesize",
                "512K",
                "--max-columns",
                "240",
                "--max-columns-preview",
            ]);
            if search.case_insensitive {
                rg.arg("-i");
            }
            match search.output_mode {
                OutputMode::FilesWithMatches => {
                    rg.arg("-l");
                }
                OutputMode::Count => {
                    rg.arg("--count");
                }
                OutputMode::Content => {
                    rg.args(["--max-count", &SEARCH_MAX_COUNT_PER_FILE.to_string()]);
                    if let Some(before) = search.before {
                        rg.arg("-B").arg(before.to_string());
                    }
                    if let Some(after) = search.after {
                        rg.arg("-A").arg(after.to_string());
                    }
                }
            }
            for exclude in fs_skip::rg_exclude_globs() {
                rg.arg("-g").arg(exclude);
            }
            if let Some(glob) = glob {
                rg.arg("-g").arg(glob);
            }
            rg.arg("--").arg(&search.pattern).arg(".");
            prepare_command(&mut rg);
            let Some(output) = run_command_cancellable(ctx, &mut rg)? else {
                continue;
            };
            let stdout = crate::runtime::encoding::decode_process_bytes(&output.stdout);
            let code = output.status.code();
            if code == Some(0) || code == Some(1) || !stdout.trim().is_empty() {
                return Ok(format_hits(&stdout, search.output_mode, search.head_limit));
            }
            let stderr = crate::runtime::encoding::decode_process_bytes(&output.stderr);
            if !stderr.trim().is_empty() {
                return Err(ToolError::new(stderr.trim().to_string()));
            }
        }

        walk_search(ctx, &resolved, &search, glob)
    }
}

fn cap_for(head_limit: Option<usize>) -> usize {
    head_limit
        .unwrap_or(SEARCH_FILES_MAX_HITS)
        .min(SEARCH_FILES_MAX_HITS)
        .max(1)
}

fn format_hits(text: &str, output_mode: OutputMode, head_limit: Option<usize>) -> String {
    let cap = cap_for(head_limit);
    let all_lines: Vec<String> = text
        .lines()
        .map(normalize_hit_line)
        .filter(|l| !l.is_empty())
        .collect();
    let lines: Vec<&String> = all_lines.iter().take(cap).collect();
    if lines.is_empty() {
        return match output_mode {
            OutputMode::Content => {
                "No matches. Try a simpler pattern, a narrower path, or find_files by name."
                    .to_string()
            }
            OutputMode::FilesWithMatches => "No files matched.".to_string(),
            OutputMode::Count => "No matches.".to_string(),
        };
    }
    let mut out = lines.into_iter().cloned().collect::<Vec<_>>().join("\n");
    if all_lines.len() > cap {
        let noun = match output_mode {
            OutputMode::Content => "hits",
            OutputMode::FilesWithMatches | OutputMode::Count => "files",
        };
        out.push_str(&format!(
            "\n… truncated after {cap} {noun}. Narrow the pattern, path, glob, or head_limit."
        ));
    }
    out
}

fn normalize_hit_line(line: &str) -> String {
    let trimmed = line.trim_start_matches(".\\").trim_start_matches("./");
    truncate_hit_line(&trimmed.replace('\\', "/"))
}

fn truncate_hit_line(line: &str) -> String {
    let chars = line.chars().count();
    if chars <= SEARCH_HIT_LINE_CHARS {
        return line.to_string();
    }
    let head: String = line.chars().take(SEARCH_HIT_LINE_CHARS).collect();
    format!("{head}…")
}

fn glob_matches_name(glob: &str, rel: &str) -> bool {
    let Ok(matcher) = glob::Pattern::new(glob) else {
        return true;
    };
    let options = glob::MatchOptions {
        case_sensitive: !cfg!(windows),
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    matcher.matches_with(rel, options)
        || std::path::Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matcher.matches_with(name, options))
}

/// A matched (1-based) line number inside one file.
struct FileMatch {
    rel: String,
    line_no: usize,
}

fn walk_search(
    ctx: &ToolContext,
    root: &std::path::Path,
    search: &SearchArgs,
    glob: Option<&str>,
) -> Result<String, ToolError> {
    let re = RegexBuilder::new(&search.pattern)
        .case_insensitive(search.case_insensitive)
        .build()
        .map_err(|error| ToolError::new(error.to_string()))?;
    let cap = cap_for(search.head_limit);

    let mut content_hits = Vec::new();
    let mut files_with_matches = Vec::new();
    let mut counts: Vec<(String, usize)> = Vec::new();
    'outer: for (index, entry) in WalkDir::new(root)
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
        let rel = display_hit_path(root, entry.path());
        if let Some(glob) = glob {
            if !glob_matches_name(glob, &rel) {
                continue;
            }
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
        let lines: Vec<&str> = content.lines().collect();
        let matches: Vec<FileMatch> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| re.is_match(line))
            .take(SEARCH_MAX_COUNT_PER_FILE)
            .map(|(idx, _)| FileMatch {
                rel: rel.clone(),
                line_no: idx + 1,
            })
            .collect();
        if matches.is_empty() {
            continue;
        }
        match search.output_mode {
            OutputMode::FilesWithMatches => {
                files_with_matches.push(rel.clone());
                if files_with_matches.len() >= cap {
                    break 'outer;
                }
            }
            OutputMode::Count => {
                counts.push((rel.clone(), matches.len()));
                if counts.len() >= cap {
                    break 'outer;
                }
            }
            OutputMode::Content => {
                for m in &matches {
                    content_hits.push(format_match_with_context(
                        &lines,
                        m,
                        search.before,
                        search.after,
                    ));
                    if content_hits.len() >= cap {
                        break 'outer;
                    }
                }
            }
        }
    }

    Ok(match search.output_mode {
        OutputMode::Content => {
            if content_hits.is_empty() {
                "No matches. Try a simpler pattern, a narrower path, or find_files by name."
                    .to_string()
            } else {
                content_hits.join("\n")
            }
        }
        OutputMode::FilesWithMatches => {
            if files_with_matches.is_empty() {
                "No files matched.".to_string()
            } else {
                files_with_matches.join("\n")
            }
        }
        OutputMode::Count => {
            if counts.is_empty() {
                "No matches.".to_string()
            } else {
                counts
                    .into_iter()
                    .map(|(rel, n)| format!("{rel}:{n}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    })
}

/// `rg`-style block: context lines use `path-line-text`, the match line uses `path:line:text`.
fn format_match_with_context(
    lines: &[&str],
    m: &FileMatch,
    before: Option<u64>,
    after: Option<u64>,
) -> String {
    let idx = m.line_no - 1;
    let start = idx.saturating_sub(before.unwrap_or(0) as usize);
    let end = (idx + after.unwrap_or(0) as usize + 1).min(lines.len());
    let mut out = Vec::with_capacity(end - start);
    for (i, line) in lines[start..end].iter().enumerate() {
        let line_no = start + i + 1;
        let sep = if line_no == m.line_no { ':' } else { '-' };
        out.push(truncate_hit_line(&format!(
            "{}{sep}{line_no}{sep}{line}",
            m.rel
        )));
    }
    out.join("\n")
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
        let db = std::env::temp_dir().join(format!("peek-search-{}.db", uuid::Uuid::new_v4()));
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
    fn search_files_returns_relative_path_and_line() {
        let root = std::env::temp_dir().join(format!("peek-search-ws-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src").join("lib.rs"),
            "fn send_email() {}\nfn other() {}\n",
        )
        .unwrap();

        let (ctx, db) = make_ctx(root.clone());
        let out = SearchFilesTool
            .execute(&ctx, json!({ "pattern": "send_email", "glob": "*.rs" }))
            .unwrap();
        assert!(out.contains("src/lib.rs:1:"), "{out}");
        assert!(out.contains("send_email"), "{out}");
        assert!(!out.contains("No matches"), "{out}");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn empty_search_is_explicit() {
        assert!(format_hits("", OutputMode::Content, None).contains("No matches"));
    }

    #[test]
    fn query_is_an_alias_for_pattern() {
        let parsed = parse_search_args(&json!({ "query": "foo" })).unwrap();
        assert_eq!(parsed.pattern, "foo");
        let err = parse_search_args(&json!({})).unwrap_err();
        assert!(err.to_string().contains("pattern"));
    }

    #[test]
    fn parses_context_and_case_insensitive_and_output_mode() {
        let parsed = parse_search_args(&json!({
            "pattern": "foo",
            "-i": true,
            "-C": 3,
            "output_mode": "files_with_matches",
            "head_limit": 5
        }))
        .unwrap();
        assert!(parsed.case_insensitive);
        assert_eq!(parsed.before, Some(3));
        assert_eq!(parsed.after, Some(3));
        assert_eq!(parsed.output_mode, OutputMode::FilesWithMatches);
        assert_eq!(parsed.head_limit, Some(5));
    }

    #[test]
    fn dash_a_and_dash_b_override_dash_c() {
        let parsed = parse_search_args(&json!({ "pattern": "foo", "-C": 2, "-A": 5 })).unwrap();
        assert_eq!(parsed.before, Some(2));
        assert_eq!(parsed.after, Some(5));
    }

    #[test]
    fn walk_search_fallback_respects_case_insensitive_and_context() {
        let root =
            std::env::temp_dir().join(format!("peek-search-fallback-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("notes.txt"),
            "before line\nTARGET here\nafter line\n",
        )
        .unwrap();

        let (ctx, _db) = make_ctx(root.clone());
        let search = SearchArgs {
            pattern: "target".into(),
            case_insensitive: true,
            before: Some(1),
            after: Some(1),
            output_mode: OutputMode::Content,
            head_limit: None,
        };
        let out = walk_search(&ctx, &root, &search, None).unwrap();
        assert!(out.contains("notes.txt-1-before line"), "{out}");
        assert!(out.contains("notes.txt:2:TARGET here"), "{out}");
        assert!(out.contains("notes.txt-3-after line"), "{out}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn walk_search_fallback_files_with_matches_and_count() {
        let root =
            std::env::temp_dir().join(format!("peek-search-fallback2-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "foo\nfoo\n").unwrap();
        std::fs::write(root.join("b.txt"), "bar\n").unwrap();

        let (ctx, _db) = make_ctx(root.clone());
        let files_mode = SearchArgs {
            pattern: "foo".into(),
            case_insensitive: false,
            before: None,
            after: None,
            output_mode: OutputMode::FilesWithMatches,
            head_limit: None,
        };
        let files_out = walk_search(&ctx, &root, &files_mode, None).unwrap();
        assert_eq!(files_out.trim(), "a.txt");

        let count_mode = SearchArgs {
            pattern: "foo".into(),
            case_insensitive: false,
            before: None,
            after: None,
            output_mode: OutputMode::Count,
            head_limit: None,
        };
        let count_out = walk_search(&ctx, &root, &count_mode, None).unwrap();
        assert_eq!(count_out.trim(), "a.txt:2");

        let _ = std::fs::remove_dir_all(&root);
    }
}
