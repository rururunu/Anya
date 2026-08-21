use std::collections::HashMap;

use serde::Deserialize;

use crate::models::chat::{ChatModelInfo, ModelThinkingVariant};

use super::{
    antigravity_http_client, antigravity_http_error_message, antigravity_production_api_url,
    antigravity_transport_error_message, antigravity_user_agent, client_metadata_header,
    is_gemini_model, is_retryable_antigravity_status, ANTIGRAVITY_RETRY_BACKOFF,
    MAX_ANTIGRAVITY_RETRIES, X_GOOG_API_CLIENT,
};

/// Fallback when `fetchAvailableModels` is unreachable.
const GEMINI_DEFAULT_MODELS: &[&str] = &[
    "gemini-3-flash",
    "gemini-3-flash-agent",
    "gemini-3.1-pro-high",
    "gemini-3.1-pro-low",
    "gemini-pro-agent",
    "gemini-3.5-flash-low",
];

pub fn resolve_antigravity_model_id(model: &str) -> String {
    match model.trim() {
        "gemini-3.1-pro-high" => "gemini-pro-agent".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchAvailableModelsResponse {
    #[serde(default)]
    models: HashMap<String, FetchAvailableModelEntry>,
    #[serde(default)]
    default_agent_model_id: Option<String>,
    #[serde(default)]
    command_model_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchAvailableModelEntry {
    #[serde(default)]
    quota_info: Option<FetchAvailableQuotaInfo>,
    #[serde(default)]
    recommended: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchAvailableQuotaInfo {
    #[serde(default)]
    remaining_fraction: Option<f64>,
}

#[derive(Clone)]
struct ParsedCatalogModel {
    id: String,
    recommended: bool,
}

#[derive(Clone)]
pub(super) struct GroupedCatalogModel {
    family_key: String,
    default_variant_id: String,
    variants: Vec<ModelThinkingVariant>,
    recommended: bool,
}

pub(super) async fn fetch_available_models(
    access_token: &str,
    project_id: Option<&str>,
) -> Result<Vec<GroupedCatalogModel>, String> {
    let body = if let Some(project_id) = project_id.filter(|value| !value.trim().is_empty()) {
        serde_json::json!({ "project": project_id })
    } else {
        serde_json::json!({})
    };

    let client = antigravity_http_client()?;
    let url = antigravity_production_api_url("v1internal:fetchAvailableModels");
    let mut last_error = String::from("fetchAvailableModels request failed");

    for attempt in 0..MAX_ANTIGRAVITY_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(ANTIGRAVITY_RETRY_BACKOFF * attempt).await;
        }

        let response = match client
            .post(&url)
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, antigravity_user_agent())
            .header("x-goog-api-client", X_GOOG_API_CLIENT)
            .header("Client-Metadata", client_metadata_header())
            .json(&body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_error = antigravity_transport_error_message(&error);
                if attempt + 1 < MAX_ANTIGRAVITY_RETRIES {
                    continue;
                }
                return Err(last_error);
            }
        };

        let status = response.status();
        let text = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                last_error = format!("Failed to read fetchAvailableModels response: {error}");
                if attempt + 1 < MAX_ANTIGRAVITY_RETRIES {
                    continue;
                }
                return Err(last_error);
            }
        };
        if status.is_success() {
            let parsed: FetchAvailableModelsResponse =
                serde_json::from_str(&text).map_err(|error| {
                    format!("Failed to parse fetchAvailableModels response: {error}; body={text}")
                })?;
            return Ok(parse_gemini_catalog(&parsed));
        }

        last_error = antigravity_http_error_message(status, &text);
        if attempt + 1 < MAX_ANTIGRAVITY_RETRIES && is_retryable_antigravity_status(status) {
            continue;
        }
        return Err(last_error);
    }

    Err(last_error)
}

fn parse_gemini_catalog(response: &FetchAvailableModelsResponse) -> Vec<GroupedCatalogModel> {
    let mut order = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut push_id = |id: &str| {
        let trimmed = id.trim();
        if trimmed.is_empty() || !is_gemini_model(trimmed) {
            return;
        }
        if seen.insert(trimmed.to_string()) {
            order.push(trimmed.to_string());
        }
    };

    if let Some(default_id) = response.default_agent_model_id.as_deref() {
        push_id(default_id);
    }
    for id in &response.command_model_ids {
        push_id(id);
    }
    for id in response.models.keys() {
        push_id(id);
    }

    let mut models: Vec<ParsedCatalogModel> = order
        .into_iter()
        .filter_map(|id| {
            let entry = response.models.get(&id);
            let available = entry
                .and_then(|entry| entry.quota_info.as_ref())
                .and_then(|quota| quota.remaining_fraction)
                .map(|remaining| remaining > 0.0)
                .unwrap_or(true);
            if !available {
                return None;
            }
            Some(ParsedCatalogModel {
                id: id.clone(),
                recommended: entry.map(|entry| entry.recommended).unwrap_or(false),
            })
        })
        .collect();

    models.sort_by(|left, right| {
        right
            .recommended
            .cmp(&left.recommended)
            .then_with(|| left.id.cmp(&right.id))
    });
    group_gemini_thinking_variants(models, response.default_agent_model_id.as_deref())
}

