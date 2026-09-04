//! Local HTTP API on 127.0.0.1:18480
//!
//! Routes:
//! - POST /api/context/ide   — legacy IDE context push (VS Code / JetBrains)
//! - POST /api/ask/image     — open Anya overlay with an attached image
//!
//! Preferred IDE context path is on-demand pull via [`super::ide_bridge`].

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::AppHandle;

use crate::core::context::image_capture::file_to_data_url;
use crate::core::context::models::{CursorPosition, IDEContext};
use crate::services::window::open_overlay_with_images;

const LISTEN_ADDRESS: &str = "127.0.0.1:18480";
const IDE_CONTEXT_PATH: &str = "/api/context/ide";
const ASK_IMAGE_PATH: &str = "/api/ask/image";
const MAX_IDE_BODY_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BODY_BYTES: usize = 20 * 1024 * 1024;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_SELECTION_CHARS: usize = 8_000;
const IDE_CONTEXT_TTL: Duration = Duration::from_secs(5 * 60);

struct StoredIDEContext {
    received_at: Instant,
    context: IDEContext,
}

static LATEST_IDE_CONTEXT: OnceLock<RwLock<Option<StoredIDEContext>>> = OnceLock::new();
static SERVER_STARTED: OnceLock<()> = OnceLock::new();
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct IDEContextPayload {
    provider: String,
    ide: String,
    workspace: Option<PathBuf>,
    active_file: Option<PathBuf>,
    language: Option<String>,
    selection: Option<IDESelectionPayload>,
    cursor: Option<CursorPosition>,
}

#[derive(Debug, Deserialize)]
struct IDESelectionPayload {
    text: String,
    #[allow(dead_code)]
    start_line: Option<u32>,
    #[allow(dead_code)]
    end_line: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskImagePayload {
    /// data URL (`data:image/...;base64,...`) or absolute image file path
    image: Option<String>,
    /// absolute image file path (alias)
    path: Option<String>,
    /// raw base64 without data-URL prefix; requires `mime`
    base64: Option<String>,
    mime: Option<String>,
}

fn context_store() -> &'static RwLock<Option<StoredIDEContext>> {
    LATEST_IDE_CONTEXT.get_or_init(|| RwLock::new(None))
}

/// Legacy push-cache lookup (TTL). Prefer [`super::ide_bridge::latest`].
pub fn latest_cached() -> Option<IDEContext> {
    match context_store().read() {
        Ok(stored) => stored
            .as_ref()
            .filter(|stored| stored.received_at.elapsed() <= IDE_CONTEXT_TTL)
            .map(|stored| stored.context.clone()),
        Err(error) => {
            tracing::warn!(provider = "local_api", error = %error, "context provider failed");
            None
        }
    }
}

/// Whether the stored IDE context was received within `max_age` (pull or push).
pub fn is_fresh(max_age: Duration) -> bool {
    match context_store().read() {
        Ok(stored) => stored
            .as_ref()
            .is_some_and(|stored| stored.received_at.elapsed() <= max_age),
        Err(_) => false,
    }
}

/// Persist an IDE context from pull or push.
pub fn store_ide_context(context: IDEContext) {
    if let Ok(mut stored) = context_store().write() {
        *stored = Some(StoredIDEContext {
            received_at: Instant::now(),
            context,
        });
    }
}

/// Drop the cached IDE context (e.g. bridge reported an empty selection).
pub fn clear_ide_context() {
    if let Ok(mut stored) = context_store().write() {
        *stored = None;
    }
}

/// Parse a push/pull IDE JSON body into context. `Ok(None)` means empty selection.
pub fn parse_ide_context_payload(body: &[u8]) -> Result<Option<IDEContext>, String> {
    let payload: IDEContextPayload = serde_json::from_slice(body)
        .map_err(|error| format!("invalid IDE context JSON: {error}"))?;
    if payload.provider != "ide" {
        return Err("provider must be ide".to_string());
    }
    let ide = payload.ide.trim();
    if ide.is_empty() {
        return Err("ide must not be empty".to_string());
    }

    let selection = payload
        .selection
        .and_then(|selection| non_empty(Some(selection.text)))
        .map(|selection| truncate_chars(&selection, MAX_SELECTION_CHARS));
    if selection.is_none() {
        return Ok(None);
    }

    Ok(Some(IDEContext {
        ide: ide.to_string(),
        active_file: absolute_path(payload.active_file),
        workspace: absolute_path(payload.workspace),
        language: non_empty(payload.language),
        selection,
        cursor: payload.cursor,
    }))
}

pub fn start_server(app: AppHandle) {
    let _ = APP_HANDLE.set(app);

    if SERVER_STARTED.set(()).is_err() {
        return;
    }

    std::thread::spawn(|| {
        let listener = match TcpListener::bind(LISTEN_ADDRESS) {
            Ok(listener) => listener,
            Err(error) => {
                tracing::warn!(
                    provider = "local_api",
                    address = LISTEN_ADDRESS,
                    error = %error,
                    "local API server failed to start"
                );
                return;
            }
        };
        tracing::debug!(
            provider = "local_api",
            address = LISTEN_ADDRESS,
            "local API server listening"
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(error) = handle_connection(stream) {
                        tracing::warn!(provider = "local_api", error = %error, "request failed");
                    }
                }
                Err(error) => {
                    tracing::warn!(provider = "local_api", error = %error, "accept failed");
                }
            }
        }
    });
}

