//! DeepSeek Files API (`POST/GET/DELETE /files`) for vision `file_id` citations.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::runtime::ChatMessage;

use super::ProviderError;

pub const FILES_URL: &str = "https://api.deepseek.com/files";
pub const ANTHROPIC_FILES_BETA: &str = "files-api-2025-04-14";
pub const PURPOSE_USER_DATA: &str = "user_data";
const MAX_BYTES: usize = 64 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Chat auto-uploads expire after 30 days (API maximum).
const CHAT_EXPIRES_SECONDS: u64 = 2_592_000;

static FILE_ID_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekFileObject {
    pub id: String,
    pub object: String,
    pub bytes: i64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekFileList {
    pub object: String,
    pub data: Vec<DeepSeekFileObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekFileDeleteResult {
    pub id: String,
    pub deleted: bool,
}

#[derive(Debug, Deserialize)]
struct RawFileObject {
    id: String,
    object: String,
    bytes: i64,
    created_at: i64,
    filename: String,
    purpose: String,
    #[serde(default)]
    expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawFileList {
    object: String,
    #[serde(default)]
    data: Vec<RawFileObject>,
    first_id: Option<String>,
    last_id: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct RawDelete {
    id: String,
    deleted: bool,
}

/// Built-in DeepSeek provider only — custom proxies do not host `/files`.
pub fn uses_files_api(provider_id: &str) -> bool {
    provider_id == "deepseek"
}

/// Extract a Files API id from `file:file-api-…` or a bare `file-api-…` ref.
pub fn file_id_from_ref(raw: &str) -> Option<&str> {
    let value = raw.trim();
    let id = value.strip_prefix("file:").unwrap_or(value).trim();
    if id.starts_with("file-api-") {
        Some(id)
    } else {
        None
    }
}

pub fn json_contains_file_id(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            if map.get("file_id").and_then(Value::as_str).is_some() {
                return true;
            }
            map.values().any(json_contains_file_id)
        }
        Value::Array(items) => items.iter().any(json_contains_file_id),
        _ => false,
    }
}

/// Upload local images in chat markdown so DeepSeek can cite `file_id`.
pub async fn attach_files_to_messages(api_key: &str, messages: &mut [ChatMessage]) {
    let re = image_markdown_re();
    for message in messages.iter_mut() {
        if !message.content.contains("![image](") {
            continue;
        }
        let original = message.content.clone();
        let mut rewritten = String::with_capacity(original.len());
        let mut last = 0;
        for cap in re.captures_iter(&original) {
            let Some(mat) = cap.get(0) else { continue };
            rewritten.push_str(&original[last..mat.start()]);
            let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            match ensure_file_id(api_key, raw).await {
                Ok(id) => rewritten.push_str(&format!("![image](file:{id})")),
                Err(error) => {
                    tracing::warn!(
                        target: "anya::ai",
                        error = %error,
                        raw = %raw,
                        "DeepSeek Files upload failed; keeping original image ref"
                    );
                    rewritten.push_str(&original[mat.start()..mat.end()]);
                }
            }
            last = mat.end();
        }
        rewritten.push_str(&original[last..]);
        message.content = rewritten;
    }
}

pub async fn upload_path(
    api_key: &str,
    path: &str,
    expires_seconds: Option<u64>,
) -> Result<DeepSeekFileObject, ProviderError> {
    let bytes = std::fs::read(path)
        .map_err(|error| ProviderError::message(format!("failed to read file: {error}")))?;
    let filename = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.png");
    let (bytes, mime, filename) = prepare_image_bytes(bytes, filename)?;
    upload_file(api_key, bytes, &filename, &mime, expires_seconds).await
}

pub async fn upload_file(
    api_key: &str,
    bytes: Vec<u8>,
    filename: &str,
    mime: &str,
    expires_seconds: Option<u64>,
) -> Result<DeepSeekFileObject, ProviderError> {
    if bytes.len() > MAX_BYTES {
        return Err(ProviderError::message(format!(
            "file exceeds 64 MiB limit ({} bytes)",
            bytes.len()
        )));
    }
    let mut form = reqwest::multipart::Form::new().text("purpose", PURPOSE_USER_DATA);
    if let Some(seconds) = expires_seconds {
        form = form
            .text("expires_after[anchor]", "created_at")
            .text("expires_after[seconds]", seconds.to_string());
    }
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename.to_string())
        .mime_str(mime)
        .map_err(|error| ProviderError::message(format!("invalid image type: {error}")))?;
    form = form.part("file", part);

    let response = files_client()?
        .post(FILES_URL)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .multipart(form)
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;
    let text = read_success(response).await?;
    parse_file_object(&text)
}

pub async fn list_files(
    api_key: &str,
    after: Option<&str>,
    limit: Option<u32>,
    order: Option<&str>,
) -> Result<DeepSeekFileList, ProviderError> {
    let mut request = files_client()?
        .get(FILES_URL)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .query(&[("purpose", PURPOSE_USER_DATA)]);
    if let Some(after) = after.filter(|value| !value.is_empty()) {
        request = request.query(&[("after", after)]);
    }
    if let Some(limit) = limit {
        request = request.query(&[("limit", limit.clamp(1, 1000))]);
    }
    if let Some(order) = order.filter(|value| *value == "asc" || *value == "desc") {
        request = request.query(&[("order", order)]);
    }
    let response = request
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;
    let text = read_success(response).await?;
    parse_file_list(&text)
}

pub async fn retrieve_file(
    api_key: &str,
    file_id: &str,
) -> Result<DeepSeekFileObject, ProviderError> {
    let response = files_client()?
        .get(format!("{FILES_URL}/{file_id}"))
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;
    let text = read_success(response).await?;
    parse_file_object(&text)
}

pub async fn delete_file(
    api_key: &str,
    file_id: &str,
) -> Result<DeepSeekFileDeleteResult, ProviderError> {
    let response = files_client()?
        .delete(format!("{FILES_URL}/{file_id}"))
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;
    let text = read_success(response).await?;
    let payload: RawDelete = serde_json::from_str(&text)
        .map_err(|error| ProviderError::message(format!("invalid delete payload: {error}")))?;
    Ok(DeepSeekFileDeleteResult {
        id: payload.id,
        deleted: payload.deleted,
    })
}

pub fn parse_file_object(text: &str) -> Result<DeepSeekFileObject, ProviderError> {
    let raw: RawFileObject = serde_json::from_str(text)
        .map_err(|error| ProviderError::message(format!("invalid file payload: {error}")))?;
    Ok(map_file(raw))
}

pub fn parse_file_list(text: &str) -> Result<DeepSeekFileList, ProviderError> {
    let raw: RawFileList = serde_json::from_str(text)
        .map_err(|error| ProviderError::message(format!("invalid file list payload: {error}")))?;
    Ok(DeepSeekFileList {
        object: raw.object,
        data: raw.data.into_iter().map(map_file).collect(),
        first_id: raw.first_id,
        last_id: raw.last_id,
        has_more: raw.has_more,
    })
}

fn map_file(raw: RawFileObject) -> DeepSeekFileObject {
    DeepSeekFileObject {
        id: raw.id,
        object: raw.object,
        bytes: raw.bytes,
        created_at: raw.created_at,
        filename: raw.filename,
        purpose: raw.purpose,
        expires_at: raw.expires_at,
    }
}

async fn ensure_file_id(api_key: &str, raw: &str) -> Result<String, ProviderError> {
    if let Some(id) = file_id_from_ref(raw) {
        return Ok(id.to_string());
    }
    let (bytes, mime, filename) = load_image_for_upload(raw).await?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if let Some(cached) = cached_file_id(&digest) {
        return Ok(cached);
    }
    let uploaded = upload_file(
        api_key,
        bytes,
        &filename,
        &mime,
        Some(CHAT_EXPIRES_SECONDS),
    )
    .await?;
    remember_file_id(&digest, &uploaded.id);
    Ok(uploaded.id)
}

async fn load_image_for_upload(raw: &str) -> Result<(Vec<u8>, String, String), ProviderError> {
    let value = raw.trim();
    let bytes = if value.starts_with("http://") || value.starts_with("https://") {
        download_bytes(value).await?
    } else {
        crate::core::ai::image_gen::decode_image_source(value)
            .map_err(ProviderError::message)?
    };
    let filename = if value.starts_with("http://") || value.starts_with("https://") {
        value
            .rsplit('/')
            .next()
            .and_then(|name| name.split('?').next())
            .filter(|name| name.contains('.'))
            .unwrap_or("image.png")
    } else {
        Path::new(value)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.contains('.'))
            .unwrap_or("image.png")
    };
    prepare_image_bytes(bytes, filename)
}

