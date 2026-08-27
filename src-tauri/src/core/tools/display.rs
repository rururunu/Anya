use serde_json::Value;

/// Presentation model for a tool call in the chat timeline.
pub struct ToolActivityView {
    pub title: String,
    pub detail: Option<String>,
    pub kind: String,
}

/// Build a human-readable activity card from a tool name, args, and optional result.
pub fn build_activity_view(
    tool_name: &str,
    args: &Value,
    result: Option<&str>,
) -> ToolActivityView {
    let kind = activity_kind(tool_name);
    let mut title = build_title(tool_name, args);
    if result.is_some_and(|r| r.contains("fuzzy")) {
        title.push_str(" (fuzzy)");
    }
    let detail = match tool_name {
        "run_shell" | "read_shell_output" | "wait_for_shell" => {
            let cmd = args["command"]
                .as_str()
                .or_else(|| args["job_id"].as_str())
                .unwrap_or("");
            let output = result.unwrap_or("");
            if output.is_empty() && result.is_none() {
                if cmd.is_empty() {
                    None
                } else {
                    Some(format!("```powershell\n{cmd}\n```"))
                }
            } else {
                Some(format!(
                    "```powershell\n{cmd}\n```\n\n**输出：**\n```\n{}\n```",
                    truncate(output, 4000)
                ))
            }
        }
        "read_file"
        | "list_folder"
        | "find_files"
        | "search_files"
        | "list_symbols"
        | "search_codebase"
        | "fetch_url"
        | "web_search"
        | "browser_read"
        | "get_context"
        | "get_workspace"
        | "word_get_document_content"
        | "word_get_selection"
        | "word_get_document_range"
        | "word_get_document_paragraphs"
        | "word_list_comments"
        | "excel_get_selection"
        | "excel_get_used_range"
        | "ppt_get_selection"
        | "ppt_get_slide_text" => result
            .filter(|value| !should_hide_result_detail(tool_name, value))
            .map(str::to_string),
        _ => result
            .filter(|value| !should_hide_result_detail(tool_name, value))
            .map(|r| r.to_string())
            .or_else(|| build_detail_from_args(tool_name, args)),
    };
    ToolActivityView {
        title,
        detail,
        kind,
    }
}

fn should_hide_result_detail(tool_name: &str, result: &str) -> bool {
    if result.starts_with("tool error:") {
        return false;
    }
    if matches!(tool_name, "update_tasks" | "todo_write")
        && matches!(result.trim(), "updated" | "ok" | "success")
    {
        return true;
    }
    if matches!(
        tool_name,
        "read_file"
            | "list_folder"
            | "find_files"
            | "search_files"
            | "list_symbols"
            | "search_codebase"
            | "fetch_url"
            | "web_search"
            | "browser_read"
            | "get_context"
            | "get_workspace"
            | "word_get_document_content"
            | "word_get_selection"
            | "word_get_document_range"
            | "word_get_document_paragraphs"
            | "word_list_comments"
            | "excel_get_selection"
            | "excel_get_used_range"
            | "ppt_get_selection"
            | "ppt_get_slide_text"
    ) {
        return true;
    }
    matches!(
        tool_name,
        "apply_patch"
            | "write_file"
            | "replace_in_file"
            | "replace_many_in_file"
            | "move_path"
            | "delete_text_range"
            | "delete_go_symbol"
            | "edit_notebook_cell"
            | "generate_image"
    ) && (matches!(
        result.trim(),
        "written" | "replaced" | "moved" | "deleted" | "updated"
    ) || result.trim().starts_with("replaced (")
        || result.trim().starts_with("applied ")
        || result.contains("![") && result.contains("path:"))
}

