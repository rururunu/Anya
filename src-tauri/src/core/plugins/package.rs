//! Zip/folder pack of a user plugin (sources only; no generated `ui/.anya` / `data` / `bin`).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::bundled::OFFICIAL_PLUGIN_IDS;
use super::manifest::{load_manifest, normalize_rel, plugin_dir, sanitize_plugin_id};
use super::MAX_PLUGIN_FILE_BYTES;
use crate::core::tools::error::ToolError;

const MAX_PACKAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_FILES: usize = 200;
const SKIP_DIR_NAMES: &[&str] = &[".anya", "node_modules", "data", "bin"];

/// True when `id` is a bundled official plugin that launch will overwrite.
pub fn is_official_plugin_id(id: &str) -> bool {
    OFFICIAL_PLUGIN_IDS.iter().any(|known| *known == id)
}

/// Writes `{id}-{version}.anya-plugin.zip` (or `dest` when it already ends in `.zip`).
pub fn export_plugin_zip(id: &str, dest: &Path) -> Result<PathBuf, ToolError> {
    let id = sanitize_plugin_id(id)?;
    let manifest = load_manifest(&id)?;
    let src = plugin_dir(&id)?;
    if !src.join("plugin.json").is_file() {
        return Err(ToolError::new(format!("plugin `{id}` not found")));
    }
    let dest = resolve_zip_dest(&id, &manifest.version, dest)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ToolError::new(format!("create export dir: {e}")))?;
    }
    pack_folder_to_zip(&src, &dest)?;
    Ok(dest)
}

