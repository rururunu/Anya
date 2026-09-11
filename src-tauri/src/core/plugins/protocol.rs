//! `anya-plugin://localhost/<id>/...` serves files only from that plugin folder.
//!
//! Windows WebView2 rewrites this to `http://anya-plugin.localhost/<id>/...`.
//! The handler must accept both forms. Serving on the WebView UI thread (sync
//! protocol) deadlocks window creation, so registration is asynchronous.

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use tauri::http::{header::CONTENT_TYPE, Request, Response, StatusCode};

use super::{load_manifest, plugin_dir, PLUGIN_SCHEME};

pub fn handle_plugin_protocol(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    if request.method() == tauri::http::Method::OPTIONS {
        return cors(
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Cow::Borrowed(&b""[..])),
        )
        .unwrap_or_else(|_| empty_not_found());
    }
    match serve(&request) {
        Ok(response) => response,
        Err(_) => cors(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Cow::Borrowed(
                    &b"<!DOCTYPE html><html><body style=\"background:#12141a;color:#e8eaed;font:15px system-ui;padding:2rem\">Plugin page not found.</body></html>"[..],
                )),
        )
        .unwrap_or_else(|_| empty_not_found()),
    }
}

fn empty_not_found() -> Response<Cow<'static, [u8]>> {
    Response::new(Cow::Borrowed(&b""[..]))
}

fn cors(
    builder: Result<Response<Cow<'static, [u8]>>, tauri::http::Error>,
) -> Result<Response<Cow<'static, [u8]>>, tauri::http::Error> {
    let mut response = builder?;
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, OPTIONS".parse().unwrap(),
    );
    headers.insert("Access-Control-Allow-Headers", "*".parse().unwrap());
    Ok(response)
}

/// `(plugin_id, relative path inside the plugin)`.
pub fn parse_plugin_request(
    scheme: Option<&str>,
    host: Option<&str>,
    path: &str,
) -> Option<(String, String)> {
    let path = path.trim_start_matches('/').replace('\\', "/");
    let path = path
        .strip_prefix("localhost/")
        .unwrap_or(path.as_str())
        .to_string();
    let host = host.unwrap_or("");
    let scheme = scheme.unwrap_or("");
    let windows_host = format!("{PLUGIN_SCHEME}.localhost");
    let (id, rel) = if scheme == PLUGIN_SCHEME
        && host != "localhost"
        && !host.is_empty()
        && host != windows_host
    {
        (host.to_string(), path)
    } else if scheme == PLUGIN_SCHEME || host == windows_host || host == "localhost" {
        let mut parts = path.splitn(2, '/');
        let id = parts.next().unwrap_or("").to_string();
        let rel = parts.next().unwrap_or("").to_string();
        (id, rel)
    } else {
        return None;
    };
    if id.is_empty() || id == "localhost" || id.contains("..") || rel.contains("..") {
        return None;
    }
    Some((id, rel))
}

fn serve(request: &Request<Vec<u8>>) -> Result<Response<Cow<'static, [u8]>>, ()> {
    let url = request.uri();
    let (id, mut rel) = parse_plugin_request(url.scheme_str(), url.host(), url.path()).ok_or(())?;
    let manifest = load_manifest(&id).map_err(|_| ())?;
    let root = plugin_dir(&manifest.id).map_err(|_| ())?;
    if rel.is_empty() {
        rel = manifest.entry.trim_start_matches('/').to_string();
    }
    let bundled = root.join("ui").join(".anya").join("activate.js");
    let path = if bundled.is_file()
        && (rel == manifest.ui.entry || rel == "ui/activate.js" || rel == "ui/.anya/activate.js")
    {
        bundled
    } else {
        root.join(&rel)
    };
    if !contained_in(&root, &path) || !path.is_file() {
        return Err(());
    }
    let bytes = fs::read(&path).map_err(|_| ())?;
    let mime = mime_for(&path);
    let len = bytes.len() as u64;
    let range_header = request
        .headers()
        .get("Range")
        .and_then(|value| value.to_str().ok());
    if let Some((start, end)) = parse_byte_range(range_header, len) {
        let slice = bytes[start as usize..=end as usize].to_vec();
        return cors(
            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, mime)
                .header("Accept-Ranges", "bytes")
                .header("Content-Range", format!("bytes {start}-{end}/{len}"))
                .header("Content-Length", slice.len().to_string())
                .body(Cow::Owned(slice)),
        )
        .map_err(|_| ());
    }
    cors(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, mime)
            .header("Accept-Ranges", "bytes")
            .header("Content-Length", len.to_string())
            .body(Cow::Owned(bytes)),
    )
    .map_err(|_| ())
}

/// Inclusive `start..=end` from a `Range: bytes=` header. Invalid → full body.
fn parse_byte_range(header: Option<&str>, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let spec = header?.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (start_raw, end_raw) = spec.split_once('-')?;
    let (start, end) = if start_raw.is_empty() {
        let suffix: u64 = end_raw.parse().ok()?;
        (len.saturating_sub(suffix), len.saturating_sub(1))
    } else {
        let start: u64 = start_raw.parse().ok()?;
        let end = if end_raw.is_empty() {
            len.saturating_sub(1)
        } else {
            end_raw.parse().ok()?
        };
        (start, end)
    };
    if start > end || start >= len {
        return None;
    }
    Some((start, end.min(len - 1)))
}

fn contained_in(root: &Path, candidate: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_start_matches(r"\\?\")
            .to_ascii_lowercase()
    };
    let root = norm(root);
    let candidate = norm(candidate);
    candidate == root || candidate.starts_with(&format!("{root}\\"))
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "woff2" => "font/woff2",
        "webm" => "video/webm",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_rewritten_url() {
        let parsed = parse_plugin_request(
            Some("http"),
            Some("anya-plugin.localhost"),
            "/terminal/ui/index.html",
        )
        .unwrap();
        assert_eq!(parsed.0, "terminal");
        assert_eq!(parsed.1, "ui/index.html");
    }

    #[test]
    fn parses_custom_protocol_localhost() {
        let parsed = parse_plugin_request(
            Some("anya-plugin"),
            Some("localhost"),
            "/photo-editor/ui/index.html",
        )
        .unwrap();
        assert_eq!(parsed.0, "photo-editor");
        assert_eq!(parsed.1, "ui/index.html");
    }

    #[test]
    fn parses_windows_host_plus_localhost_path() {
        let parsed = parse_plugin_request(
            Some("http"),
            Some("anya-plugin.localhost"),
            "/localhost/terminal/ui/index.html",
        )
        .unwrap();
        assert_eq!(parsed.0, "terminal");
        assert_eq!(parsed.1, "ui/index.html");
    }

    #[test]
    fn rejects_path_escape() {
        assert!(parse_plugin_request(
            Some("anya-plugin"),
            Some("localhost"),
            "/terminal/../secret.txt",
        )
        .is_none());
    }

    #[test]
    fn parses_open_ended_byte_range() {
        assert_eq!(parse_byte_range(Some("bytes=0-"), 1000), Some((0, 999)));
        assert_eq!(
            parse_byte_range(Some("bytes=100-199"), 1000),
            Some((100, 199))
        );
        assert_eq!(parse_byte_range(Some("bytes=-50"), 1000), Some((950, 999)));
        assert_eq!(parse_byte_range(None, 1000), None);
    }
}
