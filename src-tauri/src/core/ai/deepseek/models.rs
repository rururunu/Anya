use std::time::Duration;

use serde_json::Value;

use crate::models::chat::{ChatModelInfo, ModelReasoningInfo};
use crate::models::settings::ProviderApiProtocol;

use super::ProviderError;

const MODELS_URL: &str = "https://api.deepseek.com/models";
const MODELS_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MODELS_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

struct ParsedModelItem {
    id: String,
    owned_by: Option<String>,
    reasoning: Option<ModelReasoningInfo>,
}

/// Best-effort extraction of a model id/owner from one entry of a models
/// listing payload. Tolerates the handful of shapes real-world
/// OpenAI-compatible gateways return beyond the canonical `{id, owned_by}`.
fn parse_model_item(value: &Value) -> Option<ParsedModelItem> {
    match value {
        Value::String(id) => {
            let id = id.trim();
            if id.is_empty() {
                None
            } else {
                Some(ParsedModelItem {
                    id: id.to_string(),
                    owned_by: None,
                    reasoning: None,
                })
            }
        }
        Value::Object(map) => {
            let id = map
                .get("id")
                .or_else(|| map.get("model"))
                .or_else(|| map.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())?;
            let owned_by = map
                .get("owned_by")
                .or_else(|| map.get("ownedBy"))
                .or_else(|| map.get("owner"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            Some(ParsedModelItem {
                id: id.to_string(),
                owned_by,
                reasoning: extract_reasoning_info(map),
            })
        }
        _ => None,
    }
}

fn json_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .collect(),
        Some(Value::String(item)) => {
            let item = item.trim().to_ascii_lowercase();
            if item.is_empty() {
                Vec::new()
            } else {
                vec![item]
            }
        }
        _ => Vec::new(),
    }
}

fn json_bool(value: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(flag)) => Some(*flag),
        Some(Value::String(text)) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        Some(Value::Number(number)) => number.as_i64().map(|n| n != 0),
        _ => None,
    }
}

const REASONING_PARAM_KEYS: &[&str] = &[
    "reasoning",
    "reasoning_effort",
    "reasoningeffort",
    "include_reasoning",
    "includereasoning",
    "thinking",
    "enable_thinking",
    "enablethinking",
    "thought",
];

fn list_has_reasoning_param(params: &[String]) -> bool {
    params.iter().any(|param| {
        let normalized = param.replace('-', "_");
        REASONING_PARAM_KEYS
            .iter()
            .any(|key| normalized == *key || normalized.contains(key))
    })
}

fn list_has_effort_param(params: &[String]) -> bool {
    params.iter().any(|param| {
        let normalized = param.replace('-', "_");
        normalized.contains("reasoning_effort") || normalized == "thinking_level"
    })
}

/// Read advertised reasoning/thinking support from a `/models` object.
/// Returns `None` when the listing did not say anything usable.
fn extract_reasoning_info(map: &serde_json::Map<String, Value>) -> Option<ModelReasoningInfo> {
    let supported_parameters =
        json_string_list(map.get("supported_parameters").or(map.get("supportedParameters")));
    let parameter_lists = [
        supported_parameters.clone(),
        json_string_list(map.get("supported_sampling_parameters")),
        json_string_list(map.get("features")),
        json_string_list(map.get("capabilities")),
        json_string_list(map.get("supported_features")),
    ];
    let params_support = parameter_lists.iter().any(|list| list_has_reasoning_param(list));
    let params_effort = parameter_lists.iter().any(|list| list_has_effort_param(list));

    let explicit = json_bool(map.get("supports_reasoning"))
        .or_else(|| json_bool(map.get("support_reasoning")))
        .or_else(|| json_bool(map.get("supports_thinking")))
        .or_else(|| json_bool(map.get("support_thinking")))
        .or_else(|| json_bool(map.get("thinking")))
        .or_else(|| json_bool(map.get("reasoning")));

    match (explicit, params_support, supported_parameters.is_empty()) {
        (Some(false), _, _) => Some(ModelReasoningInfo {
            supported: false,
            can_disable: None,
        }),
        (Some(true), _, _) => Some(ModelReasoningInfo {
            supported: true,
            can_disable: if params_effort { Some(true) } else { None },
        }),
        (None, true, _) => Some(ModelReasoningInfo {
            supported: true,
            can_disable: if params_effort { Some(true) } else { None },
        }),
        // OpenRouter-style: a non-empty supported_parameters list that omits
        // reasoning keys is a real negative, not "unknown".
        (None, false, false) => Some(ModelReasoningInfo {
            supported: false,
            can_disable: None,
        }),
        _ => None,
    }
}

