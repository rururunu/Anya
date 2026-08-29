use std::path::Path;

pub(super) fn validate_root(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("Workspace root must be an absolute path".to_string());
    }
    if !root.is_dir() {
        return Err("Workspace root does not exist or is not a directory".to_string());
    }
    Ok(())
}

pub(super) fn workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| root.display().to_string())
}

pub(super) fn display_name(stored: &str, root: &Path) -> String {
    let trimmed = stored.trim();
    if trimmed.is_empty() {
        workspace_name(root)
    } else {
        trimmed.to_string()
    }
}

pub(super) fn normalize_ide_source(ide: &str) -> Option<String> {
    let ide = ide.trim();
    if ide.is_empty() {
        return None;
    }
    Some(match ide.to_ascii_lowercase().as_str() {
        "visual studio code" | "vs code" | "vscode" => "vscode".to_string(),
        "idea" | "intellij" | "intellij idea" => "idea".to_string(),
        _ => ide.chars().take(64).collect(),
    })
}
