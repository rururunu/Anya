use std::path::{Component, Path, PathBuf};

use super::context::ToolContext;
use super::error::ToolError;
use super::path_permission::PathAccess;

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

pub fn resolve_path_candidate(workspace: &Path, raw: &str) -> Result<PathBuf, ToolError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ToolError::new("path is required"));
    }

    let candidate = PathBuf::from(trimmed);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(candidate)
    };

    Ok(normalize_path(&resolved))
}

#[allow(dead_code)]
pub fn resolve_in_workspace(workspace: &Path, raw: &str) -> Result<PathBuf, ToolError> {
    let normalized = resolve_path_candidate(workspace, raw)?;
    let workspace = normalize_path(workspace);

    if !normalized.starts_with(&workspace) {
        return Err(ToolError::new(format!(
            "path escapes workspace: {}",
            normalized.display()
        )));
    }

    Ok(normalized)
}

pub fn resolve_tool_path(
    ctx: &ToolContext,
    raw: &str,
    access: PathAccess,
    tool_name: &str,
) -> Result<PathBuf, ToolError> {
    let normalized = resolve_path_candidate(&ctx.workspace_root, raw)?;
    let workspace = normalize_path(&ctx.workspace_root);

    // Workspace-local paths do not need a path-permission prompt.
    // Mutating tools are already gated by tool_approval (ask / auto / alwaysAllow).
    // Asking again here produced a second, redundant "询问" UI for writes.
    if normalized.starts_with(&workspace) {
        return Ok(normalized);
    }

    // Outside-workspace writes are denied by default (sandbox). Opt-in via settings.
    if access == PathAccess::Write && !super::sandbox::allow_outside_workspace_writes() {
        return Err(ToolError::new(format!(
            "write outside workspace denied (enable allowOutsideWorkspaceWrites to permit after approval): {}",
            normalized.display()
        )));
    }

    if ctx
        .path_permission_store
        .is_granted(ctx.root_session_id(), &normalized, access)
    {
        return Ok(normalized);
    }

    ctx.path_permission_store.request_and_grant(
        ctx.root_session_id(),
        &ctx.event_bus,
        normalized,
        access,
        tool_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_escape() {
        let ws = PathBuf::from("/workspace/project");
        let err = resolve_in_workspace(&ws, "../outside.txt").unwrap_err();
        assert!(err.message.contains("escapes workspace"));
    }

    #[test]
    fn outside_workspace_writes_denied_by_default() {
        use crate::core::chat::conversation_manager::ConversationManager;
        use crate::core::event::{BusEvent, EventBus};
        use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
        use std::sync::{atomic::AtomicBool, Arc, Mutex};

        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        crate::core::tools::sandbox::configure(false, false, 120, 120);
        let db = std::env::temp_dir().join(format!("peek-path-{}.db", uuid::Uuid::new_v4()));
        let ws = std::env::temp_dir().join(format!("peek-ws-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&ws);
        let ctx = ToolContext {
            workspace_root: ws,
            request_context: Default::default(),
            session_id: "s".into(),
            assistant_message_id: "a".into(),
            conversation: Arc::new(ConversationManager::new(db)),
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
        let outside = std::env::temp_dir().join("peek-outside-write.txt");
        let err = resolve_tool_path(
            &ctx,
            outside.to_str().unwrap_or("C:\\outside.txt"),
            PathAccess::Write,
            "write_file",
        )
        .unwrap_err();
        assert!(err.message.contains("write outside workspace denied"));
    }
}