fn activity_kind(tool_name: &str) -> String {
    match tool_name {
        "run_shell" | "read_shell_output" | "wait_for_shell" | "stop_shell" => "shell".into(),
        "write_file" => "create".into(),
        "apply_patch" | "replace_in_file" | "replace_many_in_file" | "edit_notebook_cell" => {
            "edit".into()
        }
        "delete_text_range" | "delete_go_symbol" => "delete".into(),
        "move_path" => "move".into(),
        "read_file"
        | "list_folder"
        | "find_files"
        | "search_files"
        | "list_symbols"
        | "search_codebase"
        | "lsp"
        | "fetch_url"
        | "web_search"
        | "browser_read"
        | "get_context"
        | "get_workspace"
        | "word_get_document_content"
        | "word_get_selection"
        | "word_get_document_range"
        | "word_get_document_paragraphs"
        | "word_list_comments"
        | "excel_get_selection"
        | "excel_get_used_range"
        | "ppt_get_selection"
        | "ppt_get_slide_text" => "read".into(),
        "word_replace_selection"
        | "word_insert_text"
        | "word_insert_table"
        | "word_apply_font"
        | "word_save_document"
        | "word_add_comment"
        | "word_accept_all_revisions"
        | "word_reject_all_revisions"
        | "excel_set_selection"
        | "excel_save_workbook"
        | "ppt_replace_selection"
        | "ppt_insert_text"
        | "ppt_save_presentation" => "edit".into(),
        "generate_image" => "image".into(),
        _ => "other".into(),
    }
}