/// Reads `plugin.json` id from a zip or folder without installing.
pub fn peek_plugin_id(path: &Path) -> Result<String, ToolError> {
    let (staging, owned) = unpack_source(path)?;
    let result =
        find_plugin_root(&staging).and_then(|root| read_manifest_id(&root.join("plugin.json")));
    if owned {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// Installs a zip or a folder that contains `plugin.json`. Does not enable the plugin.
pub fn import_plugin_from_path(path: &Path, overwrite: bool) -> Result<String, ToolError> {
    let (staging, owned) = unpack_source(path)?;
    let result = install_from_staging(&staging, overwrite);
    if owned {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn install_from_staging(staging: &Path, overwrite: bool) -> Result<String, ToolError> {
    let root = find_plugin_root(staging)?;
    let id = read_manifest_id(&root.join("plugin.json"))?;
    if is_official_plugin_id(&id) {
        return Err(ToolError::new(format!(
            "cannot import over official plugin `{id}`"
        )));
    }
    let dest = plugin_dir(&id)?;
    if dest.join("plugin.json").is_file() {
        if !overwrite {
            return Err(ToolError::new(format!(
                "plugin `{id}` already exists; pass overwrite to replace it"
            )));
        }
        fs::remove_dir_all(&dest).map_err(|e| ToolError::new(format!("replace plugin: {e}")))?;
    }
    copy_plugin_sources(&root, &dest)?;
    let _ = load_manifest(&id)?;
    Ok(id)
}

fn resolve_zip_dest(id: &str, version: &str, dest: &Path) -> Result<PathBuf, ToolError> {
    let name = format!("{id}-{version}.anya-plugin.zip");
    let is_zip = dest
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
    if is_zip {
        return Ok(dest.to_path_buf());
    }
    if dest.exists() && !dest.is_dir() {
        return Err(ToolError::new(
            "export dest must be a folder or a .zip file",
        ));
    }
    fs::create_dir_all(dest).map_err(|e| ToolError::new(format!("create export dir: {e}")))?;
    Ok(dest.join(name))
}

fn skip_dir(name: &str) -> bool {
    SKIP_DIR_NAMES.iter().any(|skip| *skip == name)
}

fn collect_source_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ToolError> {
    if out.len() >= MAX_PACKAGE_FILES {
        return Err(ToolError::new("plugin package has too many files"));
    }
    for entry in fs::read_dir(dir).map_err(|e| ToolError::new(e.to_string()))? {
        let entry = entry.map_err(|e| ToolError::new(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if skip_dir(&name_str) {
            continue;
        }
        if path.is_dir() {
            collect_source_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel = rel.to_string_lossy().replace('\\', "/");
            normalize_rel(&rel)?;
            out.push(path);
        }
    }
    Ok(())
}

fn pack_folder_to_zip(src: &Path, dest: &Path) -> Result<(), ToolError> {
    let mut files = Vec::new();
    collect_source_files(src, src, &mut files)?;
    if files.is_empty() {
        return Err(ToolError::new("plugin folder has no packable files"));
    }
    let file = File::create(dest).map_err(|e| ToolError::new(format!("create zip: {e}")))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut total = 0u64;
    for path in &files {
        let rel = path
            .strip_prefix(src)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(path).map_err(|e| ToolError::new(format!("read {rel}: {e}")))?;
        if bytes.len() > MAX_PLUGIN_FILE_BYTES {
            return Err(ToolError::new(format!(
                "{rel} is too large ({} bytes; max {MAX_PLUGIN_FILE_BYTES})",
                bytes.len()
            )));
        }
        total += bytes.len() as u64;
        if total > MAX_PACKAGE_BYTES {
            return Err(ToolError::new("plugin package exceeds 32MB"));
        }
        zip.start_file(&rel, opts)
            .map_err(|e| ToolError::new(format!("zip {rel}: {e}")))?;
        zip.write_all(&bytes)
            .map_err(|e| ToolError::new(format!("zip write {rel}: {e}")))?;
    }
    zip.finish()
        .map_err(|e| ToolError::new(format!("finish zip: {e}")))?;
    Ok(())
}

fn unpack_source(path: &Path) -> Result<(PathBuf, bool), ToolError> {
    if path.is_dir() {
        return Ok((path.to_path_buf(), false));
    }
    if !path.is_file() {
        return Err(ToolError::new("import path is not a file or folder"));
    }
    let staging =
        std::env::temp_dir().join(format!("anya-import-{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&staging).map_err(|e| ToolError::new(format!("staging: {e}")))?;
    extract_zip(path, &staging)?;
    Ok((staging, true))
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), ToolError> {
    let file = File::open(zip_path).map_err(|e| ToolError::new(format!("open zip: {e}")))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ToolError::new(format!("read zip: {e}")))?;
    if archive.len() > MAX_PACKAGE_FILES {
        return Err(ToolError::new("plugin package has too many files"));
    }
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ToolError::new(format!("zip entry: {e}")))?;
        let Some(enclosed) = entry.enclosed_name() else {
            return Err(ToolError::new("zip contains an unsafe path"));
        };
        if entry.is_dir() {
            continue;
        }
        let rel = enclosed.to_string_lossy().replace('\\', "/");
        if rel.split('/').any(skip_dir) {
            continue;
        }
        normalize_rel(&rel)?;
        let size = entry.size();
        if size > MAX_PLUGIN_FILE_BYTES as u64 {
            return Err(ToolError::new(format!("{rel} is too large")));
        }
        total += size;
        if total > MAX_PACKAGE_BYTES {
            return Err(ToolError::new("plugin package exceeds 32MB"));
        }
        let out = dest.join(&enclosed);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ToolError::new(format!("unzip {rel}: {e}")))?;
        fs::write(&out, buf).map_err(|e| ToolError::new(format!("write {rel}: {e}")))?;
    }
    Ok(())
}

fn find_plugin_root(staging: &Path) -> Result<PathBuf, ToolError> {
    if staging.join("plugin.json").is_file() {
        return Ok(staging.to_path_buf());
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(staging).map_err(|e| ToolError::new(e.to_string()))? {
        let entry = entry.map_err(|e| ToolError::new(e.to_string()))?;
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }
    if dirs.len() == 1 && dirs[0].join("plugin.json").is_file() {
        return Ok(dirs[0].clone());
    }
    Err(ToolError::new(
        "package must contain plugin.json at the zip root or in a single top-level folder",
    ))
}

fn read_manifest_id(path: &Path) -> Result<String, ToolError> {
    let raw =
        fs::read_to_string(path).map_err(|e| ToolError::new(format!("read plugin.json: {e}")))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| ToolError::new(format!("invalid plugin.json: {e}")))?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    sanitize_plugin_id(id)
}

fn copy_plugin_sources(src: &Path, dest: &Path) -> Result<(), ToolError> {
    let mut files = Vec::new();
    collect_source_files(src, src, &mut files)?;
    fs::create_dir_all(dest).map_err(|e| ToolError::new(e.to_string()))?;
    for path in files {
        let rel = path
            .strip_prefix(src)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let out = dest.join(&rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
        }
        fs::copy(&path, &out).map_err(|e| ToolError::new(format!("copy {rel}: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("anya-plugin-pack-{}", Uuid::new_v4().simple()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_sample(root: &Path) {
        fs::create_dir_all(root.join("ui").join("src")).unwrap();
        fs::create_dir_all(root.join("ui").join(".anya")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("plugin.json"),
            r#"{"id":"pack-demo","name":"Pack","version":"1.2.0","apiVersion":"1.0"}"#,
        )
        .unwrap();
        fs::write(
            root.join("ui").join("src").join("activate.js"),
            "export function activate() {}",
        )
        .unwrap();
        fs::write(
            root.join("ui").join(".anya").join("activate.js"),
            "generated",
        )
        .unwrap();
        fs::write(root.join("data").join("cli-port.txt"), "18765").unwrap();
    }

    #[test]
    fn pack_skips_generated_dirs_and_roundtrips() {
        let root = temp_dir();
        write_sample(&root);
        let zip_path = root
            .parent()
            .unwrap()
            .join(format!("{}.zip", Uuid::new_v4().simple()));
        pack_folder_to_zip(&root, &zip_path).unwrap();
        let staging = temp_dir();
        extract_zip(&zip_path, &staging).unwrap();
        assert!(staging.join("plugin.json").is_file());
        assert!(staging.join("ui").join("src").join("activate.js").is_file());
        assert!(!staging
            .join("ui")
            .join(".anya")
            .join("activate.js")
            .is_file());
        assert!(!staging.join("data").join("cli-port.txt").is_file());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(zip_path);
        let _ = fs::remove_dir_all(staging);
    }

    #[test]
    fn rejects_exe_and_zip_slip() {
        let root = temp_dir();
        write_sample(&root);
        fs::write(root.join("evil.exe"), b"MZ").unwrap();
        let zip_path = root
            .parent()
            .unwrap()
            .join(format!("{}.zip", Uuid::new_v4().simple()));
        assert!(pack_folder_to_zip(&root, &zip_path).is_err());

        let slip = root
            .parent()
            .unwrap()
            .join(format!("slip-{}.zip", Uuid::new_v4().simple()));
        {
            let file = File::create(&slip).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("../escape.js", opts).unwrap();
            zip.write_all(b"export function activate() {}").unwrap();
            zip.finish().unwrap();
        }
        let staging = temp_dir();
        assert!(extract_zip(&slip, &staging).is_err());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(slip);
        let _ = fs::remove_dir_all(staging);
    }

    #[test]
    fn official_ids_are_blocked() {
        assert!(is_official_plugin_id("terminal"));
        assert!(is_official_plugin_id("computer-use"));
        assert!(!is_official_plugin_id("pet-gallery"));
        assert!(!is_official_plugin_id("system-terminal"));
        assert!(!is_official_plugin_id("anya-cli"));
        assert!(!is_official_plugin_id("pack-demo"));
    }

    #[test]
    fn nested_folder_zip_finds_manifest() {
        let outer = temp_dir();
        let inner = outer.join("pack-demo");
        write_sample(&inner);
        assert_eq!(find_plugin_root(&outer).unwrap(), inner);
        let _ = fs::remove_dir_all(outer);
    }
}
