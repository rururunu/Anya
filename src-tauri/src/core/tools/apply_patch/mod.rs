//! Codex-compatible apply_patch: parse `*** Begin Patch` envelopes and apply Add/Update/Delete.
mod parser;
mod seek_sequence;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::context::{Tool, ToolContext};
use super::error::ToolError;
use super::file_io::atomic_write;
use super::path::{normalize_path, resolve_tool_path};
use super::path_permission::PathAccess;
use super::preview::{unified_diff, ChangeKind, ToolPreview};
use parser::{parse_patch, Hunk, ParseError, UpdateChunk};
use seek_sequence::seek_sequence;

pub fn register(registry: &mut super::registry::ToolRegistry) {
    registry.register(std::sync::Arc::new(ApplyPatchTool));
}

struct ApplyPatchTool;

impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a Codex-style patch for structural insertions/deletions, connected block rewrites, or coordinated multi-hunk/multi-file edits. Not a unified git diff.

Usage:
- Do NOT use for one or several localized replacements — use replace_in_file or replace_many_in_file instead.
- Hunks contain ONLY changed lines plus the minimal context needed; never echo an entire file.
- Wrap with `*** Begin Patch` / `*** End Patch`.
- File ops use three asterisks: `*** Update File: path`, `*** Add File: path`, `*** Delete File: path`.
- Hunks use `@@` then lines starting with ` ` / `-` / `+`.
- Example:
*** Begin Patch
*** Update File: README.md
@@
-old
+new
*** End Patch"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Codex apply_patch text. Must use *** Begin Patch / *** End Patch and *** Update File: / *** Add File: / *** Delete File:. Do not pass `--- a/file` unified-diff headers."
                }
            },
            "required": ["input"]
        })
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let patch = patch_arg(args)?;
        let plan = plan_patch(ctx, &patch)?;
        let first = plan
            .ops
            .first()
            .ok_or_else(|| ToolError::new("patch contains no file operations"))?;
        let unified_diff = plan
            .ops
            .iter()
            .map(|op| op.diff.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let affected_paths = plan
            .ops
            .iter()
            .flat_map(|op| std::iter::once(op.display_path.clone()).chain(op.move_to.clone()))
            .collect();
        Ok(Some(ToolPreview {
            path: first.display_path.clone(),
            affected_paths,
            kind: first.kind,
            old_text: first.old_text.clone(),
            new_text: first.new_text.clone(),
            unified_diff,
        }))
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let patch = patch_arg(&args)?;
        let plan = plan_patch(ctx, &patch)?;
        execute_plan(&plan)?;
        let summary = plan
            .ops
            .iter()
            .map(|op| match op.kind {
                ChangeKind::Create => format!("A {}", op.display_path),
                ChangeKind::Delete => format!("D {}", op.display_path),
                ChangeKind::Modify => {
                    if let Some(moved) = &op.move_to {
                        format!("M {} -> {}", op.display_path, moved)
                    } else {
                        format!("M {}", op.display_path)
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!("applied {} file(s)\n{summary}", plan.ops.len()))
    }
}

fn patch_arg(args: &Value) -> Result<String, ToolError> {
    let raw = args
        .get("input")
        .or_else(|| args.get("patch"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if raw.is_empty() {
        return Err(ToolError::new("input patch is required"));
    }
    Ok(raw.to_string())
}

#[derive(Debug)]
struct PlannedOp {
    display_path: String,
    absolute: PathBuf,
    move_to: Option<String>,
    move_absolute: Option<PathBuf>,
    kind: ChangeKind,
    old_text: Option<String>,
    new_text: Option<String>,
    diff: String,
}

#[derive(Debug)]
struct PatchPlan {
    ops: Vec<PlannedOp>,
}

fn plan_patch(ctx: &ToolContext, patch: &str) -> Result<PatchPlan, ToolError> {
    let parsed = parse_patch(patch).map_err(|e| ToolError::new(e.to_string()))?;
    if parsed.hunks.is_empty() {
        return Err(ToolError::new("patch contains no file operations"));
    }
    let mut ops = Vec::new();
    for hunk in parsed.hunks {
        ops.push(plan_hunk(ctx, hunk)?);
    }
    reject_overlapping_operations(&ops)?;
    Ok(PatchPlan { ops })
}

fn reject_overlapping_operations(ops: &[PlannedOp]) -> Result<(), ToolError> {
    let mut touched = HashSet::new();
    for op in ops {
        for path in std::iter::once(&op.absolute).chain(op.move_absolute.as_ref()) {
            if !touched.insert(path.clone()) {
                return Err(ToolError::new(format!(
                    "patch contains multiple operations for {}; combine them into one file operation",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn plan_hunk(ctx: &ToolContext, hunk: Hunk) -> Result<PlannedOp, ToolError> {
    match hunk {
        Hunk::Add { path, contents } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            if absolute.exists() {
                return Err(ToolError::new(format!(
                    "cannot add existing file: {display}; use Update File instead"
                )));
            }
            Ok(PlannedOp {
                display_path: display.clone(),
                absolute,
                move_to: None,
                move_absolute: None,
                kind: ChangeKind::Create,
                old_text: None,
                new_text: Some(contents.clone()),
                diff: unified_diff(&display, "", &contents),
            })
        }
        Hunk::Delete { path } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            if !absolute.is_file() {
                return Err(ToolError::new(format!(
                    "cannot delete missing file: {display}"
                )));
            }
            let old = fs::read_to_string(&absolute)?;
            Ok(PlannedOp {
                display_path: display.clone(),
                absolute,
                move_to: None,
                move_absolute: None,
                kind: ChangeKind::Delete,
                old_text: Some(old.clone()),
                new_text: None,
                diff: unified_diff(&display, &old, ""),
            })
        }
        Hunk::Update {
            path,
            move_path,
            chunks,
        } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            if !absolute.is_file() {
                return Err(ToolError::new(format!(
                    "cannot update missing file: {display}"
                )));
            }
            let original = fs::read_to_string(&absolute)?;
            let new_contents = apply_chunks_to_contents(&original, &display, &chunks)?;
            let (move_to, move_absolute) = if let Some(dest) = move_path {
                let dest_display = path_to_display(&dest)?;
                let dest_abs = resolve_write(ctx, &dest_display)?;
                (Some(dest_display), Some(dest_abs))
            } else {
                (None, None)
            };
            let diff_path = move_to.clone().unwrap_or_else(|| display.clone());
            Ok(PlannedOp {
                display_path: display,
                absolute,
                move_to,
                move_absolute,
                kind: ChangeKind::Modify,
                old_text: Some(original.clone()),
                new_text: Some(new_contents.clone()),
                diff: unified_diff(&diff_path, &original, &new_contents),
            })
        }
    }
}

fn apply_op(op: &PlannedOp) -> Result<(), ToolError> {
    match op.kind {
        ChangeKind::Delete => {
            fs::remove_file(&op.absolute)?;
            Ok(())
        }
        ChangeKind::Create | ChangeKind::Modify => {
            let target = op.move_absolute.as_ref().unwrap_or(&op.absolute);
            let content = op.new_text.clone().unwrap_or_default();
            atomic_write(target, content)?;
            if op.move_absolute.is_some() && &op.absolute != target && op.absolute.exists() {
                fs::remove_file(&op.absolute)?;
            }
            Ok(())
        }
    }
}

fn execute_plan(plan: &PatchPlan) -> Result<(), ToolError> {
    let snapshots = snapshot_plan_files(plan)?;
    for op in &plan.ops {
        if let Err(error) = apply_op(op) {
            return match restore_plan_files(&snapshots) {
                Ok(()) => Err(error),
                Err(rollback) => Err(ToolError::new(format!(
                    "{error}; rollback also failed: {rollback}"
                ))),
            };
        }
    }
    Ok(())
}

fn snapshot_plan_files(plan: &PatchPlan) -> Result<HashMap<PathBuf, Option<Vec<u8>>>, ToolError> {
    let mut snapshots = HashMap::new();
    for op in &plan.ops {
        for path in std::iter::once(&op.absolute).chain(op.move_absolute.as_ref()) {
            if snapshots.contains_key(path) {
                continue;
            }
            let content = if path.exists() {
                Some(fs::read(path)?)
            } else {
                None
            };
            snapshots.insert(path.clone(), content);
        }
    }
    Ok(snapshots)
}

fn restore_plan_files(snapshots: &HashMap<PathBuf, Option<Vec<u8>>>) -> Result<(), ToolError> {
    let mut failures = Vec::new();
    for (path, content) in snapshots {
        let result = match content {
            Some(bytes) => atomic_write(path, bytes),
            None if path.exists() => fs::remove_file(path),
            None => Ok(()),
        };
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(ToolError::new(failures.join("; ")))
    }
}

fn resolve_write(ctx: &ToolContext, display: &str) -> Result<PathBuf, ToolError> {
    resolve_tool_path(ctx, display, PathAccess::Write, "apply_patch")
}

fn path_to_display(path: &Path) -> Result<String, ToolError> {
    let raw = path.to_string_lossy();
    if raw.trim().is_empty() {
        return Err(ToolError::new("patch path is empty"));
    }
    if path.is_absolute() {
        return Err(ToolError::new(format!(
            "patch paths must be relative, got {}",
            path.display()
        )));
    }
    let normalized = normalize_path(path);
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn apply_chunks_to_contents(
    original: &str,
    path: &str,
    chunks: &[UpdateChunk],
) -> Result<String, ToolError> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_trailing_newline = original.ends_with('\n');
    let normalized = original.replace("\r\n", "\n");
    let mut original_lines: Vec<String> = if normalized.is_empty() {
        Vec::new()
    } else {
        normalized.split('\n').map(str::to_string).collect()
    };
    if had_trailing_newline && original_lines.last().is_some_and(String::is_empty) {
        original_lines.pop();
    }
    let replacements = compute_replacements(&original_lines, path, chunks)?;
    let new_lines = apply_replacements(original_lines, &replacements);
    let mut output = new_lines.join(newline);
    if had_trailing_newline {
        output.push_str(newline);
    }
    Ok(output)
}

fn compute_replacements(
    original_lines: &[String],
    path: &str,
    chunks: &[UpdateChunk],
) -> Result<Vec<(usize, usize, Vec<String>)>, ToolError> {
    let mut replacements = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(ctx_line) = &chunk.change_context {
            if let Some(idx) = seek_unique(
                original_lines,
                std::slice::from_ref(ctx_line),
                line_index,
                false,
                path,
            )? {
                line_index = idx + 1;
            } else {
                return Err(ToolError::new(format!(
                    "Failed to find context '{ctx_line}' in {path}"
                )));
            }
        }

        if chunk.old_lines.is_empty() {
            let insertion_idx = if original_lines.last().is_some_and(String::is_empty) {
                original_lines.len() - 1
            } else {
                original_lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern: &[String] = &chunk.old_lines;
        let mut found = seek_unique(
            original_lines,
            pattern,
            line_index,
            chunk.is_end_of_file,
            path,
        )?;
        let mut new_slice: &[String] = &chunk.new_lines;

        if found.is_none() && pattern.last().is_some_and(String::is_empty) {
            pattern = &pattern[..pattern.len() - 1];
            if new_slice.last().is_some_and(String::is_empty) {
                new_slice = &new_slice[..new_slice.len() - 1];
            }
            found = seek_unique(
                original_lines,
                pattern,
                line_index,
                chunk.is_end_of_file,
                path,
            )?;
        }

        if let Some(start_idx) = found {
            replacements.push((start_idx, pattern.len(), new_slice.to_vec()));
            line_index = start_idx + pattern.len();
        } else {
            return Err(ToolError::new(format!(
                "Failed to find expected lines in {path}:\n{}",
                chunk.old_lines.join("\n")
            )));
        }
    }

    replacements.sort_by_key(|(index, _, _)| *index);
    Ok(replacements)
}

fn seek_unique(
    lines: &[String],
    pattern: &[String],
    start: usize,
    eof: bool,
    path: &str,
) -> Result<Option<usize>, ToolError> {
    seek_sequence(lines, pattern, start, eof).map_err(|count| {
        ToolError::new(format!(
            "patch context is ambiguous in {path}: matched {count} locations; include more unique context"
        ))
    })
}

fn apply_replacements(
    mut lines: Vec<String>,
    replacements: &[(usize, usize, Vec<String>)],
) -> Vec<String> {
    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        let start_idx = *start_idx;
        for _ in 0..*old_len {
            if start_idx < lines.len() {
                lines.remove(start_idx);
            }
        }
        for (offset, line) in new_segment.iter().enumerate() {
            lines.insert(start_idx + offset, line.clone());
        }
    }
    lines
}

impl From<ParseError> for ToolError {
    fn from(value: ParseError) -> Self {
        ToolError::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
    use std::sync::{Arc, Mutex};

    struct NoopBus;
    impl EventBus for NoopBus {
        fn emit(&self, _event: BusEvent) {}
    }

    fn make_ctx(root: PathBuf) -> (ToolContext, PathBuf) {
        let db_path =
            std::env::temp_dir().join(format!("peek-apply-patch-{}.db", uuid::Uuid::new_v4()));
        let ctx = ToolContext {
            workspace_root: root,
            request_context: RequestContext::default(),
            session_id: uuid::Uuid::new_v4().to_string(),
            assistant_message_id: "m".into(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NoopBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        crate::core::tools::tool_approval::shared_tool_approval_store().set_session_mode(
            &ctx.session_id,
            Some(crate::models::settings::ToolApprovalMode::Auto),
        );
        (ctx, db_path)
    }

    #[test]
    fn parses_and_applies_add_update_delete() {
        let root = std::env::temp_dir().join(format!("peek-patch-ws-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("old.txt"), "hello\nworld\n").unwrap();
        fs::write(root.join("gone.txt"), "bye\n").unwrap();

        let patch = r#"*** Begin Patch
*** Add File: new.txt
+alpha
+beta
*** Update File: old.txt
@@
-hello
+HELLO
*** Delete File: gone.txt
*** End Patch"#;

        let (ctx, db) = make_ctx(root.clone());
        let result = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .expect("apply");
        assert!(result.contains("applied 3"));
        assert_eq!(
            fs::read_to_string(root.join("new.txt")).unwrap(),
            "alpha\nbeta\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("old.txt")).unwrap(),
            "HELLO\nworld\n"
        );
        assert!(!root.join("gone.txt").exists());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn preview_includes_every_file_in_a_multi_file_patch() {
        let root =
            std::env::temp_dir().join(format!("peek-patch-preview-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("one.txt"), "old one\n").unwrap();
        fs::write(root.join("two.txt"), "old two\n").unwrap();
        let patch = r#"*** Begin Patch
*** Update File: one.txt
@@
-old one
+new one
*** Update File: two.txt
@@
-old two
+new two
*** End Patch"#;

        let (ctx, db) = make_ctx(root.clone());
        let preview = ApplyPatchTool
            .preview(&ctx, &json!({ "input": patch }))
            .expect("preview")
            .expect("file preview");

        assert!(preview.unified_diff.contains("--- a/one.txt"));
        assert!(preview.unified_diff.contains("--- a/two.txt"));
        assert_eq!(preview.affected_paths, vec!["one.txt", "two.txt"]);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn update_preserves_crlf_and_trailing_newline() {
        let root = std::env::temp_dir().join(format!("peek-patch-crlf-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), b"one\r\ntwo\r\nthree\r\n").unwrap();
        let patch = "*** Begin Patch\n*** Update File: file.txt\n@@\n-two\n+TWO\n*** End Patch";
        let (ctx, db) = make_ctx(root.clone());

        ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap();

        let output = fs::read(root.join("file.txt")).unwrap();
        assert_eq!(output, b"one\r\nTWO\r\nthree\r\n");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn update_preserves_missing_trailing_newline() {
        let root = std::env::temp_dir().join(format!("peek-patch-eof-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), "one\ntwo").unwrap();
        let patch = "*** Begin Patch\n*** Update File: file.txt\n@@\n-two\n+TWO\n*** End Patch";
        let (ctx, db) = make_ctx(root.clone());

        ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap();

        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "one\nTWO"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn rejects_ambiguous_patch_context() {
        let root =
            std::env::temp_dir().join(format!("peek-patch-ambiguous-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), "same\nmiddle\nsame\n").unwrap();
        let patch =
            "*** Begin Patch\n*** Update File: file.txt\n@@\n-same\n+changed\n*** End Patch";
        let (ctx, db) = make_ctx(root.clone());

        let error = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap_err();

        assert!(error.to_string().contains("ambiguous"));
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "same\nmiddle\nsame\n"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn rejects_multiple_operations_for_the_same_file() {
        let root =
            std::env::temp_dir().join(format!("peek-patch-duplicate-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), "one\ntwo\n").unwrap();
        let patch = r#"*** Begin Patch
*** Update File: file.txt
@@
-one
+ONE
*** Update File: file.txt
@@
-two
+TWO
*** End Patch"#;
        let (ctx, db) = make_ctx(root.clone());

        let error = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap_err();

        assert!(error.to_string().contains("multiple operations"));
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "one\ntwo\n"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn add_file_refuses_to_overwrite_existing_content() {
        let root =
            std::env::temp_dir().join(format!("peek-patch-add-existing-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), "keep\n").unwrap();
        let patch = "*** Begin Patch\n*** Add File: file.txt\n+replace\n*** End Patch";
        let (ctx, db) = make_ctx(root.clone());

        let error = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap_err();

        assert!(error.to_string().contains("existing file"));
        assert_eq!(fs::read_to_string(root.join("file.txt")).unwrap(), "keep\n");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn multi_file_failure_rolls_back_already_written_files() {
        let root =
            std::env::temp_dir().join(format!("peek-patch-rollback-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let blocker = root.join("blocker");
        let child = blocker.join("child.txt");
        let plan = PatchPlan {
            ops: vec![
                PlannedOp {
                    display_path: "blocker".into(),
                    absolute: blocker.clone(),
                    move_to: None,
                    move_absolute: None,
                    kind: ChangeKind::Create,
                    old_text: None,
                    new_text: Some("first".into()),
                    diff: String::new(),
                },
                PlannedOp {
                    display_path: "blocker/child.txt".into(),
                    absolute: child.clone(),
                    move_to: None,
                    move_absolute: None,
                    kind: ChangeKind::Create,
                    old_text: None,
                    new_text: Some("second".into()),
                    diff: String::new(),
                },
            ],
        };

        assert!(execute_plan(&plan).is_err());
        assert!(!blocker.exists());
        assert!(!child.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_absolute_paths() {
        let root = std::env::temp_dir().join(format!("peek-patch-abs-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let abs = root.join("abs.txt");
        let patch = format!(
            "*** Begin Patch\n*** Add File: {}\n+x\n*** End Patch",
            abs.display()
        );
        let (ctx, db) = make_ctx(root.clone());
        let err = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("relative")
                || err.to_string().to_lowercase().contains("path")
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }
}