/// Extract the list of model entries from a models-listing JSON payload.
/// Accepts the canonical OpenAI `{"data": [...]}` shape as well as common
/// variants (`{"models": [...]}`, `{"result": [...]}`, or a bare array).
fn extract_model_items(payload: &Value) -> Option<Vec<ParsedModelItem>> {
    let array = match payload {
        Value::Array(items) => items,
        Value::Object(map) => map
            .get("data")
            .or_else(|| map.get("models"))
            .or_else(|| map.get("result"))
            .and_then(Value::as_array)?,
        _ => return None,
    };
    Some(array.iter().filter_map(parse_model_item).collect())
}

fn models_http_client() -> Result<reqwest::Client, ProviderError> {
    reqwest::Client::builder()
        .connect_timeout(MODELS_CONNECT_TIMEOUT)
        .timeout(MODELS_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| ProviderError::message(format!("failed to build HTTP client: {error}")))
}

pub async fn list_models(api_key: &str) -> Result<Vec<ChatModelInfo>, ProviderError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(ProviderError::message(
            "DeepSeek API Key is not configured. Please enter it in Settings.",
        ));
    }

    let models = list_openai_compatible_models(MODELS_URL, api_key, "deepseek", None).await?;
    Ok(models)
}

/// `GET {base}/models` — OpenAI Models API compatible listing.
pub async fn list_openai_compatible_models(
    models_url: &str,
    api_key: &str,
    provider: &str,
    owned_by_fallback: Option<&str>,
) -> Result<Vec<ChatModelInfo>, ProviderError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(ProviderError::message("API Key is not configured."));
    }

    let client = models_http_client()?;
    let response = client
        .get(models_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        // Some gateways sit behind Cloudflare/WAF rules that block requests with no
        // browser-like User-Agent (reqwest sends none by default), returning a 403
        // challenge page instead of JSON. A realistic UA avoids that false positive.
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|error| {
            ProviderError::message(format!(
                "network error: {error}. Check the Base URL and network/proxy settings."
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        let looks_like_waf_html =
            text.contains("Cloudflare") || text.contains("cf-error") || text.contains("<!DOCTYPE");
        if looks_like_waf_html {
            return Err(ProviderError::message(format!(
                "API {status}: the gateway returned an HTML challenge page instead of JSON. \
This host is likely blocking automated requests (Cloudflare/WAF bot protection) rather than \
rejecting the API key — contact the provider or try a different Base URL."
            )));
        }
        return Err(ProviderError::message(format!("API {status}: {text}")));
    }

    let text = response
        .text()
        .await
        .map_err(|error| ProviderError::message(format!("failed to read response: {error}")))?;
    let payload: Value = serde_json::from_str(&text).map_err(|error| {
        ProviderError::message(format!(
            "invalid models payload: {error}. Response was not valid JSON."
        ))
    })?;
    let items = extract_model_items(&payload).ok_or_else(|| {
        ProviderError::message(
            "invalid models payload: expected a `data`/`models` array of model entries.",
        )
    })?;

    let fallback = owned_by_fallback.unwrap_or(provider);
    Ok(items
        .into_iter()
        .filter(|item| !item.id.trim().is_empty())
        .map(|item| ChatModelInfo {
            id: item.id,
            owned_by: item.owned_by.unwrap_or_else(|| fallback.to_string()),
            provider: provider.to_string(),
            display_name: None,
            thinking_variants: None,
            reasoning: item.reasoning,
        })
        .collect())
}

/// Normalize an OpenAI-compatible base URL to a models listing endpoint.
pub fn normalize_models_url(base_url: &str) -> String {
    format!("{}/models", openai_versioned_root(base_url))
}

/// Normalize an OpenAI-compatible base URL to a chat completions endpoint.
///
/// Bare hosts such as `https://www.micuapi.ai` (NewAPI) must become
/// `.../v1/chat/completions`, not `.../chat/completions`.
pub(crate) fn normalize_chat_completions_url(base_url: &str) -> String {
    format!("{}/chat/completions", openai_versioned_root(base_url))
}

/// xAI Chat Completions does not return reasoning text. Grok thinking is only
/// available on the Responses API (`/v1/responses`).
pub(crate) fn normalize_responses_url(base_url: &str) -> String {
    format!("{}/responses", openai_versioned_root(base_url))
}

/// Anthropic-compatible Messages API (`/v1/messages`). Aggregators such as
/// OpenCode Go host MiniMax here instead of `/v1/chat/completions`.
pub(crate) fn normalize_anthropic_messages_url(base_url: &str) -> String {
    let root = openai_versioned_root(base_url);
    if root.ends_with("/messages") {
        root
    } else {
        format!("{root}/messages")
    }
}

pub(crate) fn endpoint_url_for_protocol(base_url: &str, protocol: ProviderApiProtocol) -> String {
    match protocol {
        ProviderApiProtocol::Responses => normalize_responses_url(base_url),
        ProviderApiProtocol::ChatCompletions => normalize_chat_completions_url(base_url),
    }
}

fn openai_versioned_root(base_url: &str) -> String {
    let mut base = base_url.trim().trim_end_matches('/').to_string();
    if let Some(stripped) = base.strip_suffix("/chat/completions") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if let Some(stripped) = base.strip_suffix("/responses") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if let Some(stripped) = base.strip_suffix("/messages") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if let Some(stripped) = base.strip_suffix("/models") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if !has_versioned_api_path(&base) {
        base = format!("{base}/v1");
    }
    base
}

fn has_versioned_api_path(base: &str) -> bool {
    let path = url_path(base);
    if path.is_empty() || path == "/" {
        return false;
    }
    path == "/v1"
        || path.ends_with("/v1")
        || path.contains("/v1/")
        || path.contains("/v1beta")
        || path == "/v3"
        || path.ends_with("/v3")
        || path.contains("/v3/")
        || path == "/v4"
        || path.ends_with("/v4")
        || path.contains("/v4/")
}

fn url_path(base: &str) -> &str {
    let rest = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(base);
    match rest.find('/') {
        Some(index) => &rest[index..],
        None => "",
    }
}

#[cfg(test)]
mod reasoning_listing_tests {
    use super::{extract_model_items, extract_reasoning_info};
    use serde_json::json;

    #[test]
    fn openrouter_supported_parameters_marks_reasoning() {
        let map = json!({
            "supported_parameters": ["temperature", "reasoning", "reasoning_effort"]
        })
        .as_object()
        .cloned()
        .unwrap();
        let info = extract_reasoning_info(&map).expect("reasoning info");
        assert!(info.supported);
        assert_eq!(info.can_disable, Some(true));
    }

    #[test]
    fn openrouter_supported_parameters_without_reasoning_is_negative() {
        let map = json!({
            "supported_parameters": ["temperature", "max_tokens"]
        })
        .as_object()
        .cloned()
        .unwrap();
        let info = extract_reasoning_info(&map).expect("reasoning info");
        assert!(!info.supported);
    }

    #[test]
    fn explicit_supports_reasoning_flag() {
        let map = json!({ "supports_reasoning": true }).as_object().cloned().unwrap();
        let info = extract_reasoning_info(&map).expect("reasoning info");
        assert!(info.supported);
        assert_eq!(info.can_disable, None);
    }

    #[test]
    fn bare_openai_object_stays_unknown() {
        let map = json!({ "id": "gpt-4o", "owned_by": "openai" })
            .as_object()
            .cloned()
            .unwrap();
        assert!(extract_reasoning_info(&map).is_none());
    }

    #[test]
    fn extract_items_keeps_reasoning() {
        let payload = json!({
            "data": [
                {
                    "id": "claude-sonnet-4",
                    "supported_parameters": ["reasoning"]
                }
            ]
        });
        let items = extract_model_items(&payload).expect("items");
        assert_eq!(items[0].id, "claude-sonnet-4");
        assert_eq!(items[0].reasoning.as_ref().map(|info| info.supported), Some(true));
    }
}
