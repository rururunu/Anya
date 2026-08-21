use crate::core::context::models::IDEContext;
use crate::core::context::store;
use crate::core::runtime::RequestContext;
use crate::core::workspace::Workspace;
use std::path::Path;

pub struct ContextResolver;

impl ContextResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(&self) -> RequestContext {
        store::latest_request_context()
    }

    pub fn resolve_environment(
        &self,
        current_workspace: Option<&Workspace>,
        known_workspaces: &[Workspace],
    ) -> RequestContext {
        let captured = store::latest_request_context();
        tracing::debug!(
            active_window = ?captured.active_window.as_deref(),
            selected_files = captured.selected_files.len(),
            "ContextResolver::resolve_environment received captured context"
        );
        self.resolve_request(captured, current_workspace, known_workspaces)
    }

    pub fn resolve_request(
        &self,
        context: RequestContext,
        current_workspace: Option<&Workspace>,
        known_workspaces: &[Workspace],
    ) -> RequestContext {
        let ide_context =
            crate::core::context::providers::ide::latest().or_else(|| context.ide_context.clone());
        self.resolve_request_with_ide(context, current_workspace, known_workspaces, ide_context)
    }

    fn resolve_request_with_ide(
        &self,
        mut context: RequestContext,
        current_workspace: Option<&Workspace>,
        known_workspaces: &[Workspace],
        ide_context: Option<IDEContext>,
    ) -> RequestContext {
        // The IDE push has a multi-minute cache TTL (see local_api.rs), so it
        // can still be "latest" long after the user has switched away from the
        // IDE entirely. Only trust it — for workspace binding, active file,
        // and selection — when the window that's actually in the foreground
        // right now plausibly belongs to that IDE; otherwise a quick-ask
        // triggered from the desktop would silently inherit (and bind chat
        // history to) a stale workspace.
        let ide_context = ide_context
            .filter(|ide| ide_context_matches_active_window(ide, context.active_window.as_deref()));

        let active_file = ide_context
            .as_ref()
            .and_then(|ide| ide.active_file.clone())
            .or_else(|| crate::core::context::provider::active_file_provider::infer(&context));
        let resolved_workspace = ide_context
            .as_ref()
            .and_then(|ide| ide.workspace.clone())
            .map(|root| (workspace_name(&root), root))
            .or_else(|| {
                active_file
                    .as_deref()
                    .and_then(|file| workspace_containing(file, known_workspaces))
                    .map(|workspace| (workspace.name.clone(), workspace.root.clone()))
            })
            .or_else(|| {
                active_file.as_deref().and_then(|file| {
                    crate::core::context::provider::active_file_provider::infer_project_root(file)
                        .map(|root| (workspace_name(&root), root))
                })
            })
            .or_else(|| {
                workspace_from_window(context.active_window.as_deref(), known_workspaces)
                    .map(|workspace| (workspace.name.clone(), workspace.root.clone()))
            })
            .or_else(|| {
                current_workspace.map(|workspace| (workspace.name.clone(), workspace.root.clone()))
            });

        if let Some(selection) = ide_context
            .as_ref()
            .and_then(|ide| ide.selection.as_deref())
            .map(str::trim)
            .filter(|selection| !selection.is_empty())
        {
            context.selection = Some(selection.to_string());
        }
        context.active_file = active_file.map(|path| path.display().to_string());
        if let Some((name, root)) = resolved_workspace {
            context.set_workspace(name, &root);
        }
        context.ide_context = ide_context;
        context
    }
}

/// Whether `active_window` (the real foreground window at capture time)
/// plausibly belongs to the IDE that pushed `ide`. Used to distinguish "you
/// just triggered this from inside your editor" from "you left the editor a
/// while ago and its last-pushed context is still cached".
fn ide_context_matches_active_window(ide: &IDEContext, active_window: Option<&str>) -> bool {
    let Some(active_window) = active_window else {
        return false;
    };
    let window = active_window.to_ascii_lowercase();
    let id = ide.ide.trim().to_ascii_lowercase();
    if id.is_empty() {
        return false;
    }
    if window.contains(&id) {
        return true;
    }
    match id.as_str() {
        "vscode" | "vs code" | "code" => window.contains("visual studio code"),
        "jetbrains" | "idea" | "intellij" => window.contains("intellij"),
        _ => false,
    }
}

fn workspace_from_window<'a>(
    active_window: Option<&str>,
    workspaces: &'a [Workspace],
) -> Option<&'a Workspace> {
    let segments: Vec<String> = active_window?
        .split(" - ")
        .map(|segment| segment.trim().to_ascii_lowercase())
        .collect();
    workspaces
        .iter()
        .filter(|workspace| {
            let name = workspace.name.trim().to_ascii_lowercase();
            !name.is_empty() && segments.iter().any(|segment| segment == &name)
        })
        .max_by_key(|workspace| workspace.name.len())
}

fn workspace_containing<'a>(file: &Path, workspaces: &'a [Workspace]) -> Option<&'a Workspace> {
    workspaces
        .iter()
        .filter(|workspace| path_starts_with(file, &workspace.root))
        .max_by_key(|workspace| workspace.root.components().count())
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    if cfg!(windows) {
        let path = path.to_string_lossy().to_ascii_lowercase();
        let root = root
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase();
        path == root
            || path
                .strip_prefix(&root)
                .is_some_and(|rest| rest.starts_with('\\') || rest.starts_with('/'))
    } else {
        path.starts_with(root)
    }
}

fn workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Workspace")
        .to_string()
}

