use std::time::Duration;

use serde_json::json;

use crate::models::settings::{AppSettings, ProviderApiProtocol};

use super::{normalize_chat_completions_url, ProviderError, RETRY_BACKOFF};
fn load_image_as_base64(path_or_data: &str) -> Result<String, String> {
    crate::core::ai::image_gen::resolve_image_url_for_api(path_or_data)
}

#[derive(Debug)]
pub(super) struct MultimodalEndpoint {
    pub(super) api_key: String,
    pub(super) url: String,
    pub(super) base_url: String,
    pub(super) protocol: ProviderApiProtocol,
}

fn resolve_deepseek_multimodal_endpoint(
    settings: &AppSettings,
    mm_model: &str,
) -> Option<MultimodalEndpoint> {
    if !crate::core::ai::registry::looks_like_deepseek_model(mm_model) {
        return None;
    }
    let api_key = settings.deepseek_api_key.trim();
    if api_key.is_empty() {
        return None;
    }
    let base_url = "https://api.deepseek.com";
    Some(MultimodalEndpoint {
        api_key: api_key.to_string(),
        url: super::normalize_chat_completions_url(base_url),
        base_url: base_url.to_string(),
        protocol: ProviderApiProtocol::ChatCompletions,
    })
}

pub(super) fn resolve_multimodal_endpoint(
    settings: &AppSettings,
    mm_model: &str,
    provider_hint: &str,
) -> Result<MultimodalEndpoint, ProviderError> {
    let hint = provider_hint.trim();
    if hint.is_empty() || hint == "deepseek" {
        if let Some(endpoint) = resolve_deepseek_multimodal_endpoint(settings, mm_model) {
            return Ok(endpoint);
        }
        if hint == "deepseek" {
            return Err(ProviderError::message(
                "DeepSeek API Key is not configured. Please enter it in Settings.",
            ));
        }
    }

    for custom in &settings.custom_providers {
        if !hint.is_empty() && custom.id != hint {
            continue;
        }
        let custom_ids: Vec<&str> = custom
            .models
            .split([',', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let hint_matches = !hint.is_empty() && custom.id == hint;
        let configured = !custom.api_key.trim().is_empty() && !custom.base_url.trim().is_empty();
        if !hint_matches && !custom_ids.contains(&mm_model) {
            continue;
        }
        if crate::core::ai::registry::provider_model_is_disabled(custom, mm_model) {
            return Err(ProviderError::message(format!(
                "模型 “{mm_model}” 已在设置中被禁用，请先在供应商设置里重新启用后再使用。"
            )));
        }
        if !configured {
            if hint_matches {
                return Err(ProviderError::message(
                    "API Key / Base URL for the multimodal provider is not configured. Please set it in Settings.",
                ));
            }
            continue;
        }
        if custom.api_key.trim().is_empty() {
            return Err(ProviderError::message(
                "API Key for the multimodal provider is not configured. Please set it in Settings.",
            ));
        }
        if custom.base_url.trim().is_empty() {
            return Err(ProviderError::message(
                "Base URL for the multimodal provider is not configured. Please set it in Settings.",
            ));
        }
        return Ok(MultimodalEndpoint {
            api_key: custom.api_key.trim().to_string(),
            url: normalize_chat_completions_url(&custom.base_url),
            base_url: custom.base_url.trim().to_string(),
            protocol: custom.api_protocol,
        });
    }

    Err(ProviderError::message(format!(
        "Multimodal model \"{mm_model}\" is not configured under any custom provider. Add a vision-capable provider (Base URL, API Key, and model name) in Settings, or disable multimodal split analysis."
    )))
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

pub(crate) fn multimodal_transport_error_message(error: &reqwest::Error) -> String {
    let detail = format_reqwest_error_chain(error);
    let lower = detail.to_lowercase();

    let reason = if lower.contains("timed out") || lower.contains("timeout") {
        "Connection timed out: the multimodal provider did not respond, or the network/proxy is too slow."
    } else if lower.contains("dns")
        || lower.contains("name resolution")
        || lower.contains("no such host")
    {
        "DNS resolution failed: could not resolve the multimodal provider host. Check network or DNS settings."
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        "TLS/certificate verification failed: check system time, or whether a proxy is intercepting the certificate."
    } else if lower.contains("connection refused") {
        "Connection refused: the provider address is unreachable, or the local proxy port is not running."
    } else if lower.contains("error sending request") {
        "Could not establish a network connection to the multimodal provider. If you use Clash/V2Ray (especially fake-ip mode), enable the system proxy or add Anya to proxy rules and retry."
    } else {
        "The network request could not be sent. Check network, system proxy, and the multimodal Base URL."
    };

    format!("{reason} Details: {detail}")
}

pub(crate) fn multimodal_http_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let code = status.as_u16();
    let reason = match code {
        401 | 403 => "Authentication failed: the API Key is invalid, or it cannot call this vision model.",
        404 => "Endpoint or model name is incorrect: check the custom provider Base URL and multimodal model name.",
        413 => "Request body too large: the image exceeds the provider limit. Try a smaller image.",
        429 => "Rate limited or quota exceeded: retry later, or check the provider quota.",
        500 => "Vision model internal error: usually a temporary upstream failure. Retry later.",
        502 => "Bad gateway (502): the multimodal proxy/upstream did not respond correctly. Common causes include an oversized image, temporary upstream outage, or a wrong Base URL/proxy setup.",
        503 => "Service unavailable (503): the upstream vision service is overloaded or under maintenance. Retry later.",
        504 => "Gateway timeout (504): image analysis took too long or the upstream did not respond. Retry later or use a smaller image.",
        _ if status.is_client_error() => {
            "Request rejected by the provider: check the multimodal model name, Base URL, and whether the image format is supported."
        }
        _ if status.is_server_error() => {
            "Vision model server error: retry later, or switch multimodal providers."
        }
        _ => "Multimodal API call failed.",
    };

    let detail = body.trim();
    if detail.is_empty() || detail == "unknown error" {
        format!("Multimodal API returned {code}. {reason}")
    } else {
        let truncated = if detail.len() > 240 {
            format!("{}…", &detail[..240])
        } else {
            detail.to_string()
        };
        format!("Multimodal API returned {code}. {reason} Response: {truncated}")
    }
}

const MAX_MULTIMODAL_RETRIES: u32 = 2;
const MULTIMODAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const MULTIMODAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MULTIMODAL_IMAGE_PROMPT: &str = "You are a professional visual analyst. Provide a detailed, structured description of this image covering: (1) all visible text transcribed via precise OCR; (2) primary subjects and scene content; (3) charts, diagrams, or layout structure; (4) key information; and (5) color palette and visual style. Output the analysis directly—no preamble, greetings, or closing remarks.";

async fn request_multimodal_image_description(
    client: &reqwest::Client,
    endpoint: &MultimodalEndpoint,
    mm_model: &str,
    b64_url: &str,
    stream: bool,
) -> Result<String, ProviderError> {
    let mut body = json!({
        "model": mm_model,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": b64_url
                        }
                    },
                    {
                        "type": "text",
                        "text": MULTIMODAL_IMAGE_PROMPT
                    }
                ]
            }
        ],
        "stream": stream
    });

    if wants_max_completion_tokens(mm_model) {
        body.as_object_mut()
            .expect("request object")
            .insert("max_completion_tokens".into(), json!(4096));
    } else if wants_max_tokens(mm_model) {
        body.as_object_mut()
            .expect("request object")
            .insert("max_tokens".into(), json!(4096));
    }

    let response = client
        .post(&endpoint.url)
        .header("Authorization", format!("Bearer {}", endpoint.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .timeout(MULTIMODAL_REQUEST_TIMEOUT)
        .json(&body)
        .send()
        .await
        .map_err(|error| ProviderError::message(multimodal_transport_error_message(&error)))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let response_text = read_multimodal_response_text(response).await?;

    if !status.is_success() {
        return Err(ProviderError::message(multimodal_http_error_message(
            status,
            &response_text,
        )));
    }

    if content_type.to_ascii_lowercase().contains("text/html")
        || crate::core::ai::multimodal_response::body_looks_like_html(&response_text)
    {
        return Err(ProviderError::message(
            crate::core::ai::multimodal_response::html_instead_of_json_error(
                &response_text,
                Some(&endpoint.url),
                Some(&content_type),
            ),
        ));
    }

    crate::core::ai::multimodal_response::parse_multimodal_description_body(&response_text)
        .map_err(ProviderError::message)
}

pub(super) fn multimodal_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(MULTIMODAL_CONNECT_TIMEOUT)
        .timeout(MULTIMODAL_REQUEST_TIMEOUT)
        // Match Antigravity: HTTP/1.1 + system proxy avoid flaky HTTP/2 / proxy body reads
        // that surface as "error decoding response body".
        .http1_only()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn read_multimodal_response_text(
    response: reqwest::Response,
) -> Result<String, ProviderError> {
    let bytes = response.bytes().await.map_err(|error| {
        ProviderError::message(format!(
            "Failed to read multimodal response: {}. If the multimodal model is reached via a proxy (Clash/V2Ray), enable the system proxy or switch multimodal analysis to a Gemini model with Antigravity login.",
            format_reqwest_error_chain(&error)
        ))
    })?;
    Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| {
        // Full body should be UTF-8 JSON; fall back without inventing CJK mojibake.
        String::from_utf8_lossy(&bytes).into_owned()
    }))
}

