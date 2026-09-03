//! Images API adapter (`POST /v1/images/generations`, or `/v1/images/edits`
//! when a reference image is attached).
//!
//! First supported model is `gpt-image-2`. Endpoints come from **Settings → Image**
//! (`image_providers`), never from chat custom providers. Base URL is typically
//! a dedicated Images host such as `https://api.openai.com/v1`.

use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};

use crate::models::settings::{AppSettings, CustomProviderConfig};

use super::deepseek::normalize_images_generations_url;
use super::image_markdown::MAX_IMAGE_REFS;
use super::registry::provider_model_is_disabled;

pub const DEFAULT_IMAGE_MODEL: &str = "gpt-image-2";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// Keep in sync with `image_markdown::MAX_IMAGE_REFS` and FE `IMAGE_GEN` count max.
const MAX_IMAGES: u8 = MAX_IMAGE_REFS as u8;

// Compatibility re-exports for existing `image_gen::…` call sites.
pub use super::image_markdown::{merge_reference_sources, strip_edit_region_images};

#[derive(Debug, Clone)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub model: String,
    pub size: String,
    pub quality: String,
    pub n: u8,
    pub reference_images: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub extension: String,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone)]
struct ImageEndpoint {
    api_key: String,
    url: String,
}

impl Default for ImageGenRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: DEFAULT_IMAGE_MODEL.to_string(),
            size: "1024x1024".into(),
            quality: "auto".into(),
            n: 1,
            reference_images: Vec::new(),
        }
    }
}

pub fn normalize_quality(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" | "medium" | "high" | "auto" => raw.trim().to_ascii_lowercase(),
        _ => "auto".into(),
    }
}

/// Pixel bounds for Images API `size` (`WxH`).
/// Keep in sync with FE `IMAGE_GEN_MIN_PX` / `IMAGE_GEN_MAX_PX` / `IMAGE_GEN_SIZE_STEP`
/// in `src/services/chat/imageGenMode.ts`.
pub const IMAGE_SIZE_MIN: u32 = 256;
pub const IMAGE_SIZE_MAX: u32 = 4096;
pub const IMAGE_SIZE_STEP: u32 = 16;

fn snap_image_px(value: u32) -> u32 {
    let clamped = value.clamp(IMAGE_SIZE_MIN, IMAGE_SIZE_MAX);
    let rounded = (clamped + IMAGE_SIZE_STEP / 2) / IMAGE_SIZE_STEP * IMAGE_SIZE_STEP;
    rounded.clamp(IMAGE_SIZE_MIN, IMAGE_SIZE_MAX)
}

pub fn normalize_size(raw: &str) -> String {
    let value = raw.trim();
    if value.eq_ignore_ascii_case("auto") {
        return "auto".into();
    }
    let lower = value.to_ascii_lowercase();
    let Some((width, height)) = lower.split_once('x') else {
        return "1024x1024".into();
    };
    let Ok(width) = width.parse::<u32>() else {
        return "1024x1024".into();
    };
    let Ok(height) = height.parse::<u32>() else {
        return "1024x1024".into();
    };
    format!("{}x{}", snap_image_px(width), snap_image_px(height))
}

pub fn normalize_count(n: u64) -> u8 {
    n.clamp(1, MAX_IMAGES as u64) as u8
}

pub fn decode_image_source(raw: &str) -> Result<Vec<u8>, String> {
    let value = raw.trim();
    if value.starts_with("data:image/") {
        return decode_b64(value);
    }
    let path = value.strip_prefix("path:").unwrap_or(value);
    std::fs::read(path).map_err(|error| format!("failed to read reference image: {error}"))
}