/// Cursor-style titles: `Verb Object Constraints` — no colon, no repeated “file”.
fn build_title(tool_name: &str, args: &Value) -> String {
    match tool_name {
        "run_shell" => {
            let description = args["description"].as_str().unwrap_or("").trim();
            if !description.is_empty() {
                truncate(description, 120)
            } else {
                let cmd = truncate(args["command"].as_str().unwrap_or(""), 100);
                if cmd.is_empty() {
                    "Run shell".into()
                } else {
                    format!("Run {cmd}")
                }
            }
        }
        "wait_for_shell" => format!("Wait {}", job_arg(args)),
        "read_shell_output" => format!("Read output {}", job_arg(args)),
        "stop_shell" => format!("Stop {}", job_arg(args)),

        "read_file" => {
            let path = display_path(path_arg(args));
            match line_range(args) {
                Some(range) => format!("Read {path} {range}"),
                None => format!("Read {path}"),
            }
        }
        "list_folder" => format!("List {}", display_path(path_arg(args))),
        "find_files" => format!(
            "Find {}",
            truncate(args["pattern"].as_str().unwrap_or("*"), 80)
        ),
        "search_files" => format!(
            "Search {}",
            truncate(args["pattern"].as_str().unwrap_or(""), 80)
        ),
        "list_symbols" => format!("Symbols {}", display_path(path_arg(args))),
        "search_codebase" => {
            let q = args["query"]
                .as_str()
                .or_else(|| args["pattern"].as_str())
                .unwrap_or("");
            if q.is_empty() {
                "Search codebase".into()
            } else {
                format!("Search codebase {}", truncate(q, 80))
            }
        }
        "lsp" => {
            let action = args["action"].as_str().unwrap_or("query");
            let path = display_path(path_arg(args));
            if path == "." || path.is_empty() {
                format!("LSP {action}")
            } else {
                format!("LSP {action} {path}")
            }
        }

        "write_file" => format!("Write {}", display_path(path_arg(args))),
        "replace_in_file" => format!("Edit {}", display_path(path_arg(args))),
        "replace_many_in_file" => {
            let n = args["edits"].as_array().map(|a| a.len()).unwrap_or(0);
            let path = display_path(path_arg(args));
            if n > 1 {
                format!("Edit {path} ({n})")
            } else {
                format!("Edit {path}")
            }
        }
        "apply_patch" => {
            let input = args["input"]
                .as_str()
                .or_else(|| args["patch"].as_str())
                .unwrap_or("");
            let files = input
                .lines()
                .filter_map(|line| {
                    let t = line.trim();
                    t.strip_prefix("*** Add File: ")
                        .or_else(|| t.strip_prefix("*** Update File: "))
                        .or_else(|| t.strip_prefix("*** Delete File: "))
                        .map(str::trim)
                })
                .collect::<Vec<_>>();
            match files.as_slice() {
                [] => "Patch".into(),
                [one] => format!("Patch {}", display_path(one)),
                many => format!("Patch {} files", many.len()),
            }
        }
        "move_path" => format!(
            "Move {} → {}",
            display_path(args["from"].as_str().unwrap_or("")),
            display_path(args["to"].as_str().unwrap_or(""))
        ),
        "delete_text_range" => format!("Delete in {}", display_path(path_arg(args))),
        "delete_go_symbol" => {
            let symbol = args["symbol"].as_str().unwrap_or("symbol");
            format!("Delete {symbol} in {}", display_path(path_arg(args)))
        }
        "edit_notebook_cell" => {
            let idx = args["cell_index"].as_u64();
            let path = display_path(path_arg(args));
            match idx {
                Some(i) => format!("Edit notebook {path} #{i}"),
                None => format!("Edit notebook {path}"),
            }
        }

        "run_subagent" => {
            let desc = args["description"].as_str().unwrap_or("").trim();
            if desc.is_empty() {
                "Subagent".into()
            } else {
                format!("Subagent {}", truncate(desc, 80))
            }
        }
        "run_parallel_subagents" => {
            let n = args["tasks"].as_array().map(|a| a.len()).unwrap_or(0);
            if n == 0 {
                "Subagents".into()
            } else {
                format!("Subagents ({n})")
            }
        }
        "ask_user" => "Ask user".into(),
        "share_to_companion" => "Share file".into(),
        "share_preview_url" => "Share preview URL".into(),
        "generate_image" => {
            let prompt = truncate(args["prompt"].as_str().unwrap_or("").trim(), 80);
            if prompt.is_empty() {
                "Generate image".into()
            } else {
                format!("Generate {prompt}")
            }
        }
        "update_tasks" | "todo_write" => "Update tasks".into(),
        "complete_plan_step" => "Complete plan step".into(),

        "web_search" => {
            let q = truncate(
                args["query"]
                    .as_str()
                    .or_else(|| args["q"].as_str())
                    .or_else(|| args["search"].as_str())
                    .unwrap_or(""),
                80,
            );
            if q.is_empty() {
                "Web search".into()
            } else {
                format!("Web search {q}")
            }
        }
        "browser_read" | "fetch_url" => {
            let url = truncate(
                args["url"]
                    .as_str()
                    .or_else(|| args["uri"].as_str())
                    .or_else(|| args["href"].as_str())
                    .unwrap_or(""),
                80,
            );
            if url.is_empty() {
                "Fetch page".into()
            } else {
                format!("Fetch {url}")
            }
        }
        "get_context" => "Read context".into(),
        "get_workspace" => "Read workspace".into(),

        "git" => {
            let action = args["action"].as_str().unwrap_or("");
            if action.is_empty() {
                "Git".into()
            } else {
                format!("Git {action}")
            }
        }
        "git_commit" => "Git commit".into(),

        "word_get_document_content" => "Read Word".into(),
        "word_get_selection" => "Read Word selection".into(),
        "word_get_document_range" => word_range_title(args),
        "word_get_document_paragraphs" => "Read Word paragraphs".into(),
        "word_list_comments" => "List Word comments".into(),
        "word_replace_selection" => "Edit Word selection".into(),
        "word_insert_text" => "Insert Word text".into(),
        "word_insert_table" => "Insert Word table".into(),
        "word_apply_font" => "Apply Word font".into(),
        "word_add_comment" => "Add Word comment".into(),
        "word_accept_all_revisions" => "Accept Word revisions".into(),
        "word_reject_all_revisions" => "Reject Word revisions".into(),
        "word_save_document" => "Save Word".into(),
        "excel_get_selection" => "Read Excel selection".into(),
        "excel_get_used_range" => "Read Excel range".into(),
        "excel_set_selection" => "Edit Excel selection".into(),
        "excel_save_workbook" => "Save Excel".into(),
        "ppt_get_selection" => "Read PPT selection".into(),
        "ppt_get_slide_text" => "Read PPT slide".into(),
        "ppt_replace_selection" => "Edit PPT selection".into(),
        "ppt_insert_text" => "Insert PPT text".into(),
        "ppt_save_presentation" => "Save PPT".into(),

        "save_memory" => "Save memory".into(),
        "search_memory" => {
            let q = truncate(args["query"].as_str().unwrap_or(""), 60);
            if q.is_empty() {
                "Search memory".into()
            } else {
                format!("Search memory {q}")
            }
        }
        "delete_memory" => "Delete memory".into(),
        "search_past_chats" => {
            let q = truncate(args["query"].as_str().unwrap_or(""), 60);
            if q.is_empty() {
                "Search chats".into()
            } else {
                format!("Search chats {q}")
            }
        }
        "read_chat" => format!("Read chat {}", args["session_id"].as_str().unwrap_or("")),
        "list_chats" => "List chats".into(),

        "load_skill" => format!("Load skill {}", args["name"].as_str().unwrap_or("")),
        "run_skill" => format!("Run skill {}", args["name"].as_str().unwrap_or("")),
        "list_skills" => "List skills".into(),

        other => humanize_tool_name(other),
    }
}

