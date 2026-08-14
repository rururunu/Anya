use tauri::{AppHandle, Manager};

use crate::core::ai::embed::{EmbeddingConfig, SemanticSearchEngine, SemanticSearchState};
use crate::models::settings::{SemanticSearchBackend, SemanticSearchModel};
use crate::services::settings_store::{get_settings, set_settings};

/// Full RAG embedding configuration (persisted to settings + applied to engine).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchConfig {
    pub enabled: bool,
    pub backend: SemanticSearchBackend,
    pub model: SemanticSearchModel,
    pub api_base_url: String,
    pub api_key: String,
    pub api_model: String,
}

pub fn model_cache_dir(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("models")
}

#[tauri::command]
pub fn get_semantic_search_status() -> Result<SemanticSearchState, String> {
    Ok(SemanticSearchEngine::state())
}

#[tauri::command]
pub fn set_semantic_search(
    app: AppHandle,
    config: SemanticSearchConfig,
) -> Result<SemanticSearchState, String> {
    let mut settings = get_settings(&app)?;
    settings.semantic_search_enabled = config.enabled;
    settings.semantic_search_backend = config.backend;
    settings.semantic_search_model = config.model;
    settings.semantic_search_api_base_url = config.api_base_url;
    settings.semantic_search_api_key = config.api_key;
    settings.semantic_search_api_model = config.api_model;
    set_settings(&app, settings.clone())?;

    if config.enabled {
        SemanticSearchEngine::enable(
            EmbeddingConfig {
                backend: settings.semantic_search_backend,
                local_model: settings.semantic_search_model,
                api_base_url: settings.semantic_search_api_base_url,
                api_key: settings.semantic_search_api_key,
                api_model: settings.semantic_search_api_model,
            },
            model_cache_dir(&app),
        );
    } else {
        SemanticSearchEngine::disable();
    }

    Ok(SemanticSearchEngine::state())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTestResult {
    pub ok: bool,
    pub dim: usize,
}

fn truncate_body(body: &str) -> &str {
    let mut end = body.len().min(400);
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    &body[..end]
}

async fn embeddings_test(base_url: &str, api_key: &str, model: &str) -> Result<usize, String> {
    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&serde_json::json!({ "input": ["ping"], "model": model }))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}：{}", truncate_body(&body)));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("响应解析失败：{e}"))?;
    let dim = parsed["data"]
        .get(0)
        .and_then(|item| item["embedding"].as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if dim == 0 {
        return Err("响应里没有 embedding 数据".to_string());
    }
    Ok(dim)
}

async fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}：{}", truncate_body(&body)));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("响应解析失败：{e}"))?;
    let mut ids: Vec<String> = parsed["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err("没有获取到任何模型".to_string());
    }
    Ok(ids)
}

/// Probe the configured API with a single tiny embedding request.
#[tauri::command]
pub async fn test_semantic_search_api(
    base_url: String,
    api_key: String,
    model: String,
) -> Result<ApiTestResult, String> {
    let dim = embeddings_test(&base_url, &api_key, &model).await?;
    Ok(ApiTestResult { ok: true, dim })
}

/// List model ids from the OpenAI-compatible `/models` endpoint.
#[tauri::command]
pub async fn fetch_semantic_search_models(
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    fetch_models(&base_url, &api_key).await
}
