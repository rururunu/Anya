//! Companion → desktop file uploads.
//!
//! Workspace chats land under `{workspace}/.anya/uploads/{sessionId}/`.
//! Ask chats (no bound workspace) land under `{config}/companion-inbox/{sessionId}/`.
//! Chunked to disk; the 500MB cap is for bid docs and similar, not a WS frame limit.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::app_state::AppState;

pub const MAX_UPLOAD_BYTES: u64 = 500 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: usize = 512 * 1024;

struct ActiveUpload {
    session_id: String,
    dest: PathBuf,
    rel_path: String,
    expected: u64,
    written: u64,
    file: File,
}

fn store() -> &'static Mutex<HashMap<String, ActiveUpload>> {
    static STORE: OnceLock<Mutex<HashMap<String, ActiveUpload>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn inbox_dir(app: &AppHandle, session_id: &str) -> Option<PathBuf> {
    let sid = session_id.trim();
    if sid.is_empty() {
        return None;
    }
    Some(config_dir(app).join("companion-inbox").join(sid))
}

pub fn inbox_root_if_exists(app: &AppHandle, session_id: &str) -> Option<PathBuf> {
    let dir = inbox_dir(app, session_id)?;
    dir.is_dir().then_some(dir)
}

pub fn cleanup_session_uploads(app: &AppHandle, session_id: &str, workspace_root: Option<&Path>) {
    let sid = session_id.trim();
    if sid.is_empty() {
        return;
    }
    abort_session_uploads(sid);
    if let Some(dir) = inbox_dir(app, sid) {
        let _ = fs::remove_dir_all(dir);
    }
    if let Some(root) = workspace_root {
        let dir = root.join(".anya").join("uploads").join(sid);
        let _ = fs::remove_dir_all(dir);
    }
}

pub fn begin(
    app: &AppHandle,
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    file_name: &str,
    size: u64,
) -> Result<Value, String> {
    if size > MAX_UPLOAD_BYTES {
        return Err(format!(
            "file too large: {size} bytes (max {MAX_UPLOAD_BYTES})"
        ));
    }
    let session_id = allocate_session_id(session_id);
    if let Some(ws) = bound_workspace(app, &session_id, workspace_id) {
        if let Some(state) = app.try_state::<AppState>() {
            state
                .core
                .chat()
                .conversation()
                .bind_workspace(&session_id, &ws.id);
        }
    }
    let dest_dir = dest_dir(app, &session_id, workspace_id)?;
    fs::create_dir_all(&dest_dir).map_err(|e| format!("create upload dir: {e}"))?;
    let safe_name = unique_name(&dest_dir, &sanitize_filename(file_name));
    let dest = dest_dir.join(&safe_name);
    let rel_path = rel_path_for(app, &session_id, workspace_id, &safe_name)?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&dest)
        .map_err(|e| format!("create file: {e}"))?;
    let upload_id = Uuid::new_v4().to_string();
    let mut map = store().lock().map_err(|e| e.to_string())?;
    map.insert(
        upload_id.clone(),
        ActiveUpload {
            session_id: session_id.clone(),
            dest,
            rel_path: rel_path.clone(),
            expected: size,
            written: 0,
            file,
        },
    );
    Ok(json!({
        "uploadId": upload_id,
        "sessionId": session_id,
        "relPath": rel_path,
        "name": safe_name,
        "size": size,
    }))
}

pub fn chunk(upload_id: &str, offset: u64, data_base64: &str) -> Result<Value, String> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    let bytes = B64
        .decode(data_base64.trim())
        .map_err(|_| "invalid base64 chunk".to_string())?;
    if bytes.len() > MAX_CHUNK_BYTES {
        return Err(format!(
            "chunk too large: {} bytes (max {MAX_CHUNK_BYTES})",
            bytes.len()
        ));
    }
    let mut map = store().lock().map_err(|e| e.to_string())?;
    let upload = map
        .get_mut(upload_id)
        .ok_or_else(|| "unknown uploadId".to_string())?;
    if offset != upload.written {
        return Err(format!(
            "offset mismatch: got {offset}, expected {}",
            upload.written
        ));
    }
    let next = upload
        .written
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| "upload overflow".to_string())?;
    if next > upload.expected {
        return Err("chunk exceeds declared size".into());
    }
    upload
        .file
        .write_all(&bytes)
        .map_err(|e| format!("write chunk: {e}"))?;
    upload.written = next;
    Ok(json!({
        "uploadId": upload_id,
        "written": upload.written,
        "expected": upload.expected,
    }))
}