fn prepare_image_bytes(
    bytes: Vec<u8>,
    filename: &str,
) -> Result<(Vec<u8>, String, String), ProviderError> {
    if bytes.is_empty() {
        return Err(ProviderError::message("image file is empty"));
    }
    if let Some(mime) = sniff_image_mime(&bytes) {
        let filename = filename_for_mime(filename, mime);
        return Ok((bytes, mime.to_string(), filename));
    }
    let jpeg = encode_jpeg(&bytes)?;
    Ok((jpeg, "image/jpeg".into(), filename_for_mime(filename, "image/jpeg")))
}

fn filename_for_mime(filename: &str, mime: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("image");
    let ext = match mime {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    };
    format!("{stem}.{ext}")
}

fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else {
        None
    }
}

fn encode_jpeg(bytes: &[u8]) -> Result<Vec<u8>, ProviderError> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| ProviderError::message(format!("unsupported image: {error}")))?;
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
        .map_err(|error| ProviderError::message(format!("failed to encode jpeg: {error}")))?;
    Ok(out)
}

async fn download_bytes(url: &str) -> Result<Vec<u8>, ProviderError> {
    let response = files_client()?
        .get(url)
        .header("User-Agent", "Anya/0.2")
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("failed to download image: {error}")))?;
    if !response.status().is_success() {
        return Err(ProviderError::message(format!(
            "failed to download image: {}",
            response.status()
        )));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| ProviderError::message(format!("failed to download image: {error}")))
}

fn files_client() -> Result<reqwest::Client, ProviderError> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| ProviderError::message(format!("failed to build HTTP client: {error}")))
}

async fn read_success(response: reqwest::Response) -> Result<String, ProviderError> {
    let status = response.status();
    let text = response
        .text()
        .await
        .unwrap_or_else(|_| "unknown error".to_string());
    if !status.is_success() {
        return Err(ProviderError::message(format!("Files API {status}: {text}")));
    }
    Ok(text)
}

fn image_markdown_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"!\[image\]\((.*?)\)").expect("image markdown regex"))
}

fn cached_file_id(digest: &str) -> Option<String> {
    FILE_ID_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()?
        .get(digest)
        .cloned()
}

fn remember_file_id(digest: &str, file_id: &str) {
    if let Ok(mut cache) = FILE_ID_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        cache.insert(digest.to_string(), file_id.to_string());
    }
}