impl Default for ContextResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn workspace(name: &str, root: &str) -> Workspace {
        Workspace {
            id: root.to_string(),
            name: name.to_string(),
            root: PathBuf::from(root),
            description: None,
            source: None,
            created_at: Utc::now(),
            last_used_at: Utc::now(),
            pinned: false,
            archived: false,
            sort_order: 0,
        }
    }

    #[test]
    fn active_file_workspace_wins_over_current_workspace() {
        let project_a = workspace("Project A", r"C:\code\project-a");
        let temp_b = workspace("Temp B", r"C:\temp\project-b");
        let context = RequestContext {
            selected_files: vec![r"C:\code\project-a\src\main.rs".to_string()],
            ..RequestContext::default()
        };

        let resolved = ContextResolver::new().resolve_request(
            context,
            Some(&temp_b),
            &[project_a.clone(), temp_b.clone()],
        );

        assert_eq!(
            resolved.active_file.as_deref(),
            Some(r"C:\code\project-a\src\main.rs")
        );
        assert_eq!(
            resolved.workspace.unwrap().root,
            project_a.root.display().to_string()
        );
    }

    #[test]
    fn current_workspace_is_used_when_active_file_is_unavailable() {
        let current = workspace("Current", r"C:\code\current");
        let resolved = ContextResolver::new().resolve_request(
            RequestContext::default(),
            Some(&current),
            std::slice::from_ref(&current),
        );
        assert_eq!(resolved.workspace.unwrap().name, "Current");
    }

    #[test]
    fn active_project_title_wins_over_current_workspace() {
        let project_a = workspace("Project A", r"C:\code\project-a");
        let temp_b = workspace("Temp B", r"C:\temp\project-b");
        let context = RequestContext {
            active_window: Some(
                "Code.exe - main.rs - Project A - Visual Studio Code (pid 42)".to_string(),
            ),
            ..RequestContext::default()
        };

        let resolved = ContextResolver::new().resolve_request(
            context,
            Some(&temp_b),
            &[project_a.clone(), temp_b.clone()],
        );

        assert_eq!(
            resolved.workspace.unwrap().root,
            project_a.root.display().to_string()
        );
    }

    #[test]
    fn ide_context_overrides_window_context() {
        let window_workspace = workspace("Project A", r"C:\code\project-a");
        let ide = IDEContext {
            ide: "vscode".to_string(),
            active_file: Some(PathBuf::from(r"C:\code\project-b\src\main.rs")),
            workspace: Some(PathBuf::from(r"C:\code\project-b")),
            language: Some("rust".to_string()),
            selection: Some("fn main() {}".to_string()),
            cursor: None,
        };
        let context = RequestContext {
            active_window: Some("Code.exe - main.rs - Project A - Visual Studio Code".to_string()),
            selection: Some("clipboard selection".to_string()),
            ..RequestContext::default()
        };

        let resolved = ContextResolver::new().resolve_request_with_ide(
            context,
            Some(&window_workspace),
            std::slice::from_ref(&window_workspace),
            Some(ide),
        );

        assert_eq!(
            resolved.active_file.as_deref(),
            Some(r"C:\code\project-b\src\main.rs")
        );
        assert_eq!(
            resolved
                .workspace
                .as_ref()
                .map(|workspace| workspace.root.as_str()),
            Some(r"C:\code\project-b")
        );
        assert_eq!(resolved.selection.as_deref(), Some("fn main() {}"));
        assert_eq!(
            resolved.ide_context.as_ref().map(|ide| ide.ide.as_str()),
            Some("vscode")
        );
    }

    #[test]
    fn missing_ide_context_preserves_environment_resolution() {
        let current = workspace("Current", r"C:\code\current");
        let context = RequestContext {
            active_window: Some("Code.exe - notes.txt".to_string()),
            selection: Some("clipboard selection".to_string()),
            ..RequestContext::default()
        };

        let resolved = ContextResolver::new().resolve_request_with_ide(
            context,
            Some(&current),
            std::slice::from_ref(&current),
            None,
        );

        assert_eq!(
            resolved.workspace.unwrap().root,
            current.root.display().to_string()
        );
        assert_eq!(resolved.selection.as_deref(), Some("clipboard selection"));
        assert!(resolved.ide_context.is_none());
    }

    #[test]
    fn stale_ide_context_is_ignored_when_ide_is_not_foreground() {
        // The IDE-pushed context is cached for minutes (see local_api.rs's
        // IDE_CONTEXT_TTL), so it can still be "latest" long after the user
        // switched away from the editor. If the foreground window at capture
        // time is unrelated (e.g. Explorer/desktop), it must not be used to
        // silently bind a quick-ask to that stale workspace.
        let ide_workspace = workspace("Project B", r"C:\code\project-b");
        let ide = IDEContext {
            ide: "vscode".to_string(),
            active_file: Some(PathBuf::from(r"C:\code\project-b\src\main.rs")),
            workspace: Some(PathBuf::from(r"C:\code\project-b")),
            language: Some("rust".to_string()),
            selection: Some("fn main() {}".to_string()),
            cursor: None,
        };
        let context = RequestContext {
            active_window: Some("Explorer.EXE - Program Manager".to_string()),
            ..RequestContext::default()
        };

        let resolved = ContextResolver::new().resolve_request_with_ide(
            context,
            None,
            std::slice::from_ref(&ide_workspace),
            Some(ide),
        );

        assert!(resolved.ide_context.is_none());
        assert!(resolved.workspace.is_none());
        assert!(resolved.active_file.is_none());
    }
}
