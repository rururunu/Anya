//! Structured capability declarations layered on top of legacy string permissions.
//!
//! Legacy `permissions: string[]` stays as-is for coarse grants (storage, pty, run...).
//! `capabilities` lets a plugin declare a *scoped* instance of a capability category
//! (e.g. "net.listen" restricted to a loopback port range) without Anya adding a new
//! permission string + UI branch + approval branch for every new scenario. Adding a
//! scenario is filling in `scope`, not adding a match arm.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::tools::error::ToolError;

/// Categories Anya knows how to validate scope for. New categories still need one
/// validator, but the same category covers unlimited scoped scenarios.
pub const KNOWN_CAPABILITY_CATEGORIES: &[&str] = &["net.listen", "net.connect", "fs.scope"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDecl {
    /// Category id, e.g. "net.listen".
    pub id: String,
    /// Category-specific constraints, validated by `validate_scope`.
    #[serde(default)]
    pub scope: Value,
}

pub fn is_known_capability(id: &str) -> bool {
    KNOWN_CAPABILITY_CATEGORIES.contains(&id)
}

pub fn capability_label(id: &str) -> &'static str {
    match id {
        "net.listen" => "Open a loopback network port in the plugin's own scope",
        "net.connect" => "Connect out to a specific host/port from the plugin sidecar",
        "fs.scope" => "Read/write a specific folder outside the plugin's own directory",
        _ => "Custom capability",
    }
}

/// Validates `scope` for a known category. Unknown categories are rejected in
/// `validate_capability_decl`, so this only needs to handle known ones.
pub fn validate_scope(id: &str, scope: &Value) -> Result<(), ToolError> {
    match id {
        "net.listen" | "net.connect" => {
            let port_range = scope
                .get("portRange")
                .and_then(|v| v.as_array())
                .filter(|arr| arr.len() == 2);
            let Some(range) = port_range else {
                return Err(ToolError::new(format!(
                    "capability `{id}` scope requires portRange: [min, max]"
                )));
            };
            let lo = range[0].as_u64().unwrap_or(0);
            let hi = range[1].as_u64().unwrap_or(0);
            if lo < 1024 || hi > 65535 || lo > hi {
                return Err(ToolError::new(format!(
                    "capability `{id}` portRange must be within 1024-65535 and lo <= hi"
                )));
            }
            let host = scope.get("host").and_then(|v| v.as_str()).unwrap_or("");
            if host != "127.0.0.1" && host != "localhost" {
                return Err(ToolError::new(format!(
                    "capability `{id}` host must be 127.0.0.1 or localhost (loopback only)"
                )));
            }
            Ok(())
        }
        "fs.scope" => {
            let has_root = scope
                .get("root")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !has_root {
                return Err(ToolError::new(
                    "capability `fs.scope` scope requires a non-empty `root`",
                ));
            }
            Ok(())
        }
        _ => Err(ToolError::new(format!("unknown capability `{id}`"))),
    }
}

pub fn validate_capability_decl(decl: &CapabilityDecl) -> Result<(), ToolError> {
    if !is_known_capability(&decl.id) {
        return Err(ToolError::new(format!("unknown capability `{}`", decl.id)));
    }
    validate_scope(&decl.id, &decl.scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_loopback_listen() {
        let decl = CapabilityDecl {
            id: "net.listen".into(),
            scope: json!({ "host": "127.0.0.1", "portRange": [40000, 40100] }),
        };
        assert!(validate_capability_decl(&decl).is_ok());
    }

    #[test]
    fn rejects_non_loopback_host() {
        let decl = CapabilityDecl {
            id: "net.listen".into(),
            scope: json!({ "host": "0.0.0.0", "portRange": [40000, 40100] }),
        };
        assert!(validate_capability_decl(&decl).is_err());
    }

    #[test]
    fn rejects_unknown_category() {
        let decl = CapabilityDecl {
            id: "net.broadcast".into(),
            scope: json!({}),
        };
        assert!(validate_capability_decl(&decl).is_err());
    }
}
