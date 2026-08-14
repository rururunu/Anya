//! Semantic embedding engine for workspace search.
//!
//! Supports two backends behind one interface:
//! - **Local** — [`fastembed`] (ONNX Runtime), model downloaded lazily on first
//!   enable and never bundled with the app.
//! - **API** — any OpenAI-compatible `/embeddings` endpoint (e.g. SiliconFlow,
//!   OpenAI, local Ollama), configured with a base URL + API key + model id.
//!
//! Inference is synchronous so it can run inside the sync tool execution path
//! (which itself runs on a blocking thread via `dispatch_async`). The API path
//! uses `reqwest::blocking` for the same reason.
//!
//! [`SemanticSearchEngine`] is a process-wide singleton; enable/disable and the
//! lifecycle are driven by the `set_semantic_search` command, while
//! `search_codebase` calls [`SemanticSearchEngine::rerank`] to re-rank keyword
//! hits by cosine similarity against the query embedding.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use serde_json::json;

use crate::models::settings::{SemanticSearchBackend, SemanticSearchModel};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SemanticSearchState {
    /// Disabled (or enabled but not yet started on this process).
    Idle,
    /// Local model is downloading / loading.
    Downloading,
    /// Backend loaded and ready for inference.
    Ready,
    /// Last load attempt failed (message).
    Error { message: String },
}

/// Full embedding configuration for one enabled backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingConfig {
    pub backend: SemanticSearchBackend,
    pub local_model: SemanticSearchModel,
    pub api_base_url: String,
    pub api_key: String,
    pub api_model: String,
}

impl SemanticSearchModel {
    pub fn to_fastembed(self) -> EmbeddingModel {
        match self {
            Self::MultilingualE5Small => EmbeddingModel::MultilingualE5Small,
            Self::BGESmallZHV15 => EmbeddingModel::BGESmallZHV15,
            Self::BGESmallENV15 => EmbeddingModel::BGESmallENV15,
            Self::JinaEmbeddingsV2BaseCode => EmbeddingModel::JinaEmbeddingsV2BaseCode,
            Self::BGEM3 => EmbeddingModel::BGEM3,
        }
    }

    /// E5-family models expect `query:` / `passage:` prefixes for best results.
    fn needs_prefix(self) -> bool {
        matches!(self, Self::MultilingualE5Small)
    }
}

enum Backend {
    Local {
        embedder: TextEmbedding,
        needs_prefix: bool,
    },
    Api(ApiEmbedder),
}

struct ApiEmbedder {
    base_url: String,
    api_key: String,
    model: String,
}

impl ApiEmbedder {
    fn embed_blocking(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .json(&json!({ "input": texts, "model": self.model }))
            .send()
            .map_err(|e| format!("embeddings request failed: {e}"))?;

        let status = response.status();
        let body = response.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            let mut end = body.len().min(400);
            while end > 0 && !body.is_char_boundary(end) {
                end -= 1;
            }
            return Err(format!(
                "embeddings API returned {status}: {}",
                &body[..end]
            ));
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("invalid embeddings response: {e}"))?;
        let mut items: Vec<(usize, Vec<f32>)> = Vec::new();
        if let Some(arr) = parsed["data"].as_array() {
            for item in arr {
                let index = item["index"].as_u64().unwrap_or(items.len() as u64) as usize;
                let embedding: Vec<f32> = item["embedding"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_f64())
                            .map(|f| f as f32)
                            .collect()
                    })
                    .unwrap_or_default();
                if !embedding.is_empty() {
                    items.push((index, embedding));
                }
            }
        }
        if items.is_empty() {
            return Err("embeddings API returned no data".to_string());
        }
        items.sort_by_key(|(index, _)| *index);
        Ok(items.into_iter().map(|(_, embedding)| embedding).collect())
    }
}

struct EngineInner {
    config: EmbeddingConfig,
    state: SemanticSearchState,
    backend: Option<Backend>,
    /// Monotonic epoch: bumped on every enable/disable so an in-flight download
    /// that is superseded (disabled or re-enabled) is ignored.
    generation: u64,
}

pub struct SemanticSearchEngine {
    inner: Mutex<EngineInner>,
}

