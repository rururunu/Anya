//! Per-session compose mirror for Remote Companion sync.
//!
//! Desktop Pinia (`sessionCompose`) is the UI source of truth; this store is a
//! Rust-side mirror so the gateway can apply overrides on `chat.send` and push
//! bidirectional updates without round-tripping through Vue for every phone RPC.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::models::settings::{ChatMode, ToolApprovalMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionCompose {
    pub chat_mode: ChatMode,
    pub tool_approval_mode: ToolApprovalMode,
    #[serde(default)]
    pub chat_model: String,
    #[serde(default)]
    pub chat_model_provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_model_label: Option<String>,
}

impl Default for SessionCompose {
    fn default() -> Self {
        Self {
            chat_mode: ChatMode::Agent,
            tool_approval_mode: ToolApprovalMode::Ask,
            chat_model: String::new(),
            chat_model_provider: String::new(),
            chat_model_label: None,
        }
    }
}

impl SessionCompose {
    pub fn merge_patch(&mut self, patch: &SessionComposePatch) {
        if let Some(mode) = patch.chat_mode {
            self.chat_mode = mode;
        }
        if let Some(mode) = patch.tool_approval_mode {
            self.tool_approval_mode = mode;
        }
        if let Some(model) = patch.chat_model.as_ref() {
            self.chat_model = model.clone();
        }
        if let Some(provider) = patch.chat_model_provider.as_ref() {
            self.chat_model_provider = provider.clone();
        }
        if let Some(label) = patch.chat_model_label.as_ref() {
            self.chat_model_label = if label.is_empty() {
                None
            } else {
                Some(label.clone())
            };
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionComposePatch {
    #[serde(default)]
    pub chat_mode: Option<ChatMode>,
    #[serde(default)]
    pub tool_approval_mode: Option<ToolApprovalMode>,
    #[serde(default)]
    pub chat_model: Option<String>,
    #[serde(default)]
    pub chat_model_provider: Option<String>,
    #[serde(default)]
    pub chat_model_label: Option<String>,
}

struct ComposeStore {
    by_session: HashMap<String, SessionCompose>,
}

fn store() -> &'static Mutex<ComposeStore> {
    static STORE: OnceLock<Mutex<ComposeStore>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(ComposeStore {
            by_session: HashMap::new(),
        })
    })
}

pub fn get(session_id: &str) -> SessionCompose {
    store()
        .lock()
        .ok()
        .and_then(|g| g.by_session.get(session_id).cloned())
        .unwrap_or_default()
}

pub fn set(session_id: &str, compose: SessionCompose) -> SessionCompose {
    if let Ok(mut guard) = store().lock() {
        guard.by_session.insert(session_id.to_string(), compose.clone());
    }
    compose
}

pub fn patch(session_id: &str, patch: &SessionComposePatch) -> SessionCompose {
    let mut current = get(session_id);
    current.merge_patch(patch);
    set(session_id, current)
}

pub fn event_payload(session_id: &str, compose: &SessionCompose) -> serde_json::Map<String, Value> {
    json!({
        "sessionId": session_id,
        "compose": compose,
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}