/// Vision / chat APIs only accept `data:image/…` or `http(s)://…` in `image_url`.
/// Companion refs arrive as `path:C:\…` / `path:/…` — expand them before the wire.
pub fn resolve_image_url_for_api(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("empty image ref".into());
    }
    if value.starts_with("data:image/") || value.starts_with("http://") || value.starts_with("https://")
    {
        return Ok(value.to_string());
    }
    if value.starts_with("data:") {
        return Ok(value.to_string());
    }
    let bytes = decode_image_source(value)?;
    if bytes.is_empty() {
        return Err("image file is empty".into());
    }
    bytes_to_vision_data_url(&bytes)
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

fn bytes_to_vision_data_url(bytes: &[u8]) -> Result<String, String> {
    if let Some(mime) = sniff_image_mime(bytes) {
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(format!("data:{mime};base64,{b64}"));
    }
    // HEIC / BMP / etc. — re-encode so DeepSeek / OpenAI accept the payload.
    use image::codecs::jpeg::JpegEncoder;
    use image::ImageReader;
    use std::io::Cursor;
    let decoded = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("unrecognized image format: {error}"))?
        .decode()
        .map_err(|error| format!("failed to decode image: {error}"))?;
    let rgb = decoded.to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 85)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| format!("jpeg encode failed: {error}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

fn provider_has_model(provider: &CustomProviderConfig, model: &str) -> bool {
    provider
        .models
        .split([',', '\n'])
        .map(str::trim)
        .any(|id| !id.is_empty() && id == model)
}

fn provider_is_configured(provider: &CustomProviderConfig) -> bool {
    let url = provider.base_url.trim();
    let key = provider.api_key.trim();
    !url.is_empty()
        && !key.is_empty()
        && (url.starts_with("http://") || url.starts_with("https://"))
}

fn image_provider_for_selection<'a>(
    settings: &'a AppSettings,
    model: &str,
    provider_hint: &str,
) -> Option<&'a CustomProviderConfig> {
    if !provider_hint.is_empty() {
        if let Some(provider) = settings
            .image_providers
            .iter()
            .find(|provider| provider.id == provider_hint)
        {
            if provider_is_configured(provider) || provider_has_model(provider, model) {
                return Some(provider);
            }
        }
        return None;
    }

    settings
        .image_providers
        .iter()
        .find(|provider| provider_has_model(provider, model))
}

fn resolve_image_endpoint(
    settings: &AppSettings,
    model: &str,
    provider_hint: &str,
) -> Result<ImageEndpoint, String> {
    let model = if model.trim().is_empty() {
        DEFAULT_IMAGE_MODEL
    } else {
        model.trim()
    };
    let Some(provider) = image_provider_for_selection(settings, model, provider_hint) else {
        if provider_hint.trim().is_empty() {
            return Err(format!(
                "Image model \"{model}\" is not configured. Add an Images provider under Settings → Image (official Base URL: https://api.openai.com/v1), then pick gpt-image-2 there. Chat providers are not used for image generation."
            ));
        }
        return Err(format!(
            "Image provider \"{}\" was not found. Add it under Settings → Image, not Provider.",
            provider_hint.trim()
        ));
    };
    if provider_model_is_disabled(provider, model) {
        return Err(format!(
            "Image model \"{model}\" is disabled in Settings → Image. Re-enable it on the Images provider first."
        ));
    }
    let api_key = provider.api_key.trim();
    let base_url = provider.base_url.trim();
    if api_key.is_empty() || base_url.is_empty() {
        return Err(
            "API Key / Base URL for the Images provider is not configured. Set it in Settings → Image."
                .into(),
        );
    }
    Ok(ImageEndpoint {
        api_key: api_key.to_string(),
        url: normalize_images_generations_url(base_url),
    })
}

