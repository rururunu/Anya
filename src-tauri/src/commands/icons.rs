//! Install-scoped icon cache.
//!
//! Policy:
//! - Only called when the user installs an MCP / Skill (not while browsing catalogs).
//! - Files are keyed by stable install identity (`mcp/{id}`, `skill/{id}`), never by
//!   display name alone, so renames / collisions do not mix icons.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

fn icons_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("cache dir unavailable: {error}"))?
        .join("icons");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create icons cache: {error}"))?;
    Ok(dir)
}

fn sanitize_cache_key(kind: &str, cache_key: &str) -> Result<(String, String), String> {
    let kind = kind.trim().to_ascii_lowercase();
    if kind != "mcp" && kind != "skill" && kind != "provider" {
        return Err("kind must be `mcp`, `skill`, or `provider`".into());
    }
    let key = cache_key
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_ascii_lowercase();
    if key.is_empty() || key.len() > 96 {
        return Err("invalid icon cache key".into());
    }
    Ok((kind, key))
}

fn kind_dir(app: &AppHandle, kind: &str) -> Result<PathBuf, String> {
    let dir = icons_root(app)?.join(kind);
    std::fs::create_dir_all(&dir).map_err(|error| format!("failed to create icon dir: {error}"))?;
    Ok(dir)
}

fn find_existing(dir: &Path, key: &str) -> Option<PathBuf> {
    for ext in ["png", "jpg", "webp", "gif", "svg"] {
        let path = dir.join(format!("{key}.{ext}"));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn ext_from_url_or_ctype(url: &str, content_type: Option<&str>) -> &'static str {
    let lower_ctype = content_type.unwrap_or("").to_ascii_lowercase();
    if lower_ctype.contains("image/jpeg") || lower_ctype.contains("image/jpg") {
        return "jpg";
    }
    if lower_ctype.contains("image/webp") {
        return "webp";
    }
    if lower_ctype.contains("image/gif") {
        return "gif";
    }
    if lower_ctype.contains("image/svg") {
        return "svg";
    }
    if lower_ctype.contains("image/png") {
        return "png";
    }
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        return "jpg";
    }
    if path.ends_with(".webp") {
        return "webp";
    }
    if path.ends_with(".gif") {
        return "gif";
    }
    if path.ends_with(".svg") {
        return "svg";
    }
    "png"
}

fn normalize_http_url(url: &str) -> Result<String, String> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("icon url is empty".into());
    }
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) icon urls can be cached".into());
    }
    Ok(url)
}

/// Look up an install icon by stable identity (no network).
#[tauri::command]
pub fn lookup_install_icon(
    app: AppHandle,
    kind: String,
    cache_key: String,
) -> Result<Option<String>, String> {
    let (kind, key) = sanitize_cache_key(&kind, &cache_key)?;
    let dir = kind_dir(&app, &kind)?;
    Ok(find_existing(&dir, &key).map(|path| path.to_string_lossy().into_owned()))
}

/// Encode a cached install icon as a `data:` URL for remote clients (phone companion).
pub fn install_icon_data_url(app: &AppHandle, kind: &str, cache_key: &str) -> Option<String> {
    let (kind, key) = sanitize_cache_key(kind, cache_key).ok()?;
    let dir = kind_dir(app, &kind).ok()?;
    let path = find_existing(&dir, &key)?;
    let bytes = std::fs::read(&path).ok()?;
    if bytes.is_empty() || bytes.len() > 512 * 1024 {
        return None;
    }
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconLookupEntry {
    pub kind: String,
    pub cache_key: String,
}

/// Batch disk lookup for install icons. Keys in the result are `kind:cacheKey`.
#[tauri::command]
pub fn lookup_install_icons(
    app: AppHandle,
    entries: Vec<IconLookupEntry>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut out = std::collections::HashMap::new();
    for entry in entries {
        let Ok((kind, key)) = sanitize_cache_key(&entry.kind, &entry.cache_key) else {
            continue;
        };
        let Ok(dir) = kind_dir(&app, &kind) else {
            continue;
        };
        if let Some(path) = find_existing(&dir, &key) {
            // Preserve the caller-facing key so the frontend memory map matches.
            out.insert(
                format!("{}:{}", entry.kind.trim(), entry.cache_key.trim()),
                path.to_string_lossy().into_owned(),
            );
        }
    }
    Ok(out)
}

/// Cache an icon for an installed MCP/Skill.
/// Keyed by install identity; downloads only when that key is missing on disk.
#[tauri::command]
pub async fn cache_install_icon(
    app: AppHandle,
    kind: String,
    cache_key: String,
    url: String,
) -> Result<String, String> {
    let (kind, key) = sanitize_cache_key(&kind, &cache_key)?;
    let url = normalize_http_url(&url)?;
    let dir = kind_dir(&app, &kind)?;

    if let Some(existing) = find_existing(&dir, &key) {
        return Ok(existing.to_string_lossy().into_owned());
    }

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("Anya/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("icon download failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("icon download failed ({})", response.status()));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let ext = ext_from_url_or_ctype(&url, content_type.as_deref());
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("icon body read failed: {error}"))?;
    if bytes.is_empty() {
        return Err("icon body is empty".into());
    }
    if bytes.starts_with(b"<!DOCTYPE") || bytes.starts_with(b"<html") || bytes.starts_with(b"<HTML")
    {
        return Err("icon url returned HTML instead of an image".into());
    }

    // Fingerprint URL into a sidecar so we can refresh later if the remote icon changes.
    let meta_path = dir.join(format!("{key}.urlsha"));
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let _ = std::fs::write(&meta_path, format!("{:x}", hasher.finalize()));

    let out = dir.join(format!("{key}.{ext}"));
    std::fs::write(&out, &bytes).map_err(|error| format!("failed to write icon cache: {error}"))?;
    Ok(out.to_string_lossy().into_owned())
}

/// Remove cached install icon when the user uninstalls.
#[tauri::command]
pub fn clear_install_icon(app: AppHandle, kind: String, cache_key: String) -> Result<(), String> {
    let (kind, key) = sanitize_cache_key(&kind, &cache_key)?;
    let dir = kind_dir(&app, &kind)?;
    for ext in ["png", "jpg", "webp", "gif", "svg", "urlsha"] {
        let path = dir.join(format!("{key}.{ext}"));
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}