/// Use Antigravity only when the *configured multimodal model* is Gemini + OAuth.
/// (Split analysis itself should not run for Gemini chat primaries.)
pub(super) fn antigravity_model_for_image_describe(
    settings: &AppSettings,
    mm_model: &str,
) -> Option<String> {
    let provider = settings.multimodal_model_provider.trim();
    if (provider.is_empty() || provider == "gemini")
        && crate::services::gemini_oauth::can_use_antigravity_for_model(settings, mm_model)
    {
        Some(mm_model.to_string())
    } else {
        None
    }
}

pub(super) fn should_retry_multimodal_as_stream(error: &ProviderError) -> bool {
    match error {
        ProviderError::Cancelled => false,
        ProviderError::Message(message) => {
            let lower = message.to_ascii_lowercase();
            // Retry stream only for empty/unparseable JSON bodies — not transport/decode failures.
            (lower.contains("failed to extract")
                || lower.contains("empty")
                || lower.contains("parse")
                || lower.contains("snippet"))
                && !lower.contains("failed to read multimodal response")
                && !lower.contains("timed out")
                && !lower.contains("connection")
                && !lower.contains("html")
        }
    }
}

fn wants_max_completion_tokens(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.contains("gpt-5")
}

fn wants_max_tokens(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.contains("gpt-4") || lower.contains("gpt-3.5") || lower.contains("chatgpt")
}