/// Blocking Images API call. Must not run on a Tokio worker — the caller
/// should wrap this in `runtime::isolated::run_isolated`.
///
/// Hosts such as gpt-image aggregators only accept `n=1`. A requested count
/// greater than 1 is issued as that many parallel single-image requests.
pub fn generate_images_blocking(
    settings: &AppSettings,
    request: &ImageGenRequest,
) -> Result<Vec<GeneratedImage>, String> {
    let count = normalize_count(request.n as u64) as usize;
    if count <= 1 {
        return generate_one_blocking(settings, request);
    }

    let mut results = Vec::with_capacity(count);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..count)
            .map(|_| scope.spawn(|| generate_one_blocking(settings, request)))
            .collect();
        for handle in handles {
            results.push(handle.join().unwrap_or_else(|_| {
                Err("image request thread panicked".into())
            }));
        }
    });
    collect_image_results(results)
}

fn generate_one_blocking(
    settings: &AppSettings,
    request: &ImageGenRequest,
) -> Result<Vec<GeneratedImage>, String> {
    let model = if request.model.trim().is_empty() {
        DEFAULT_IMAGE_MODEL.to_string()
    } else {
        request.model.trim().to_string()
    };
    let endpoint = resolve_image_endpoint(settings, &model, &settings.image_model_provider)?;
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err("prompt is required".into());
    }

    let client = images_http_client()?;
    let (post_url, response) = if request.reference_images.is_empty() {
        let response = client
            .post(&endpoint.url)
            .header("Authorization", format!("Bearer {}", endpoint.api_key))
            .header("Content-Type", "application/json")
            .json(&build_images_body(&model, prompt, request))
            .send()
            .map_err(|error| images_transport_error(&endpoint.url, &error))?;
        (endpoint.url.clone(), response)
    } else {
        let edits_url = images_edits_url(&endpoint.url);
        let form = build_images_edits_form(&model, prompt, request)?;
        let response = client
            .post(&edits_url)
            .header("Authorization", format!("Bearer {}", endpoint.api_key))
            .multipart(form)
            .send()
            .map_err(|error| images_transport_error(&edits_url, &error))?;
        (edits_url, response)
    };

    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("Failed to read image API response: {error}"))?;
    if !status.is_success() {
        return Err(images_http_error(status.as_u16(), &post_url, &text));
    }
    parse_images_response(&text)
}

fn collect_image_results(
    results: Vec<Result<Vec<GeneratedImage>, String>>,
) -> Result<Vec<GeneratedImage>, String> {
    let mut images = Vec::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(batch) => images.extend(batch),
            Err(error) => errors.push(error),
        }
    }
    if images.is_empty() {
        return Err(errors.into_iter().next().unwrap_or_else(|| {
            "image provider returned no images".into()
        }));
    }
    if !errors.is_empty() {
        tracing::warn!(
            succeeded = images.len(),
            failed = errors.len(),
            error = %errors[0],
            "some image generation requests failed"
        );
    }
    Ok(images)
}

/// Official OpenAI Images body for gpt-image-2 (`POST /v1/images/generations`).
/// GPT Image models return `data[].b64_json` by default; aggregators may still
/// send `data[].url`.
pub fn build_images_body(model: &str, prompt: &str, request: &ImageGenRequest) -> Value {
    json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": normalize_size(&request.size),
        "quality": normalize_quality(&request.quality),
    })
}

fn images_edits_url(generations_url: &str) -> String {
    if let Some(prefix) = generations_url.strip_suffix("/images/generations") {
        format!("{prefix}/images/edits")
    } else {
        generations_url.replace("/generations", "/edits")
    }
}

fn build_images_edits_form(
    model: &str,
    prompt: &str,
    request: &ImageGenRequest,
) -> Result<reqwest::blocking::multipart::Form, String> {
    let mut form = reqwest::blocking::multipart::Form::new()
        .text("model", model.to_string())
        .text("prompt", prompt.to_string())
        .text("n", "1")
        .text("size", normalize_size(&request.size))
        .text("quality", normalize_quality(&request.quality));
    let field = if request.reference_images.len() > 1 {
        "image[]"
    } else {
        "image"
    };
    for (index, bytes) in request.reference_images.iter().enumerate() {
        let (mime, file_name) = image_part_meta(bytes, index);
        let part = reqwest::blocking::multipart::Part::bytes(bytes.clone())
            .file_name(file_name)
            .mime_str(mime)
            .map_err(|error| format!("invalid reference image: {error}"))?;
        form = form.part(field, part);
    }
    Ok(form)
}