/// Group high/low/agent tiers of the same Gemini family into one list entry.
fn group_gemini_thinking_variants(
    models: Vec<ParsedCatalogModel>,
    default_agent_model_id: Option<&str>,
) -> Vec<GroupedCatalogModel> {
    let mut by_family: HashMap<String, Vec<ParsedCatalogModel>> = HashMap::new();
    for model in models {
        let family = gemini_family_key(&model.id);
        by_family.entry(family).or_default().push(model);
    }

    let mut grouped: Vec<GroupedCatalogModel> = by_family
        .into_iter()
        .map(|(family_key, mut variants)| {
            variants.sort_by(|left, right| {
                thinking_tier_sort_key(&thinking_tier_label(&left.id))
                    .cmp(&thinking_tier_sort_key(&thinking_tier_label(&right.id)))
                    .then_with(|| left.id.cmp(&right.id))
            });
            variants.dedup_by(|left, right| left.id == right.id);

            let default_variant_id = variants
                .iter()
                .max_by_key(|model| variant_selection_score(model, default_agent_model_id))
                .map(|model| model.id.clone())
                .unwrap_or_else(|| family_key.clone());

            let thinking_variants = variants
                .into_iter()
                .map(|model| ModelThinkingVariant {
                    id: model.id.clone(),
                    label: thinking_tier_label(&model.id),
                    recommended: model.recommended,
                })
                .collect::<Vec<_>>();

            let recommended = thinking_variants.iter().any(|variant| variant.recommended);
            GroupedCatalogModel {
                family_key,
                default_variant_id,
                variants: thinking_variants,
                recommended,
            }
        })
        .collect();

    grouped.sort_by(|left, right| {
        right.recommended.cmp(&left.recommended).then_with(|| {
            prettify_gemini_family_display(&left.family_key)
                .cmp(&prettify_gemini_family_display(&right.family_key))
        })
    });
    grouped
}

fn thinking_tier_label(id: &str) -> String {
    let lower = id.trim().to_ascii_lowercase();
    if lower.ends_with("-agent") {
        "Agent".to_string()
    } else if lower.ends_with("-high") {
        "High".to_string()
    } else if lower.ends_with("-low") {
        "Low".to_string()
    } else {
        "Default".to_string()
    }
}

fn thinking_tier_sort_key(label: &str) -> i32 {
    match label {
        "Low" => 0,
        "Default" => 1,
        "High" => 2,
        "Agent" => 3,
        _ => 4,
    }
}

fn gemini_family_key(id: &str) -> String {
    let normalized = id.trim().to_ascii_lowercase();
    if normalized == "gemini-pro-agent" || normalized.starts_with("gemini-3.1-pro") {
        return "gemini-3.1-pro".to_string();
    }

    let rest = normalized
        .strip_prefix("gemini-")
        .unwrap_or(normalized.as_str());
    for suffix in ["-high", "-low", "-agent"] {
        if let Some(body) = rest.strip_suffix(suffix) {
            let body = body.trim_end_matches('-');
            if body.is_empty() {
                return normalized.clone();
            }
            return format!("gemini-{body}");
        }
    }
    format!("gemini-{rest}")
}

fn variant_selection_score(
    model: &ParsedCatalogModel,
    default_agent_model_id: Option<&str>,
) -> i32 {
    let id = model.id.trim();
    let lower = id.to_ascii_lowercase();
    let mut score = 0;
    if model.recommended {
        score += 1_000;
    }
    if default_agent_model_id.is_some_and(|default_id| default_id.eq_ignore_ascii_case(id)) {
        score += 500;
    }
    if id == "gemini-pro-agent" {
        score += 1_100;
    }
    if lower.ends_with("-high") && id != "gemini-3.1-pro-high" {
        score += 80;
    }
    if !lower.ends_with("-low") && !lower.ends_with("-agent") {
        score += 50;
    }
    if lower.ends_with("-agent") && id != "gemini-pro-agent" {
        score += 30;
    }
    if lower.ends_with("-low") {
        score += 10;
    }
    if id == "gemini-3.1-pro-high" {
        score -= 200;
    }
    score
}