fn build_detail_from_args(tool_name: &str, args: &Value) -> Option<String> {
    match tool_name {
        "run_shell" => None,
        "write_file" => {
            let content = args["content"].as_str().unwrap_or("");
            Some(format!("```\n{}\n```", truncate(content, 2000)))
        }
        "apply_patch" => {
            let input = args["input"]
                .as_str()
                .or_else(|| args["patch"].as_str())
                .unwrap_or("");
            Some(format!("```\n{}\n```", truncate(input, 4000)))
        }
        "replace_in_file" => {
            let old = args["old_string"].as_str().unwrap_or("");
            let new = args["new_string"].as_str().unwrap_or("");
            Some(format_diff(old, new))
        }
        "replace_many_in_file" => {
            let edits = args["edits"].as_array()?;
            let mut parts = Vec::new();
            for (idx, edit) in edits.iter().enumerate() {
                let old = edit["old_string"].as_str().unwrap_or("");
                let new = edit["new_string"].as_str().unwrap_or("");
                parts.push(format!("### 编辑 {}\n{}", idx + 1, format_diff(old, new)));
            }
            Some(parts.join("\n\n"))
        }
        "delete_text_range" => {
            let start = args["start_anchor"].as_str().unwrap_or("");
            let end = args["end_anchor"].as_str().unwrap_or("");
            Some(format!("删除锚点区间：\n```\n{start}\n…\n{end}\n```"))
        }
        "delete_go_symbol" => Some(format!(
            "删除符号：`{}`",
            args["symbol"].as_str().unwrap_or("")
        )),
        "run_subagent" => args["prompt"]
            .as_str()
            .map(|prompt| truncate(prompt, 1_200)),
        "run_parallel_subagents" => args["tasks"].as_array().map(|tasks| {
            let descriptions = tasks
                .iter()
                .enumerate()
                .filter_map(|(index, task)| {
                    task["prompt"]
                        .as_str()
                        .map(|prompt| format!("{}. {}", index + 1, truncate(prompt, 400)))
                })
                .collect::<Vec<_>>();
            descriptions.join("\n\n")
        }),
        "update_tasks" | "todo_write" => args["tasks"].as_array().map(|tasks| {
            tasks
                .iter()
                .filter_map(|task| {
                    let content = task["content"].as_str()?.trim();
                    if content.is_empty() {
                        return None;
                    }
                    let status = task["status"]
                        .as_str()
                        .unwrap_or("pending")
                        .to_ascii_lowercase();
                    let marker = match status.as_str() {
                        "completed" | "done" | "complete" => "[x]",
                        "in_progress" | "in-progress" | "active" | "running" => "[~]",
                        "cancelled" | "canceled" => "[-]",
                        _ => "[ ]",
                    };
                    let level = task["level"].as_u64().unwrap_or(0) as usize;
                    let indent = "  ".repeat(level.min(6));
                    Some(format!("{indent}- {marker} {content}"))
                })
                .collect::<Vec<_>>()
                .join("\n")
        }),
        _ => None,
    }
}

