use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::path::Path;
use std::process::{Command, Stdio};
use tauri::State;

use crate::app_state::AppState;
use crate::runtime::terminal::prepare_command;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffRequest {
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    #[serde(default)]
    pub unified_diff: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffDocument {
    pub rows: Vec<CodeDiffRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffRow {
    pub left: Option<CodeDiffLine>,
    pub right: Option<CodeDiffLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffLine {
    pub line_number: usize,
    pub text: String,
    pub kind: CodeDiffLineKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodeDiffLineKind {
    Context,
    Addition,
    Deletion,
}

#[tauri::command]
pub fn build_code_diff(request: CodeDiffRequest) -> CodeDiffDocument {
    match (request.old_text, request.new_text) {
        (Some(old_text), Some(new_text)) if !old_text.is_empty() || !new_text.is_empty() => {
            diff_text(&old_text, &new_text)
        }
        _ => parse_unified_diff(&request.unified_diff),
    }
}

const MAX_UNTRACKED_BYTES: u64 = 256 * 1024;
const MAX_UNTRACKED_FILES: usize = 40;
const MAX_DIFF_CHARS: usize = 1_500_000;

#[tauri::command]
pub fn working_tree_diff(state: State<'_, AppState>) -> Result<String, String> {
    let Some(workspace) = state.core.workspaces().current() else {
        return Ok(String::new());
    };
    working_tree_diff_at(Path::new(&workspace.root))
}

/** Unified + untracked diff for a workspace root (also used by Companion RPC). */
pub fn working_tree_diff_at(root: &Path) -> Result<String, String> {
    if git_output(root, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        return Ok(String::new());
    }
    let mut diff = git_output(root, &["diff", "HEAD"]).or_else(|_| git_output(root, &["diff"]))?;
    if let Ok(untracked) = git_output(root, &["ls-files", "--others", "--exclude-standard"]) {
        let mut added = 0;
        for path in untracked.lines().filter(|line| !line.is_empty()) {
            if added >= MAX_UNTRACKED_FILES || diff.len() >= MAX_DIFF_CHARS {
                break;
            }
            let Some(file_diff) = untracked_file_diff(root, path) else {
                continue;
            };
            if !diff.is_empty() && !diff.ends_with('\n') {
                diff.push('\n');
            }
            diff.push_str(&file_diff);
            added += 1;
        }
    }
    if diff.len() > MAX_DIFF_CHARS {
        diff.truncate(MAX_DIFF_CHARS);
    }
    Ok(diff)
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(["-c", "core.quotepath=false"]);
    command.args(args);
    command.current_dir(root);
    command.stdin(Stdio::null());
    prepare_command(&mut command);
    let output = command.output().map_err(|error| error.to_string())?;
    let text = crate::runtime::encoding::decode_process_bytes(&output.stdout);
    if !output.status.success() {
        let stderr = crate::runtime::encoding::decode_process_bytes(&output.stderr);
        return Err(stderr.trim().to_string());
    }
    Ok(text)
}

fn untracked_file_diff(root: &Path, path: &str) -> Option<String> {
    let file = root.join(path);
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_UNTRACKED_BYTES {
        return None;
    }
    let bytes = std::fs::read(&file).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let content = String::from_utf8_lossy(&bytes);
    Some(untracked_unified_diff(path, &content))
}

fn untracked_unified_diff(path: &str, content: &str) -> String {
    let line_count = if content.is_empty() {
        0
    } else {
        content.lines().count()
    };
    let mut out = format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{line_count} @@\n"
    );
    for line in content.lines() {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn diff_text(old_text: &str, new_text: &str) -> CodeDiffDocument {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut rows = Vec::new();
    let mut deletions = Vec::new();
    let mut additions = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                flush_changed_rows(&mut rows, &mut deletions, &mut additions);
                let line = CodeDiffLine {
                    line_number: change.old_index().unwrap_or_default() + 1,
                    text: line_text(change.value()),
                    kind: CodeDiffLineKind::Context,
                };
                rows.push(CodeDiffRow {
                    left: Some(line.clone()),
                    right: Some(CodeDiffLine {
                        line_number: change.new_index().unwrap_or_default() + 1,
                        ..line
                    }),
                });
            }
            ChangeTag::Delete => deletions.push(CodeDiffLine {
                line_number: change.old_index().unwrap_or_default() + 1,
                text: line_text(change.value()),
                kind: CodeDiffLineKind::Deletion,
            }),
            ChangeTag::Insert => additions.push(CodeDiffLine {
                line_number: change.new_index().unwrap_or_default() + 1,
                text: line_text(change.value()),
                kind: CodeDiffLineKind::Addition,
            }),
        }
    }

    flush_changed_rows(&mut rows, &mut deletions, &mut additions);
    CodeDiffDocument { rows }
}

fn parse_unified_diff(diff: &str) -> CodeDiffDocument {
    let mut rows = Vec::new();
    let mut deletions = Vec::new();
    let mut additions = Vec::new();
    let mut old_line = 0usize;
    let mut new_line = 0usize;

    for raw in diff.replace("\r\n", "\n").lines() {
        if let Some((old_start, new_start)) = hunk_starts(raw) {
            flush_changed_rows(&mut rows, &mut deletions, &mut additions);
            old_line = old_start;
            new_line = new_start;
            continue;
        }
        if raw.starts_with("--- ")
            || raw.starts_with("+++ ")
            || raw.starts_with("diff ")
            || raw.starts_with("index ")
        {
            continue;
        }
        if let Some(text) = raw.strip_prefix('-') {
            deletions.push(CodeDiffLine {
                line_number: old_line,
                text: text.to_owned(),
                kind: CodeDiffLineKind::Deletion,
            });
            old_line += 1;
            continue;
        }
        if let Some(text) = raw.strip_prefix('+') {
            additions.push(CodeDiffLine {
                line_number: new_line,
                text: text.to_owned(),
                kind: CodeDiffLineKind::Addition,
            });
            new_line += 1;
            continue;
        }
        if let Some(text) = raw.strip_prefix(' ') {
            flush_changed_rows(&mut rows, &mut deletions, &mut additions);
            rows.push(CodeDiffRow {
                left: Some(CodeDiffLine {
                    line_number: old_line,
                    text: text.to_owned(),
                    kind: CodeDiffLineKind::Context,
                }),
                right: Some(CodeDiffLine {
                    line_number: new_line,
                    text: text.to_owned(),
                    kind: CodeDiffLineKind::Context,
                }),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    flush_changed_rows(&mut rows, &mut deletions, &mut additions);
    CodeDiffDocument { rows }
}

fn hunk_starts(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "@@" {
        return None;
    }
    let old_start = parts
        .next()?
        .strip_prefix('-')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
    let new_start = parts
        .next()?
        .strip_prefix('+')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
    Some((old_start, new_start))
}

fn flush_changed_rows(
    rows: &mut Vec<CodeDiffRow>,
    deletions: &mut Vec<CodeDiffLine>,
    additions: &mut Vec<CodeDiffLine>,
) {
    let row_count = deletions.len().max(additions.len());
    rows.extend((0..row_count).map(|index| CodeDiffRow {
        left: deletions.get(index).cloned(),
        right: additions.get(index).cloned(),
    }));
    deletions.clear();
    additions.clear();
}

fn line_text(value: &str) -> String {
    value.trim_end_matches(['\r', '\n']).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        build_code_diff, diff_text, parse_unified_diff, untracked_unified_diff, CodeDiffLineKind,
        CodeDiffRequest,
    };

    #[test]
    fn aligns_replacements_with_similar() {
        let document = diff_text("one\nold\nthree\n", "one\nnew\nthree\n");
        assert_eq!(document.rows.len(), 3);
        assert_eq!(
            document.rows[1].left.as_ref().unwrap().kind,
            CodeDiffLineKind::Deletion
        );
        assert_eq!(
            document.rows[1].right.as_ref().unwrap().kind,
            CodeDiffLineKind::Addition
        );
        assert_eq!(document.rows[1].left.as_ref().unwrap().line_number, 2);
        assert_eq!(document.rows[1].right.as_ref().unwrap().line_number, 2);
    }

    #[test]
    fn keeps_empty_counterpart_for_insertions() {
        let document = diff_text("one\n", "one\ntwo\nthree\n");
        assert_eq!(document.rows.len(), 3);
        assert!(document.rows[1].left.is_none());
        assert_eq!(document.rows[1].right.as_ref().unwrap().line_number, 2);
    }

    #[test]
    fn parses_legacy_unified_diff() {
        let document =
            parse_unified_diff("--- a/file\n+++ b/file\n@@ -3,2 +3,2 @@\n old\n-old\n+new\n");
        assert_eq!(document.rows.len(), 2);
        assert_eq!(document.rows[0].left.as_ref().unwrap().line_number, 3);
        assert_eq!(
            document.rows[1].right.as_ref().unwrap().kind,
            CodeDiffLineKind::Addition
        );
    }

    #[test]
    fn falls_back_to_unified_diff_when_preview_text_is_empty() {
        let document = build_code_diff(CodeDiffRequest {
            old_text: Some(String::new()),
            new_text: Some(String::new()),
            unified_diff: "@@ -1 +1 @@\n-old\n+new\n".to_owned(),
        });

        assert_eq!(document.rows.len(), 1);
        assert_eq!(document.rows[0].left.as_ref().unwrap().text, "old");
        assert_eq!(document.rows[0].right.as_ref().unwrap().text, "new");
    }

    #[test]
    fn untracked_diff_uses_dev_null_old_side() {
        let diff = untracked_unified_diff("src/new.ts", "hello\n");
        assert!(diff.contains("--- /dev/null"));
        assert!(diff.contains("+++ b/src/new.ts"));
        assert!(diff.contains("+hello"));
    }
}