fn image_part_meta(bytes: &[u8], index: usize) -> (&'static str, String) {
    match sniff_image_mime(bytes) {
        Some("image/png") => ("image/png", format!("reference-{index}.png")),
        Some("image/jpeg") => ("image/jpeg", format!("reference-{index}.jpg")),
        Some("image/webp") => ("image/webp", format!("reference-{index}.webp")),
        Some("image/gif") => ("image/gif", format!("reference-{index}.gif")),
        _ => ("image/png", format!("reference-{index}.png")),
    }
}

pub fn parse_images_response(body: &str) -> Result<Vec<GeneratedImage>, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        let snippet = truncate(body.trim(), 240);
        format!("Image API returned non-JSON. {error} Snippet: {snippet}")
    })?;
    if let Some(err) = extract_error_message(&value) {
        return Err(format!("Image API returned an error: {err}"));
    }
    let Some(items) = value.get("data").and_then(Value::as_array) else {
        return Err(format!(
            "Image API response missing data[]. Snippet: {}",
            truncate(body.trim(), 240)
        ));
    };
    if items.is_empty() {
        return Err("Image API returned no images.".into());
    }

    let mut images = Vec::new();
    for item in items {
        let revised = item
            .get("revised_prompt")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(b64) = item.get("b64_json").and_then(Value::as_str) {
            let bytes = decode_b64(b64)?;
            images.push(GeneratedImage {
                bytes,
                extension: "png".into(),
                revised_prompt: revised,
            });
            continue;
        }
        if let Some(url) = item.get("url").and_then(Value::as_str) {
            let bytes = download_image_url(url)?;
            images.push(GeneratedImage {
                bytes,
                extension: "png".into(),
                revised_prompt: revised,
            });
            continue;
        }
        return Err("Image API item had neither b64_json nor url.".into());
    }
    Ok(images)
}

fn decode_b64(value: &str) -> Result<Vec<u8>, String> {
    let payload = value.trim().split(',').next_back().unwrap_or(value).trim();
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .map_err(|error| format!("invalid image base64: {error}"))
}

fn images_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        // Match chat/multimodal: HTTP/1.1 + system proxy on Windows (Clash/V2Ray).
        .http1_only()
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

fn download_image_url(url: &str) -> Result<Vec<u8>, String> {
    let client = images_http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Failed to download generated image from {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download generated image from {url} (HTTP {}).",
            response.status().as_u16()
        ));
    }
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("Failed to read generated image bytes: {error}"))
}