pub(super) async fn describe_image(
    client: &reqwest::Client,
    app: &tauri::AppHandle,
    image_payload: &str,
) -> Result<String, ProviderError> {
    let settings = crate::services::settings_store::get_settings(app).unwrap_or_default();
    let mm_model = if settings.multimodal_model.trim().is_empty() {
        "gpt-4o".to_string()
    } else {
        settings.multimodal_model.trim().to_string()
    };

    if let Some(ag_model) = antigravity_model_for_image_describe(&settings, &mm_model) {
        return crate::core::ai::antigravity::describe_image_via_antigravity(
            app,
            &ag_model,
            image_payload,
        )
        .await;
    }

    let endpoint = resolve_multimodal_endpoint(
        &settings,
        &mm_model,
        settings.multimodal_model_provider.trim(),
    )?;

    let b64_url = load_image_as_base64(image_payload)
        .map_err(|e| ProviderError::message(format!("Failed to load image: {e}")))?;

    let mut last_error = ProviderError::message("Multimodal model call failed");

    for attempt in 0..MAX_MULTIMODAL_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(RETRY_BACKOFF * attempt).await;
        }

        match request_multimodal_image_description(client, &endpoint, &mm_model, &b64_url, false)
            .await
        {
            Ok(description) => return Ok(description),
            Err(error) if should_retry_multimodal_as_stream(&error) => {
                match request_multimodal_image_description(
                    client, &endpoint, &mm_model, &b64_url, true,
                )
                .await
                {
                    Ok(description) => return Ok(description),
                    Err(stream_error) => last_error = stream_error,
                }
            }
            Err(error) => last_error = error,
        }
    }

    Err(last_error)
}
