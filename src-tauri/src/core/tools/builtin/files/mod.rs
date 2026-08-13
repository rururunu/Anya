//! File read/write builtin tools and shared helpers.

mod office;
mod read;
mod write;

pub use read::*;
pub use write::*;

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::fuzzy::apply_old_string_edit;
use crate::core::tools::path::resolve_tool_path;
use crate::core::tools::path_permission::PathAccess;
use crate::core::tools::preview::{unified_diff, ChangeKind, ToolPreview};

pub(super) fn resolve_read(
    ctx: &ToolContext,
    tool_name: &str,
    raw: &str,
) -> Result<std::path::PathBuf, ToolError> {
    resolve_tool_path(ctx, raw, PathAccess::Read, tool_name)
}

pub(super) fn resolve_write(
    ctx: &ToolContext,
    tool_name: &str,
    raw: &str,
) -> Result<std::path::PathBuf, ToolError> {
    resolve_tool_path(ctx, raw, PathAccess::Write, tool_name)
}

pub(super) fn required_string<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new(format!("{key} is required")))
}

pub(super) fn apply_many_edits(content: &str, args: &Value) -> Result<(String, usize), ToolError> {
    let edits = args
        .get("edits")
        .and_then(Value::as_array)
        .filter(|edits| !edits.is_empty())
        .ok_or_else(|| ToolError::new("edits must contain at least one replacement"))?;
    let mut updated = content.to_string();
    let mut fuzzy_count = 0usize;
    for (index, edit) in edits.iter().enumerate() {
        let old = required_string(edit, "old_string")
            .map_err(|error| ToolError::new(format!("edit {index}: {error}")))?;
        let new = edit
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new(format!("edit {index}: new_string is required")))?;
        let applied = apply_old_string_edit(&updated, old, new, false);
        if applied.applied != 1 {
            return Err(ToolError::new(format!(
                "edit {index}: old_string must appear exactly once, found {}",
                applied.matches
            )));
        }
        fuzzy_count += usize::from(applied.fuzzy);
        updated = applied.updated;
    }
    Ok((updated, fuzzy_count))
}

pub(super) fn single_edit_preview(
    path: &str,
    content: String,
    old: &str,
    new: &str,
) -> Option<ToolPreview> {
    let applied = apply_old_string_edit(&content, old, new, false);
    if applied.applied != 1 {
        return None;
    }
    let diff = unified_diff(path, &content, &applied.updated);
    Some(ToolPreview {
        path: path.to_string(),
        affected_paths: vec![path.to_string()],
        kind: ChangeKind::Modify,
        old_text: Some(content),
        new_text: Some(applied.updated),
        unified_diff: diff,
    })
}

/// Rejects whole-file edits: an `old_string` covering most of a file hides the real change.
pub(super) fn guard_minimal_edit(
    tool_name: &str,
    old_strings: &[&str],
    content: &str,
) -> Result<(), ToolError> {
    let total = content.lines().count();
    if total < 20 {
        return Ok(());
    }
    for old in old_strings {
        let old_lines = old.lines().count();
        if old_lines * 100 >= total * 80 {
            return Err(ToolError::new(format!(
                "{tool_name}: old_string covers {old_lines}/{total} lines (~{}%) — pass ONLY the lines that change (e.g. `- const a = 1` / `+ const a = 2`), never whole-file content or long unchanged blocks. For a connected block rewrite use `apply_patch` with minimal hunks; do NOT fall back to `write_file` on an existing file.",
                old_lines * 100 / total
            )));
        }
    }
    Ok(())
}

pub(super) fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name, ".git" | "node_modules" | "target" | "dist")
}

pub(super) fn run_command_cancellable(
    ctx: &ToolContext,
    command: &mut Command,
) -> Result<Option<std::process::Output>, ToolError> {
    ctx.ensure_not_cancelled()?;
    let stdout_path =
        std::env::temp_dir().join(format!("peek-tool-{}.stdout", uuid::Uuid::new_v4()));
    let stderr_path =
        std::env::temp_dir().join(format!("peek-tool-{}.stderr", uuid::Uuid::new_v4()));
    let stdout_file = fs::File::create(&stdout_path)?;
    let stderr_file = fs::File::create(&stderr_path)?;
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Ok(None);
        }
    };
    let status = loop {
        if ctx.is_cancelled() {
            crate::core::tools::shell_jobs::terminate_process_tree(&mut child);
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(ToolError::cancelled());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Err(ToolError::new(error.to_string()));
            }
        }
    };
    let stdout = fs::read(&stdout_path).unwrap_or_default();
    let stderr = fs::read(&stderr_path).unwrap_or_default();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Ok(Some(std::process::Output {
        status,
        stdout,
        stderr,
    }))
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replace_many_requires_at_least_one_edit() {
        let error = apply_many_edits("unchanged", &json!({ "edits": [] })).unwrap_err();
        assert!(error.to_string().contains("at least one"));
    }

    #[test]
    fn replace_many_is_all_or_nothing_in_memory() {
        let original = "one\ntwo\n";
        let error = apply_many_edits(
            original,
            &json!({
                "edits": [
                    { "old_string": "one", "new_string": "ONE" },
                    { "old_string": "missing", "new_string": "MISSING" }
                ]
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("edit 1"));
        assert_eq!(original, "one\ntwo\n");
    }

    #[test]
    fn replace_single_preview_uses_complete_file_contents() {
        let preview =
            single_edit_preview("file.txt", "before\nold\nafter\n".into(), "old", "new").unwrap();
        assert_eq!(preview.old_text.as_deref(), Some("before\nold\nafter\n"));
        assert_eq!(preview.new_text.as_deref(), Some("before\nnew\nafter\n"));
        assert!(preview.unified_diff.contains(" before"));
        assert!(preview.unified_diff.contains("-old"));
        assert!(preview.unified_diff.contains("+new"));
    }

    #[test]
    fn full_file_edit_is_rejected_by_the_guard() {
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = guard_minimal_edit("replace_in_file", &[&content], &content).unwrap_err();
        assert!(error.to_string().contains("old_string covers 50/50 lines"));
        assert!(error.to_string().contains("const a = 1"));
    }

    #[test]
    fn majority_edit_is_rejected_by_the_guard() {
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let big = (0..45)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = guard_minimal_edit("replace_in_file", &[&big], &content).unwrap_err();
        assert!(error.to_string().contains("old_string covers 45/50 lines"));
    }

    #[test]
    fn minimal_and_medium_edits_pass_the_guard() {
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        guard_minimal_edit("replace_in_file", &["line 3"], &content).unwrap();
        let medium = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        guard_minimal_edit("replace_in_file", &[&medium], &content).unwrap();
    }

    #[test]
    fn small_files_are_not_guarded() {
        let content = "a\nb\nc\nd\ne\n";
        guard_minimal_edit("replace_in_file", &[&content], &content).unwrap();
    }
}