fn handle_connection(mut stream: TcpStream) -> Result<(), String> {
    let timeout = Some(Duration::from_secs(5));
    stream
        .set_read_timeout(timeout)
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(timeout)
        .map_err(|error| error.to_string())?;

    let request = read_request(&stream)?;
    let result = match request.path.as_str() {
        IDE_CONTEXT_PATH => receive_ide_payload(&request.body),
        ASK_IMAGE_PATH => receive_ask_image(&request.body),
        _ => Err(format!(
            "unknown path {}; expected {IDE_CONTEXT_PATH} or {ASK_IMAGE_PATH}",
            request.path
        )),
    };

    let (status, response_body) = match &result {
        Ok(()) => ("204 No Content", ""),
        Err(error) => ("400 Bad Request", error.as_str()),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\n{response_body}",
        response_body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| error.to_string())?;
    result
}

struct HttpRequest {
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> Result<HttpRequest, String> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| error.to_string())?;
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("POST") {
        return Err("only POST is supported".to_string());
    }
    let path = parts
        .next()
        .ok_or_else(|| "missing request path".to_string())?
        .to_string();

    let max_body = if path == ASK_IMAGE_PATH {
        MAX_IMAGE_BODY_BYTES
    } else {
        MAX_IDE_BODY_BYTES
    };

    let mut content_length = None;
    let mut header_bytes = request_line.len();
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        header_bytes = header_bytes.saturating_add(line.len());
        if header_bytes > MAX_HEADER_BYTES {
            return Err("request headers are too large".to_string());
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "invalid Content-Length".to_string())?,
                );
            }
        }
    }

    let content_length = content_length.ok_or_else(|| "missing Content-Length".to_string())?;
    if content_length > max_body {
        return Err("request body is too large".to_string());
    }
    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| error.to_string())?;
    Ok(HttpRequest { path, body })
}

fn receive_ide_payload(body: &[u8]) -> Result<(), String> {
    let Some(context) = parse_ide_context_payload(body)? else {
        return Ok(());
    };
    tracing::debug!(
        provider = "ide",
        ide = %context.ide,
        file = ?context.active_file.as_ref().map(|path| path.display().to_string()),
        workspace = ?context.workspace.as_ref().map(|path| path.display().to_string()),
        selection_length = context.selection.as_ref().map_or(0, |text| text.chars().count()),
        "ide context received"
    );
    store_ide_context(context);
    Ok(())
}

fn receive_ask_image(body: &[u8]) -> Result<(), String> {
    let payload: AskImagePayload =
        serde_json::from_slice(body).map_err(|error| format!("invalid ask image JSON: {error}"))?;

    let data_url = resolve_image_data_url(&payload)?;
    let app = APP_HANDLE
        .get()
        .ok_or_else(|| "app handle is unavailable".to_string())?
        .clone();

    let app_for_overlay = app.clone();
    app.run_on_main_thread(move || {
        open_overlay_with_images(&app_for_overlay, vec![data_url]);
    })
    .map_err(|error| format!("failed to open overlay: {error}"))?;

    Ok(())
}

fn resolve_image_data_url(payload: &AskImagePayload) -> Result<String, String> {
    if let Some(image) = payload
        .image
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if image.starts_with("data:image/") {
            return Ok(image.to_string());
        }
        let path = PathBuf::from(image);
        if path.is_absolute() {
            return file_to_data_url(&path).map_err(|error| error.to_string());
        }
        return Err("image must be a data URL or absolute file path".to_string());
    }

    if let Some(path) = payload
        .path
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("path must be absolute".to_string());
        }
        return file_to_data_url(&path).map_err(|error| error.to_string());
    }

    if let Some(base64) = payload
        .base64
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let mime = payload
            .mime
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("image/png");
        if !mime.starts_with("image/") {
            return Err("mime must be an image/* type".to_string());
        }
        return Ok(format!("data:{mime};base64,{base64}"));
    }

    Err("provide image, path, or base64".to_string())
}

fn absolute_path(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_payload_deserializes_to_ide_context() {
        let payload = br#"{
            "provider":"ide",
            "ide":"vscode",
            "workspace":"C:/project",
            "active_file":"C:/project/src/main.rs",
            "language":"rust",
            "selection":{"text":"fn main() {}","start_line":10,"end_line":10},
            "cursor":{"line":10,"column":5}
        }"#;

        let parsed: IDEContextPayload = serde_json::from_slice(payload).expect("IDE payload");
        assert_eq!(parsed.ide, "vscode");
        assert_eq!(
            parsed.cursor,
            Some(CursorPosition {
                line: 10,
                column: 5
            })
        );
        assert_eq!(parsed.selection.unwrap().text, "fn main() {}");
    }

    #[test]
    fn ask_image_payload_accepts_data_url_field() {
        let payload = br#"{"image":"data:image/png;base64,abc"}"#;
        let parsed: AskImagePayload = serde_json::from_slice(payload).expect("ask image");
        let url = resolve_image_data_url(&parsed).expect("data url");
        assert!(url.starts_with("data:image/png;base64,"));
    }
}