pub fn finish(upload_id: &str) -> Result<Value, String> {
    let mut map = store().lock().map_err(|e| e.to_string())?;
    let upload = map
        .remove(upload_id)
        .ok_or_else(|| "unknown uploadId".to_string())?;
    if upload.written != upload.expected {
        let dest = upload.dest.clone();
        let written = upload.written;
        let expected = upload.expected;
        drop(upload);
        let _ = fs::remove_file(dest);
        return Err(format!(
            "incomplete upload: wrote {written} of {expected} bytes"
        ));
    }
    upload
        .file
        .sync_all()
        .map_err(|e| format!("sync file: {e}"))?;
    Ok(json!({
        "uploadId": upload_id,
        "sessionId": upload.session_id,
        "path": upload.rel_path,
        "name": upload
            .dest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".into()),
        "size": upload.written,
    }))
}

pub fn abort(upload_id: &str) -> Result<Value, String> {
    let mut map = store().lock().map_err(|e| e.to_string())?;
    if let Some(upload) = map.remove(upload_id) {
        drop(upload.file);
        let _ = fs::remove_file(&upload.dest);
    }
    Ok(json!({ "ok": true, "uploadId": upload_id }))
}

fn abort_session_uploads(session_id: &str) {
    let Ok(mut map) = store().lock() else {
        return;
    };
    let ids: Vec<String> = map
        .iter()
        .filter(|(_, u)| u.session_id == session_id)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        if let Some(upload) = map.remove(&id) {
            drop(upload.file);
            let _ = fs::remove_file(&upload.dest);
        }
    }
}

fn allocate_session_id(raw: Option<&str>) -> String {
    match raw.map(str::trim).filter(|s| {
        !s.is_empty() && !s.eq_ignore_ascii_case("new") && !s.eq_ignore_ascii_case("default")
    }) {
        Some(id) => id.to_string(),
        None => Uuid::new_v4().to_string(),
    }
}

fn config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("anya"))
}

fn bound_workspace(
    app: &AppHandle,
    session_id: &str,
    workspace_id: Option<&str>,
) -> Option<crate::core::workspace::Workspace> {
    let state = app.try_state::<AppState>()?;
    let list = state.core.workspaces().list();
    if let Some(id) = workspace_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(found) = list.iter().find(|w| w.id == id).cloned() {
            return Some(found);
        }
    }
    let bound = state
        .core
        .chat()
        .conversation()
        .workspace_for_session(session_id)?;
    list.into_iter().find(|w| w.id == bound)
}

fn dest_dir(
    app: &AppHandle,
    session_id: &str,
    workspace_id: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(ws) = bound_workspace(app, session_id, workspace_id) {
        return Ok(ws.root.join(".anya").join("uploads").join(session_id));
    }
    inbox_dir(app, session_id)
        .map(|root| root.join(".anya").join("uploads"))
        .ok_or_else(|| "missing session id".to_string())
}

fn rel_path_for(
    app: &AppHandle,
    session_id: &str,
    workspace_id: Option<&str>,
    file_name: &str,
) -> Result<String, String> {
    if bound_workspace(app, session_id, workspace_id).is_some() {
        Ok(format!(".anya/uploads/{session_id}/{file_name}"))
    } else {
        Ok(format!(".anya/uploads/{file_name}"))
    }
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    let normalized = name.replace('\\', "/");
    let base = normalized
        .rsplit('/')
        .next()
        .unwrap_or("file")
        .trim();
    let mut out = String::new();
    for ch in base.chars() {
        if ch.is_control() || "<>:\"|?*".contains(ch) {
            continue;
        }
        out.push(ch);
    }
    let out = out.trim_matches('.').trim().to_string();
    if out.is_empty() || out == "." || out == ".." {
        "file".into()
    } else {
        out.chars().take(180).collect()
    }
}

fn unique_name(dir: &Path, name: &str) -> String {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return name.to_string();
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let ext = path
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let suffix = &Uuid::new_v4().simple().to_string()[..6];
    format!("{stem}-{suffix}{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_path_separators_and_dots() {
        assert_eq!(sanitize_filename("..\\secret.txt"), "secret.txt");
        assert_eq!(sanitize_filename("../../x"), "x");
        assert_eq!(sanitize_filename(""), "file");
        assert_eq!(sanitize_filename("a/b/c.pdf"), "c.pdf");
    }
}