fn extract_error_message(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    if let Some(message) = error.get("message").and_then(Value::as_str) {
        let trimmed = message.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(message) = error.as_str() {
        let trimmed = message.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn format_reqwest_error_chain(error: &reqwest::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = std::error::Error::source(error);
    while let Some(err) = source {
        let text = err.to_string();
        if parts.last().is_none_or(|last| last != &text) {
            parts.push(text);
        }
        source = err.source();
    }
    parts.join(" | ")
}

fn images_transport_error(url: &str, error: &reqwest::Error) -> String {
    let detail = format_reqwest_error_chain(error);
    let lower = detail.to_ascii_lowercase();
    let reason = if lower.contains("timed out") || lower.contains("timeout") {
        "Connection timed out: the image provider did not respond in time. Image generation can take over a minute."
    } else if lower.contains("dns") || lower.contains("no such host") || lower.contains("name resolution") {
        "DNS resolution failed: could not resolve the image provider host."
    } else if lower.contains("connection refused") {
        "Connection refused: the image provider address is unreachable."
    } else {
        "Could not reach the Images API. Chat hosts often do not serve /v1/images/generations. Settings → Image Base URL should be an Images API (official example: https://api.openai.com/v1). If you use Clash/V2Ray, enable system proxy and include Anya in proxy rules."
    };
    if url.trim().is_empty() {
        format!("{reason} Details: {detail}")
    } else {
        format!("{reason} POST {url}. Details: {detail}")
    }
}

pub fn images_http_error(code: u16, url: &str, body: &str) -> String {
    let reason = match code {
        401 | 403 => {
            "Authentication failed for this Images host. Settings → Image Base URL should be the Images API (official example: https://api.openai.com/v1), not a chat host."
        }
        400 => {
            "The image request was rejected. Check the prompt (moderation), and that width and height are multiples of 16. This endpoint may not support gpt-image-2."
        }
        404 => {
            "Endpoint or model name is incorrect: the provider may not expose `/v1/images/generations` or gpt-image-2."
        }
        429 => "Rate limited or quota exceeded. Retry later, or check the provider billing quota.",
        500..=599 => "Image provider server error. Usually temporary — retry later.",
        _ => "Image API call failed.",
    };
    let prefix = if url.trim().is_empty() {
        format!("Image API returned {code}. {reason}")
    } else {
        format!("Image API returned {code} from {url}. {reason}")
    };
    let detail = body.trim();
    if detail.is_empty() {
        prefix
    } else {
        format!("{prefix} Response: {}", truncate(detail, 280))
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{AppSettings, CustomProviderConfig};

    fn provider(id: &str, models: &str) -> CustomProviderConfig {
        CustomProviderConfig {
            id: id.into(),
            name: id.into(),
            base_url: format!("https://{id}.example/v1"),
            api_key: "sk-test".into(),
            models: models.into(),
            disabled_models: String::new(),
            preset_id: None,
            api_protocol: Default::default(),
            model_protocols: Default::default(),
        }
    }

    #[test]
    fn quality_and_size_fall_back_to_safe_defaults() {
        assert_eq!(normalize_quality("HIGH"), "high");
        assert_eq!(normalize_quality("weird"), "auto");
        assert_eq!(normalize_size("1536x1024"), "1536x1024");
        assert_eq!(normalize_size("2048x878"), "2048x880");
        assert_eq!(normalize_size("1600x900"), "1600x896");
        assert_eq!(normalize_size("auto"), "auto");
        assert_eq!(normalize_size("nope"), "1024x1024");
        assert_eq!(normalize_count(0), 1);
        assert_eq!(normalize_count(99), 4);
    }

    #[test]
    fn resolve_image_url_expands_path_prefix() {
        let dir = std::env::temp_dir().join(format!("anya-img-ref-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ref.png");
        // Minimal PNG header + IHDR/IDAT/IEND is overkill; write a JPEG so sniff works.
        let jpeg = {
            // 1x1 JPEG
            let png = image::RgbImage::from_pixel(1, 1, image::Rgb([10, 20, 30]));
            let mut bytes = Vec::new();
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90);
            enc.encode(png.as_raw(), 1, 1, image::ExtendedColorType::Rgb8)
                .unwrap();
            bytes
        };
        std::fs::write(&file, &jpeg).unwrap();
        let raw = format!("path:{}", file.to_string_lossy());
        let url = resolve_image_url_for_api(&raw).unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"), "{url}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_image_url_keeps_https() {
        let url = resolve_image_url_for_api("https://cdn.example/a.png").unwrap();
        assert_eq!(url, "https://cdn.example/a.png");
    }

    #[test]
    fn resolve_uses_image_providers_not_chat_providers() {
        let settings = AppSettings {
            custom_providers: vec![provider("chat", "gpt-image-2, gpt-4o")],
            image_providers: vec![provider("images", "gpt-image-2")],
            image_model_provider: "images".into(),
            ..Default::default()
        };
        let endpoint = resolve_image_endpoint(&settings, "gpt-image-2", "images").unwrap();
        assert_eq!(endpoint.url, "https://images.example/v1/images/generations");
    }

    #[test]
    fn resolve_ignores_chat_custom_providers() {
        let settings = AppSettings {
            custom_providers: vec![provider("openai", "gpt-image-2, gpt-4o")],
            image_model_provider: "openai".into(),
            ..Default::default()
        };
        let error = resolve_image_endpoint(&settings, "gpt-image-2", "openai").unwrap_err();
        assert!(error.contains("Settings → Image"));
        assert!(!error.to_ascii_lowercase().contains("organization"));
    }

    #[test]
    fn resolve_matches_model_on_image_provider_list() {
        let settings = AppSettings {
            image_providers: vec![provider("images", "gpt-image-2")],
            ..Default::default()
        };
        let endpoint = resolve_image_endpoint(&settings, "gpt-image-2", "").unwrap();
        assert!(endpoint.url.ends_with("/images/generations"));
    }

    #[test]
    fn parse_b64_json_payload() {
        let png = base64::engine::general_purpose::STANDARD.encode([0x89, b'P', b'N', b'G']);
        let body =
            format!(r#"{{"data":[{{"b64_json":"{png}","revised_prompt":"a clearer cat"}}]}}"#);
        let images = parse_images_response(&body).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].bytes, [0x89, b'P', b'N', b'G']);
        assert_eq!(images[0].revised_prompt.as_deref(), Some("a clearer cat"));
    }

    #[test]
    fn images_body_matches_official_gpt_image() {
        let body = build_images_body(
            "gpt-image-2",
            "a cat",
            &ImageGenRequest {
                prompt: "a cat".into(),
                model: "gpt-image-2".into(),
                size: "1536x1024".into(),
                quality: "auto".into(),
                n: 1,
                ..Default::default()
            },
        );
        assert_eq!(body["model"], "gpt-image-2");
        assert_eq!(body["prompt"], "a cat");
        assert_eq!(body["size"], "1536x1024");
        assert_eq!(body["quality"], "auto");
        assert_eq!(body["n"], 1);
        assert!(body.get("response_format").is_none());
        assert!(body.get("watermark").is_none());
        assert!(body.get("output_format").is_none());
    }

    #[test]
    fn collect_keeps_successful_images_when_some_requests_fail() {
        let ok = GeneratedImage {
            bytes: vec![1, 2, 3],
            extension: "png".into(),
            revised_prompt: None,
        };
        let images = collect_image_results(vec![
            Ok(vec![ok.clone()]),
            Err("rate limited".into()),
            Ok(vec![ok.clone()]),
        ])
        .unwrap();
        assert_eq!(images.len(), 2);
        let error = collect_image_results(vec![Err("boom".into()), Err("nope".into())]).unwrap_err();
        assert_eq!(error, "boom");
    }

    #[test]
    fn images_body_always_requests_a_single_image() {
        let body = build_images_body(
            "gpt-image-2",
            "a cat",
            &ImageGenRequest {
                prompt: "a cat".into(),
                n: 4,
                ..Default::default()
            },
        );
        assert_eq!(body["n"], 1);
    }

    #[test]
    fn http_error_includes_post_url_and_images_host_hint() {
        let message = images_http_error(
            401,
            "https://api.example/v1/images/generations",
            r#"{"error":{"message":"invalid api key"}}"#,
        );
        assert!(message.contains("401"));
        assert!(message.contains("https://api.example/v1/images/generations"));
        assert!(message.contains("api.openai.com"));
        assert!(message.contains("invalid api key"));
        assert!(
            !message
                .to_ascii_lowercase()
                .contains("organization verification")
        );
    }

    #[test]
    fn edits_url_replaces_generations_suffix() {
        assert_eq!(
            images_edits_url("https://api.openai.com/v1/images/generations"),
            "https://api.openai.com/v1/images/edits"
        );
    }
}
