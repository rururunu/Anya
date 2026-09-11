//! Persistent grants and short-lived sidecar tokens.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::permissions::is_known_permission;
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;

const TOKEN_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginGrant {
    pub enabled: bool,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub granted_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GrantFile {
    #[serde(default)]
    plugins: HashMap<String, PluginGrant>,
}

#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub plugin_id: String,
    pub host_pid: u32,
    pub expires_at: u64,
}

struct TokenState {
    secret: String,
    issued: HashMap<String, IssuedToken>,
}

fn token_state() -> &'static Mutex<TokenState> {
    static STATE: OnceLock<Mutex<TokenState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(TokenState {
            secret: Uuid::new_v4().to_string(),
            issued: HashMap::new(),
        })
    })
}

fn grants_path() -> PathBuf {
    plugins_dir().join("grants.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_file() -> GrantFile {
    let path = grants_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return GrantFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_file(file: &GrantFile) -> Result<(), ToolError> {
    let path = grants_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(file).map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(path, raw).map_err(|e| ToolError::new(e.to_string()))
}

pub fn get_grant(plugin_id: &str) -> PluginGrant {
    load_file()
        .plugins
        .get(plugin_id)
        .cloned()
        .unwrap_or_default()
}

pub fn has_grant(plugin_id: &str) -> bool {
    load_file().plugins.contains_key(plugin_id)
}

#[allow(dead_code)]
pub fn list_grants() -> HashMap<String, PluginGrant> {
    load_file().plugins
}

pub fn save_grant(plugin_id: &str, grant: PluginGrant) -> Result<(), ToolError> {
    for perm in &grant.permissions {
        if !is_known_permission(perm) {
            return Err(ToolError::new(format!("unknown permission `{perm}`")));
        }
    }
    let mut file = load_file();
    file.plugins.insert(plugin_id.to_string(), grant);
    save_file(&file)
}

pub fn clear_grant(plugin_id: &str) -> Result<(), ToolError> {
    let mut file = load_file();
    file.plugins.remove(plugin_id);
    save_file(&file)?;
    revoke_token(plugin_id);
    Ok(())
}

pub fn grant_has(plugin_id: &str, permission: &str) -> bool {
    let grant = get_grant(plugin_id);
    grant.enabled && grant.permissions.iter().any(|p| p == permission)
}

pub fn issue_token(plugin_id: &str) -> Result<IssuedToken, ToolError> {
    if !get_grant(plugin_id).enabled {
        return Err(ToolError::new(format!(
            "plugin `{plugin_id}` is not enabled"
        )));
    }
    let host_pid = std::process::id();
    let expires_at = now_secs().saturating_add(TOKEN_TTL_SECS);
    let nonce = Uuid::new_v4().to_string();
    let mut state = token_state()
        .lock()
        .map_err(|_| ToolError::new("token lock"))?;
    let material = format!(
        "{}:{}:{}:{}:{}",
        state.secret, plugin_id, host_pid, expires_at, nonce
    );
    let digest = Sha256::digest(material.as_bytes());
    let token = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let issued = IssuedToken {
        token: token.clone(),
        plugin_id: plugin_id.to_string(),
        host_pid,
        expires_at,
    };
    state.issued.insert(plugin_id.to_string(), issued.clone());
    Ok(issued)
}

pub fn validate_token(plugin_id: &str, token: &str, host_pid: u32) -> Result<(), ToolError> {
    let state = token_state()
        .lock()
        .map_err(|_| ToolError::new("token lock"))?;
    let Some(issued) = state.issued.get(plugin_id) else {
        return Err(ToolError::new("plugin token not issued"));
    };
    if issued.plugin_id != plugin_id {
        return Err(ToolError::new("plugin token id mismatch"));
    }
    if issued.token != token {
        return Err(ToolError::new("plugin token mismatch"));
    }
    if issued.host_pid != host_pid {
        return Err(ToolError::new("plugin token host pid mismatch"));
    }
    if issued.expires_at < now_secs() {
        return Err(ToolError::new("plugin token expired"));
    }
    Ok(())
}

pub fn revoke_token(plugin_id: &str) {
    if let Ok(mut state) = token_state().lock() {
        state.issued.remove(plugin_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_token_roundtrip() {
        let id = format!("tok-{}", Uuid::new_v4().simple());
        save_grant(
            &id,
            PluginGrant {
                enabled: true,
                permissions: vec!["storage".into()],
                granted_at: 1,
            },
        )
        .unwrap();
        let issued = issue_token(&id).unwrap();
        validate_token(&id, &issued.token, issued.host_pid).unwrap();
        assert!(validate_token(&id, "deadbeef", issued.host_pid).is_err());
        revoke_token(&id);
        let _ = clear_grant(&id);
    }

    #[test]
    fn disabled_plugin_cannot_issue_token() {
        let id = format!("tok-{}", Uuid::new_v4().simple());
        let _ = clear_grant(&id);
        assert!(issue_token(&id).is_err());
    }

    #[test]
    fn has_grant_tracks_presence() {
        let id = format!("has-{}", Uuid::new_v4().simple());
        let _ = clear_grant(&id);
        assert!(!has_grant(&id));
        save_grant(
            &id,
            PluginGrant {
                enabled: false,
                permissions: vec![],
                granted_at: 1,
            },
        )
        .unwrap();
        assert!(has_grant(&id));
        let _ = clear_grant(&id);
    }
}