fn format_diff(old: &str, new: &str) -> String {
    let mut out = String::from("```diff\n");
    for line in old.lines() {
        out.push_str(&format!("-{line}\n"));
    }
    for line in new.lines() {
        out.push_str(&format!("+{line}\n"));
    }
    out.push_str("```");
    out
}

fn path_arg(args: &Value) -> &str {
    args["path"].as_str().unwrap_or(".")
}

fn job_arg(args: &Value) -> &str {
    let id = args["job_id"].as_str().unwrap_or("job");
    if id.is_empty() {
        "job"
    } else {
        id
    }
}

/// Prefer a short path for the timeline title (last 1–2 segments).
fn display_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches(|c| c == '/' || c == '\\');
    if trimmed.is_empty() || trimmed == "." {
        return ".".into();
    }
    let parts: Vec<&str> = trimmed
        .split(|c| c == '/' || c == '\\')
        .filter(|p| !p.is_empty())
        .collect();
    match parts.as_slice() {
        [] => ".".into(),
        [one] => (*one).to_string(),
        [.., parent, name] => format!("{parent}/{name}"),
    }
}

fn line_range(args: &Value) -> Option<String> {
    if let Some(around) = args
        .get("around_line")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
    {
        let context = args
            .get("context")
            .and_then(Value::as_u64)
            .unwrap_or(40)
            .clamp(1, 80);
        let start = around.saturating_sub(context).max(1);
        let end = around.saturating_add(context);
        return Some(format!("L{start}-{end}"));
    }
    let start = args
        .get("start_line")
        .and_then(Value::as_u64)
        .or_else(|| args.get("offset").and_then(Value::as_u64));
    let end_line = args.get("end_line").and_then(Value::as_u64);
    let has_limit = args.get("limit").is_some();
    if start.is_none() && end_line.is_none() && !has_limit {
        return None;
    }
    let offset = start.unwrap_or(1).max(1);
    if let Some(end) = end_line {
        return Some(format!("L{offset}-{}", end.max(offset)));
    }
    let limit = args["limit"].as_u64().unwrap_or(200).max(1);
    let end = offset.saturating_add(limit).saturating_sub(1);
    Some(format!("L{offset}-{end}"))
}

fn word_range_title(args: &Value) -> String {
    let start = args["start_char"]
        .as_u64()
        .or_else(|| args["start"].as_u64());
    let end = args["end_char"].as_u64().or_else(|| args["end"].as_u64());
    match (start, end) {
        (Some(s), Some(e)) => format!("Read Word chars {s}-{e}"),
        _ => "Read Word range".into(),
    }
}

