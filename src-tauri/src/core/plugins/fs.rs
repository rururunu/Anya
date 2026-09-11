//! Copy user-picked files into a plugin folder so `anya-plugin://` can serve them.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::manifest::plugin_dir;
use super::PLUGIN_SCHEME;
use crate::core::tools::error::ToolError;

/// Max size of a file copied into `data/picked/` (256 MiB).
pub const MAX_PICKED_FILE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFsPickOptions {
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub directory: bool,
    #[serde(default)]
    pub filters: Vec<PluginFsPickFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginFsPickFilter {
    pub name: String,
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedFile {
    /// Original absolute path the user selected.
    pub path: String,
    /// Original basename, for labels.
    pub name: String,
    /// `anya-plugin://` URL of the snapshot copied into the plugin folder.
    /// Empty for directory picks (folders are not copied).
    pub url: String,
}

/// Snapshot a user-selected file into `data/picked/` and return path + plugin URL.
pub fn import_user_file(plugin_id: &str, source: &Path) -> Result<PickedFile, ToolError> {
    if !source.is_file() {
        return Err(ToolError::new("picked path is not a file"));
    }
    let len = fs::metadata(source)
        .map_err(|e| ToolError::new(format!("stat picked file: {e}")))?
        .len();
    reject_if_too_large(len)?;
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "file".into());
    let dest_name = dest_file_name(source, &name);
    let root = plugin_dir(plugin_id)?;
    let dest_dir = root.join("data").join("picked");
    fs::create_dir_all(&dest_dir).map_err(|e| ToolError::new(format!("create picked dir: {e}")))?;
    let dest = dest_dir.join(&dest_name);
    fs::copy(source, &dest).map_err(|e| ToolError::new(format!("copy picked file: {e}")))?;
    Ok(PickedFile {
        path: source.to_string_lossy().into_owned(),
        name,
        url: format!("{PLUGIN_SCHEME}://localhost/{plugin_id}/data/picked/{dest_name}"),
    })
}

/// Directory picks are path-only; `url` stays empty (not an asset source).
pub fn folder_as_picked(source: &Path) -> PickedFile {
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| source.to_string_lossy().into_owned());
    PickedFile {
        path: source.to_string_lossy().into_owned(),
        name,
        url: String::new(),
    }
}

pub(crate) fn reject_if_too_large(len: u64) -> Result<(), ToolError> {
    if len > MAX_PICKED_FILE_BYTES {
        return Err(ToolError::new(format!(
            "picked file is {len} bytes; max is {MAX_PICKED_FILE_BYTES}"
        )));
    }
    Ok(())
}

fn dest_file_name(source: &Path, original_name: &str) -> String {
    let digest = Sha256::digest(source.to_string_lossy().as_bytes());
    let hex: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    format!("{hex}-{}", sanitize_file_name(original_name))
}

pub(crate) fn sanitize_file_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if out.len() >= 80 {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('.');
    if trimmed.is_empty() {
        "file".into()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(sanitize_file_name("../../../x.txt"), "_.._.._x.txt");
        assert_eq!(sanitize_file_name("..."), "file");
    }

    #[test]
    fn reject_oversize() {
        assert!(reject_if_too_large(MAX_PICKED_FILE_BYTES).is_ok());
        assert!(reject_if_too_large(MAX_PICKED_FILE_BYTES + 1).is_err());
    }

    #[test]
    fn folder_pick_has_empty_url() {
        let picked = folder_as_picked(&Path::new("videos-root").join("Videos"));
        assert_eq!(picked.name, "Videos");
        assert!(picked.url.is_empty());
    }

    #[test]
    fn import_user_file_copies_and_builds_plugin_url() {
        let id = format!("picktest{}", Uuid::new_v4().simple());
        let dir = plugin_dir(&id).unwrap();
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let src = std::env::temp_dir().join(format!("{id}-src.txt"));
        fs::write(&src, b"hello-pick").unwrap();
        let picked = import_user_file(&id, &src).unwrap();
        assert_eq!(picked.path, src.to_string_lossy());
        assert_eq!(picked.name, format!("{id}-src.txt"));
        assert!(picked
            .url
            .starts_with(&format!("anya-plugin://localhost/{id}/data/picked/")));
        assert!(picked.url.ends_with(&format!("-{id}-src.txt")));
        let copied = fs::read_dir(dir.join("data").join("picked"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(&copied).unwrap(), b"hello-pick");
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_file(&src);
    }
}
