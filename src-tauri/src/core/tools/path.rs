use std::path::{Component, Path, PathBuf};

use super::context::ToolContext;
use super::error::ToolError;
use super::path_permission::PathAccess;
use crate::models::settings::ToolApprovalMode;

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

    deny_user_plugin_hunt(&normalized)?;

    let approval_mode =
        super::tool_approval::shared_tool_approval_store().mode_for_session(ctx.root_session_id());
    let pass_all = approval_mode == ToolApprovalMode::AlwaysAllow;

    // Outside-workspace writes: denied unless settings opt-in, or AlwaysAllow (最高级放行).
    if access == PathAccess::Write && !pass_all && !super::sandbox::allow_outside_workspace_writes()
    {
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

    // AlwaysAllow: auto-pass path prompts (including outside workspace).
    // Auto: workspace tools are already approved; outside paths still ask.
    if pass_all {
        ctx.path_permission_store
            .grant_always(ctx.root_session_id(), &normalized, access);
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

const PLUGIN_HUNT_HINT: &str = "User plugins live outside the workspace. Do not read_file/list_folder/find_files/Grep AppData, LocalAppData, $HOME, or guessed plugin.json paths — that waits up to 10 minutes on a path-permission prompt (looks stuck, not slow I/O). Call manage_plugin action=list, then list_files/get_file. Those read actions are available in plan mode. Never guess %APPDATA% plugin paths.";

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn is_under(path: &Path, root: &Path) -> bool {
    let path = normalize_path(path);
    let root = normalize_path(root);
    path == root || path.starts_with(&root)
}

/// True for Anya's user plugin/data dirs, guessed AppData plugin paths, or listing the whole profile.
pub fn is_user_plugin_hunt(path: &Path) -> bool {
    let n = normalize_path(path);
    if is_under(&n, &super::memory::plugins_dir()) {
        return true;
    }
    let key = path_key(&n);
    if key.contains(r"\anya\plugins") || key.contains(r"\com.anya.app\plugins") {
        return true;
    }
    is_profile_root(&n)
}

fn is_profile_root(path: &Path) -> bool {
    let n = normalize_path(path);
    for key in ["USERPROFILE", "HOME", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(value) = std::env::var(key) {
            let root = normalize_path(Path::new(&value));
            if !value.trim().is_empty() && n == root {
                return true;
            }
        }
    }
    false
}

fn deny_user_plugin_hunt(path: &Path) -> Result<(), ToolError> {
    if is_user_plugin_hunt(path) {
        return Err(ToolError::new(format!(
            "{PLUGIN_HUNT_HINT} Blocked path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_allow_auto_grants_outside_workspace_reads() {
        use crate::core::chat::conversation_manager::ConversationManager;
        use crate::core::event::{BusEvent, EventBus};
        use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
        use crate::core::tools::tool_approval::shared_tool_approval_store;
        use crate::models::settings::ToolApprovalMode;
        use std::sync::{atomic::AtomicBool, Arc, Mutex};

        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        shared_tool_approval_store().set_session_mode("s-always", Some(ToolApprovalMode::AlwaysAllow));
        let db = std::env::temp_dir().join(format!("peek-path-allow-{}.db", uuid::Uuid::new_v4()));
        let ws = std::env::temp_dir().join(format!("peek-ws-allow-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&ws);
        let outside = std::env::temp_dir().join(format!("peek-outside-read-{}.txt", uuid::Uuid::new_v4()));
        let _ = std::fs::write(&outside, "ok");
        let ctx = ToolContext {
            workspace_root: ws,
            request_context: Default::default(),
            session_id: "s-always".into(),
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
        let resolved = resolve_tool_path(
            &ctx,
            outside.to_str().unwrap_or("C:\\outside-read.txt"),
            PathAccess::Read,
            "read_file",
        )
        .expect("AlwaysAllow should skip path permission prompt");
        assert_eq!(normalize_path(&resolved), normalize_path(&outside));
        shared_tool_approval_store().set_session_mode("s-always", None);
    }

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

    #[test]
    fn always_allow_permits_outside_workspace_writes() {
        use crate::core::chat::conversation_manager::ConversationManager;
        use crate::core::event::{BusEvent, EventBus};
        use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
        use crate::core::tools::tool_approval::shared_tool_approval_store;
        use crate::models::settings::ToolApprovalMode;
        use std::sync::{atomic::AtomicBool, Arc, Mutex};

        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        crate::core::tools::sandbox::configure(false, false, 120, 120);
        shared_tool_approval_store()
            .set_session_mode("s-always-write", Some(ToolApprovalMode::AlwaysAllow));
        let db = std::env::temp_dir().join(format!("peek-path-aw-{}.db", uuid::Uuid::new_v4()));
        let ws = std::env::temp_dir().join(format!("peek-ws-aw-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&ws);
        let outside = std::env::temp_dir().join(format!("peek-outside-aw-{}.txt", uuid::Uuid::new_v4()));
        let ctx = ToolContext {
            workspace_root: ws,
            request_context: Default::default(),
            session_id: "s-always-write".into(),
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
        let resolved = resolve_tool_path(
            &ctx,
            outside.to_str().unwrap_or("C:\\outside-aw.txt"),
            PathAccess::Write,
            "write_file",
        )
        .expect("AlwaysAllow should permit outside-workspace writes");
        assert_eq!(normalize_path(&resolved), normalize_path(&outside));
        shared_tool_approval_store().set_session_mode("s-always-write", None);
    }

    #[test]
    fn auto_still_denies_outside_workspace_writes_by_default() {
        use crate::core::chat::conversation_manager::ConversationManager;
        use crate::core::event::{BusEvent, EventBus};
        use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
        use crate::core::tools::tool_approval::shared_tool_approval_store;
        use crate::models::settings::ToolApprovalMode;
        use std::sync::{atomic::AtomicBool, Arc, Mutex};

        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        crate::core::tools::sandbox::configure(false, false, 120, 120);
        shared_tool_approval_store().set_session_mode("s-auto-write", Some(ToolApprovalMode::Auto));
        let db = std::env::temp_dir().join(format!("peek-path-auto-{}.db", uuid::Uuid::new_v4()));
        let ws = std::env::temp_dir().join(format!("peek-ws-auto-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&ws);
        let outside = std::env::temp_dir().join(format!("peek-outside-auto-{}.txt", uuid::Uuid::new_v4()));
        let ctx = ToolContext {
            workspace_root: ws,
            request_context: Default::default(),
            session_id: "s-auto-write".into(),
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
        let err = resolve_tool_path(
            &ctx,
            outside.to_str().unwrap_or("C:\\outside-auto.txt"),
            PathAccess::Write,
            "write_file",
        )
        .unwrap_err();
        assert!(err.message.contains("write outside workspace denied"));
        shared_tool_approval_store().set_session_mode("s-auto-write", None);
    }

    #[test]
    fn guessed_appdata_plugin_json_is_rejected_without_permission_wait() {
        let path = PathBuf::from(
            r"C:\Users\demo\AppData\Roaming\Anya\plugins\video-background\plugin.json",
        );
        assert!(is_user_plugin_hunt(&path));
        let err = deny_user_plugin_hunt(&path).unwrap_err();
        assert!(err.message.contains("manage_plugin"));
        assert!(err.message.contains("Blocked path"));
    }

    #[test]
    fn workspace_plugin_examples_are_not_a_hunt() {
        let ws = PathBuf::from(r"C:\code\AltAltAi");
        let example = ws.join("src-tauri").join("plugins").join("terminal");
        assert!(!is_user_plugin_hunt(&example));
        let key = path_key(&example);
        assert!(!key.contains(r"\anya\plugins"));
    }
}