fn humanize_tool_name(name: &str) -> String {
    if name.trim().is_empty() {
        return "Unknown tool".into();
    }
    let words: Vec<String> = name
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect();
    if words.is_empty() {
        name.to_string()
    } else {
        words.join(" ")
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replace_in_file_shows_diff() {
        let args = json!({
            "path": "src/main.rs",
            "old_string": "fn old() {}",
            "new_string": "fn new() {}"
        });
        let view = build_activity_view("replace_in_file", &args, Some("replaced"));
        assert_eq!(view.title, "Edit src/main.rs");
        let detail = view.detail.expect("detail");
        assert!(detail.contains("-fn old()"));
        assert!(detail.contains("+fn new()"));
    }

    #[test]
    fn run_shell_prefers_description_title() {
        let args = json!({
            "command": "cargo test --lib continues_when_token_budget",
            "description": "Run token budget continuation test"
        });
        let view = build_activity_view("run_shell", &args, Some("ok\n"));
        assert_eq!(view.title, "Run token budget continuation test");
        assert!(view.detail.unwrap().contains("cargo test"));
    }

    #[test]
    fn run_shell_shows_command_and_output() {
        let args = json!({ "command": "echo hello" });
        let view = build_activity_view("run_shell", &args, Some("hello\n"));
        assert_eq!(view.title, "Run echo hello");
        let detail = view.detail.unwrap();
        assert!(detail.contains("echo hello"));
        assert!(detail.contains("hello"));
    }

    #[test]
    fn read_file_title_is_verb_path_range() {
        let args = json!({ "path": "src-tauri/prompts/tools.md", "offset": 1, "limit": 25 });
        let view = build_activity_view("read_file", &args, Some("   1|ok\n"));
        assert_eq!(view.title, "Read prompts/tools.md L1-25");
        assert!(view.detail.is_none());
    }

    #[test]
    fn read_file_title_uses_around_line() {
        let args = json!({ "path": "src/main.rs", "around_line": 80, "context": 10 });
        let view = build_activity_view("read_file", &args, Some("  70|ok\n"));
        assert_eq!(view.title, "Read src/main.rs L70-90");
    }

    #[test]
    fn read_file_title_without_range() {
        let args = json!({ "path": "src/main.rs" });
        let view = build_activity_view("read_file", &args, Some("   1|line\n"));
        assert_eq!(view.title, "Read src/main.rs");
    }

    #[test]
    fn read_file_hides_content() {
        let args = json!({ "path": "src/main.rs" });
        let content = (1..=120)
            .map(|n| format!("{n:>6}|line {n}\n"))
            .collect::<String>();
        let view = build_activity_view("read_file", &args, Some(&content));
        assert_eq!(view.title, "Read src/main.rs");
        assert!(view.detail.is_none());
    }

    #[test]
    fn read_file_shows_error() {
        let args = json!({ "path": "missing.rs" });
        let view = build_activity_view("read_file", &args, Some("tool error: file not found"));
        assert_eq!(view.detail.as_deref(), Some("tool error: file not found"));
    }

    #[test]
    fn update_tasks_shows_checklist() {
        let args = json!({
            "tasks": [
                { "content": "Explore codebase", "status": "completed" },
                { "content": "Fix the bug", "status": "in_progress" },
                { "content": "Write tests", "status": "pending" }
            ]
        });
        let view = build_activity_view("update_tasks", &args, Some("updated"));
        assert_eq!(view.title, "Update tasks");
        let detail = view.detail.expect("detail");
        assert!(detail.contains("[x] Explore codebase"));
        assert!(detail.contains("[~] Fix the bug"));
        assert!(detail.contains("[ ] Write tests"));
    }

    #[test]
    fn search_and_find_titles_are_compact() {
        let search =
            build_activity_view("search_files", &json!({ "pattern": "build_title" }), None);
        assert_eq!(search.title, "Search build_title");
        let find = build_activity_view("find_files", &json!({ "pattern": "**/*.rs" }), None);
        assert_eq!(find.title, "Find **/*.rs");
    }

    #[test]
    fn generate_image_title_uses_prompt_and_image_kind() {
        let args = json!({ "prompt": "a cat sitting on a roof at dusk" });
        let view = build_activity_view(
            "generate_image",
            &args,
            Some("Generated 1 image with gpt-image-2 (1024x1024, auto).\n![a cat](path:C:/tmp/cat.png)"),
        );
        assert_eq!(view.title, "Generate a cat sitting on a roof at dusk");
        assert_eq!(view.kind, "image");
        assert!(view.detail.is_none());
    }

    #[test]
    fn generate_image_error_keeps_tool_result_as_detail() {
        let view = build_activity_view(
            "generate_image",
            &json!({ "prompt": "a cat" }),
            Some("tool error: Image API returned 401 from https://api.example/v1/images/generations"),
        );
        assert_eq!(view.kind, "image");
        assert_eq!(
            view.detail.as_deref(),
            Some("tool error: Image API returned 401 from https://api.example/v1/images/generations")
        );
    }

    #[test]
    fn empty_tool_name_uses_unknown_title_and_keeps_error() {
        let view = build_activity_view("", &json!({}), Some("tool error: unknown tool: "));
        assert_eq!(view.title, "Unknown tool");
        assert_eq!(view.kind, "other");
        assert_eq!(view.detail.as_deref(), Some("tool error: unknown tool: "));
    }
}