impl SemanticSearchEngine {
    fn shared() -> &'static Self {
        static ENGINE: OnceLock<SemanticSearchEngine> = OnceLock::new();
        ENGINE.get_or_init(|| SemanticSearchEngine {
            inner: Mutex::new(EngineInner {
                config: EmbeddingConfig {
                    backend: SemanticSearchBackend::default(),
                    local_model: SemanticSearchModel::default(),
                    api_base_url: String::new(),
                    api_key: String::new(),
                    api_model: String::new(),
                },
                state: SemanticSearchState::Idle,
                backend: None,
                generation: 0,
            }),
        })
    }

    pub fn state() -> SemanticSearchState {
        let engine = Self::shared();
        let guard = engine
            .inner
            .lock()
            .expect("semantic search engine lock poisoned");
        guard.state.clone()
    }

    pub fn is_ready() -> bool {
        matches!(Self::state(), SemanticSearchState::Ready)
    }

    /// Enable the given backend configuration. Local backends download/load on a
    /// background thread (command returns with `Downloading`); API backends are
    /// validated and become `Ready` immediately.
    pub fn enable(config: EmbeddingConfig, cache_dir: PathBuf) {
        // Fast path: already loaded with the exact same config.
        {
            let engine = Self::shared();
            let mut guard = engine
                .inner
                .lock()
                .expect("semantic search engine lock poisoned");
            if guard.backend.is_some() && guard.config == config {
                guard.state = SemanticSearchState::Ready;
                return;
            }
        }

        if config.backend == SemanticSearchBackend::Api {
            let engine = Self::shared();
            let mut guard = engine
                .inner
                .lock()
                .expect("semantic search engine lock poisoned");
            guard.generation += 1;
            guard.config = config.clone();
            guard.backend = None;
            if config.api_base_url.trim().is_empty()
                || config.api_key.trim().is_empty()
                || config.api_model.trim().is_empty()
            {
                guard.state = SemanticSearchState::Error {
                    message: "请填写 API 地址、API Key 与模型".to_string(),
                };
                return;
            }
            guard.backend = Some(Backend::Api(ApiEmbedder {
                base_url: config.api_base_url.trim().to_string(),
                api_key: config.api_key.trim().to_string(),
                model: config.api_model.trim().to_string(),
            }));
            guard.state = SemanticSearchState::Ready;
            return;
        }

        // Local backend: download + load in the background.
        let model = config.local_model;
        let generation = {
            let engine = Self::shared();
            let mut guard = engine
                .inner
                .lock()
                .expect("semantic search engine lock poisoned");
            guard.config = config;
            guard.generation += 1;
            guard.state = SemanticSearchState::Downloading;
            guard.generation
        };

        std::thread::spawn(move || {
            let result = load_embedder(model, cache_dir);
            let engine = Self::shared();
            let mut guard = engine
                .inner
                .lock()
                .expect("semantic search engine lock poisoned");
            if guard.generation != generation {
                // Superseded by disable() or a newer enable() — drop the result.
                return;
            }
            match result {
                Ok(embedder) => {
                    let needs_prefix = model.needs_prefix();
                    guard.backend = Some(Backend::Local {
                        embedder,
                        needs_prefix,
                    });
                    guard.state = SemanticSearchState::Ready;
                }
                Err(message) => {
                    guard.backend = None;
                    guard.state = SemanticSearchState::Error { message };
                }
            }
        });
    }

    pub fn disable() {
        let engine = Self::shared();
        let mut guard = engine
            .inner
            .lock()
            .expect("semantic search engine lock poisoned");
        guard.generation += 1;
        guard.backend = None;
        guard.state = SemanticSearchState::Idle;
    }

    /// Re-rank candidate passages against a query by cosine similarity.
    /// Returns one score per passage (in the same order). Errors when the
    /// engine is not ready — callers should fall back to keyword-only results.
    pub fn rerank(query: &str, passages: &[String]) -> Result<Vec<f32>, String> {
        let engine = Self::shared();
        let mut guard = engine
            .inner
            .lock()
            .map_err(|_| "semantic search engine lock poisoned".to_string())?;
        let state = guard.state.clone();

        match guard.backend.as_mut() {
            Some(Backend::Local {
                embedder,
                needs_prefix,
            }) => {
                let query_text = apply_prefix(query, *needs_prefix, true);
                let passage_texts: Vec<String> = passages
                    .iter()
                    .map(|p| apply_prefix(p, *needs_prefix, false))
                    .collect();
                let query_vec = embedder
                    .embed(&[query_text.as_str()], None)
                    .map_err(|e| e.to_string())?
                    .remove(0);
                let passage_vecs = embedder
                    .embed(&passage_texts, None)
                    .map_err(|e| e.to_string())?;
                Ok(passage_vecs
                    .iter()
                    .map(|p| cosine_similarity(&query_vec, p))
                    .collect())
            }
            Some(Backend::Api(api)) => {
                let mut all = Vec::with_capacity(passages.len() + 1);
                all.push(query.to_string());
                all.extend(passages.iter().cloned());
                let vectors = api.embed_blocking(&all)?;
                let query_vec = vectors
                    .first()
                    .ok_or_else(|| "embeddings API returned no vectors".to_string())?;
                Ok(vectors[1..]
                    .iter()
                    .map(|p| cosine_similarity(query_vec, p))
                    .collect())
            }
            None => Err(not_ready_message(&state)),
        }
    }
}

fn not_ready_message(state: &SemanticSearchState) -> String {
    match state {
        SemanticSearchState::Downloading => "embedding model is still downloading".to_string(),
        SemanticSearchState::Error { message } => format!("embedding unavailable: {message}"),
        _ => "semantic search is not enabled".to_string(),
    }
}

fn apply_prefix(text: &str, needs_prefix: bool, is_query: bool) -> String {
    if !needs_prefix {
        return text.to_string();
    }
    if is_query {
        format!("query: {text}")
    } else {
        format!("passage: {text}")
    }
}

fn load_embedder(
    model: SemanticSearchModel,
    cache_dir: PathBuf,
) -> Result<TextEmbedding, String> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().min(4))
        .unwrap_or(2);
    let options = TextInitOptions::new(model.to_fastembed())
        .with_cache_dir(cache_dir)
        .with_intra_threads(threads)
        .with_show_download_progress(false);
    TextEmbedding::try_new(options).map_err(|e| e.to_string())
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}
