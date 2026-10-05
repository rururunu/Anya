use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::message::ChatMessage;
use crate::core::context::models::IDEContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceContext {
    pub name: String,
    pub root: String,
}

/// Windows 上下文 — 由 ContextResolver 填充。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    pub selection: Option<String>,
    pub selected_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_images: Vec<String>,
    pub active_window: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_file: Option<String>,
    pub workspace: Option<WorkspaceContext>,
    pub clipboard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_shell_execution: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ide_context: Option<IDEContext>,
}

impl RequestContext {
    pub fn set_workspace(&mut self, name: String, root: &Path) {
        self.selected_files = self
            .selected_files
            .iter()
            .filter_map(|selected| relative_selected_file(root, Path::new(selected)))
            .collect();
        self.workspace = Some(WorkspaceContext {
            name,
            root: root.display().to_string(),
        });
    }
}

fn relative_selected_file(root: &Path, selected: &Path) -> Option<String> {
    let normalized_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let normalized_selected =
        std::fs::canonicalize(selected).unwrap_or_else(|_| selected.to_path_buf());
    let display_path = normalized_selected
        .strip_prefix(&normalized_root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(PathBuf::from)
        .or_else(|| normalized_selected.file_name().map(PathBuf::from))?;
    Some(display_path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_makes_selected_files_relative_and_hides_external_roots() {
        let mut context = RequestContext {
            selected_files: vec![
                r"D:\Code\Peek\src-tauri\src\main.rs".to_string(),
                r"C:\Temp\notes.txt".to_string(),
            ],
            ..RequestContext::default()
        };

        context.set_workspace("Peek".to_string(), Path::new(r"D:\Code\Peek"));

        assert_eq!(
            context.selected_files,
            vec!["src-tauri/src/main.rs", "notes.txt"]
        );
        assert_eq!(context.workspace.as_ref().unwrap().name, "Peek");
    }
}

/// 统一 AI 请求 — Provider 各自内部转换，不暴露 DeepSeekRequest 等。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub request_id: String,
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub context: RequestContext,
    pub provider: Option<String>,
    pub stream: bool,
    #[serde(default = "empty_tools", skip_serializing_if = "tools_is_empty")]
    pub tools: std::sync::Arc<[serde_json::Value]>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

fn empty_tools() -> std::sync::Arc<[serde_json::Value]> {
    std::sync::Arc::from([])
}

fn tools_is_empty(tools: &std::sync::Arc<[serde_json::Value]>) -> bool {
    tools.is_empty()
}