fn prettify_gemini_family_display(id: &str) -> String {
    prettify_gemini_model_id(&gemini_family_key(id))
}

pub(super) fn to_chat_model_infos(groups: Vec<GroupedCatalogModel>) -> Vec<ChatModelInfo> {
    groups
        .into_iter()
        .map(|group| {
            let display = prettify_gemini_family_display(&group.family_key);
            let thinking_variants = if group.variants.len() > 1 {
                Some(group.variants)
            } else {
                None
            };
            ChatModelInfo {
                id: group.default_variant_id,
                owned_by: "Google".to_string(),
                provider: "gemini".to_string(),
                display_name: Some(display),
                thinking_variants,
                reasoning: None,
            }
        })
        .collect()
}

pub(super) fn fallback_chat_model_infos() -> Vec<ChatModelInfo> {
    let models: Vec<ParsedCatalogModel> = GEMINI_DEFAULT_MODELS
        .iter()
        .map(|id| ParsedCatalogModel {
            id: (*id).to_string(),
            recommended: false,
        })
        .collect();
    to_chat_model_infos(group_gemini_thinking_variants(models, None))
}

fn prettify_gemini_model_id(id: &str) -> String {
    let raw = id.trim();
    let rest = raw
        .strip_prefix("gemini-")
        .or_else(|| raw.strip_prefix("Gemini-"))
        .unwrap_or(raw);

    let lower = rest.to_ascii_lowercase();
    let (body, tier) = if lower.ends_with("-agent") {
        (&rest[..rest.len().saturating_sub(6)], "Agent")
    } else if lower.ends_with("-high") {
        (&rest[..rest.len().saturating_sub(5)], "High")
    } else if lower.ends_with("-low") {
        (&rest[..rest.len().saturating_sub(4)], "Low")
    } else {
        (rest, "")
    };

    let body = body.trim_end_matches('-');
    let parts: Vec<&str> = body.split('-').filter(|part| !part.is_empty()).collect();
    if parts.len() >= 2 {
        let version = parts[0];
        let family = parts[1..]
            .iter()
            .map(|part| capitalize_ascii_word(part))
            .collect::<Vec<_>>()
            .join(" ");
        let base = format!("Gemini {version} {family}");
        if tier.is_empty() {
            return base;
        }
        return format!("{base} ({tier})");
    }

    if parts.len() == 1 {
        return format!("Gemini {}", capitalize_ascii_word(parts[0]));
    }

    format!("Gemini {body}")
}

fn capitalize_ascii_word(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettify_common_gemini_ids() {
        assert_eq!(prettify_gemini_model_id("gemini-3-flash"), "Gemini 3 Flash");
        assert_eq!(
            prettify_gemini_model_id("gemini-3-flash-agent"),
            "Gemini 3 Flash (Agent)"
        );
        assert_eq!(
            prettify_gemini_model_id("gemini-3.1-pro-high"),
            "Gemini 3.1 Pro (High)"
        );
        assert_eq!(
            prettify_gemini_model_id("gemini-3.5-flash-low"),
            "Gemini 3.5 Flash (Low)"
        );
    }

    #[test]
    fn group_thinking_tiers_by_family() {
        let models = vec![
            ParsedCatalogModel {
                id: "gemini-3.1-pro-high".into(),
                recommended: true,
            },
            ParsedCatalogModel {
                id: "gemini-3.1-pro-low".into(),
                recommended: false,
            },
            ParsedCatalogModel {
                id: "gemini-pro-agent".into(),
                recommended: false,
            },
            ParsedCatalogModel {
                id: "gemini-3-flash".into(),
                recommended: true,
            },
            ParsedCatalogModel {
                id: "gemini-3-flash-agent".into(),
                recommended: false,
            },
        ];
        let grouped = group_gemini_thinking_variants(models, Some("gemini-3-flash"));
        assert_eq!(grouped.len(), 2);

        let pro = grouped
            .iter()
            .find(|group| group.family_key == "gemini-3.1-pro")
            .expect("pro family");
        assert_eq!(pro.default_variant_id, "gemini-pro-agent");
        assert_eq!(pro.variants.len(), 3);
        assert!(pro.variants.iter().any(|variant| variant.label == "Agent"));

        let flash = grouped
            .iter()
            .find(|group| group.family_key == "gemini-3-flash")
            .expect("flash family");
        assert_eq!(flash.default_variant_id, "gemini-3-flash");
        assert_eq!(flash.variants.len(), 2);
    }

    #[test]
    fn resolve_broken_high_pro_route() {
        assert_eq!(
            resolve_antigravity_model_id("gemini-3.1-pro-high"),
            "gemini-pro-agent"
        );
    }
}
