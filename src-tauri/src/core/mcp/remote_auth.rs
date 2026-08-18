//! mcp-remote OAuth helpers.
//!
//! Smithery (and similar) hosted MCP servers are bridged through `npx mcp-remote`.
//! That package stores OAuth tokens under `MCP_REMOTE_CONFIG_DIR/mcp-remote-{version}/`.
//! Using `@latest` (or letting the package float) creates a new version subdirectory and
//! looks like a "logout", so we:
//! 1. Pin the npm package version in install args.
//! 2. Point `MCP_REMOTE_CONFIG_DIR` at a stable app config path.
//! 3. Detect / clear saved tokens for the "re-authenticate" UX.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use md5::{Digest, Md5};
use serde::Serialize;

use crate::models::settings::McpServerConfig;

/// Keep in sync with `MCP_REMOTE_PINNED_VERSION` in `src/services/mcp/remote.ts`.
pub const MCP_REMOTE_PINNED_VERSION: &str = "0.1.38";

/// npm package arg we write into installs, e.g. `mcp-remote@0.1.38`.
pub fn mcp_remote_package_spec() -> String {
    format!("mcp-remote@{MCP_REMOTE_PINNED_VERSION}")
}

static MCP_REMOTE_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static SMITHERY_API_KEY: OnceLock<Mutex<String>> = OnceLock::new();

fn smithery_api_key_slot() -> &'static Mutex<String> {
    SMITHERY_API_KEY.get_or_init(|| Mutex::new(String::new()))
}

/// Call once from app setup with `{app_config_dir}/mcp-auth`.
pub fn init_mcp_remote_config_dir(dir: PathBuf) {
    let _ = MCP_REMOTE_CONFIG_DIR.set(dir);
}

/// Update the Smithery API key used when enriching hosted MCP URLs at spawn time.
pub fn configure_smithery_api_key(key: &str) {
    if let Ok(mut lock) = smithery_api_key_slot().lock() {
        *lock = key.trim().to_string();
    }
}

pub fn smithery_api_key() -> String {
    smithery_api_key_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

pub fn mcp_remote_config_dir() -> Option<&'static Path> {
    MCP_REMOTE_CONFIG_DIR.get().map(PathBuf::as_path)
}

/// True when this server is launched via the mcp-remote stdio bridge.
pub fn uses_mcp_remote(config: &McpServerConfig) -> bool {
    mcp_remote_package_arg_index(&config.args).is_some()
}

/// Index of the `mcp-remote` / `mcp-remote@…` package argument, if any.
fn mcp_remote_package_arg_index(args: &[String]) -> Option<usize> {
    args.iter().position(|arg| {
        let base = arg.split('@').next().unwrap_or(arg);
        base.eq_ignore_ascii_case("mcp-remote")
    })
}

/// Hosted HTTP URL passed to mcp-remote (first `http(s)://` arg after the package).
pub fn mcp_remote_server_url(config: &McpServerConfig) -> Option<String> {
    let idx = mcp_remote_package_arg_index(&config.args)?;
    config
        .args
        .iter()
        .skip(idx + 1)
        .find(|arg| {
            let lower = arg.to_ascii_lowercase();
            lower.starts_with("http://") || lower.starts_with("https://")
        })
        .cloned()
}

fn is_smithery_hosted_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("smithery.ai") || lower.contains("run.tools")
}

/// Smithery Connect proxy: `https://api.smithery.ai/connect/{ns}/{id}/mcp`
pub fn is_smithery_connect_proxy_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://api.smithery.ai/connect/")
        .or_else(|| lower.strip_prefix("http://api.smithery.ai/connect/"))
    else {
        return false;
    };
    let path = rest.split('?').next().unwrap_or(rest).trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() && parts[2] == "mcp"
}

/// Append `api_key` for Smithery hosted URLs when a key is configured and missing.
pub fn with_smithery_api_key(url: &str, api_key: &str) -> String {
    let key = api_key.trim();
    if key.is_empty() || !is_smithery_hosted_url(url) || is_smithery_connect_proxy_url(url) {
        return url.to_string();
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("api_key=") {
        return url.to_string();
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}api_key={}", urlencoding::encode(key))
}

/// Inject Smithery credentials into mcp-remote args at spawn time (never persist secrets).
/// - Connect proxy → `--header Authorization: Bearer <key>`
/// - Other hosted URLs → `?api_key=`
pub fn inject_smithery_api_key_args(args: &mut Vec<String>) -> bool {
    let key = smithery_api_key();
    if key.is_empty() {
        return false;
    }
    let Some(idx) = mcp_remote_package_arg_index(args) else {
        return false;
    };
    let mut changed = false;
    let mut url_idx: Option<usize> = None;
    for (i, arg) in args.iter().enumerate().skip(idx + 1) {
        let lower = arg.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            url_idx = Some(i);
            break;
        }
    }
    let Some(url_idx) = url_idx else {
        return false;
    };
    let url = args[url_idx].clone();
    if is_smithery_connect_proxy_url(&url) {
        // Strip any stale Authorization headers first.
        let mut i = 0;
        while i + 1 < args.len() {
            if args[i] == "--header"
                && args[i + 1]
                    .to_ascii_lowercase()
                    .starts_with("authorization:")
            {
                args.remove(i);
                args.remove(i);
                continue;
            }
            i += 1;
        }
        args.push("--header".into());
        args.push(format!("Authorization: Bearer {key}"));
        return true;
    }
    let next = with_smithery_api_key(&url, &key);
    if next != url {
        args[url_idx] = next;
        changed = true;
    }
    changed
}

/// Rewrite floating `mcp-remote` / `@latest` args to the pinned package version.
/// Returns true when args were changed.
pub fn pin_mcp_remote_args(args: &mut Vec<String>) -> bool {
    let Some(idx) = mcp_remote_package_arg_index(args) else {
        return false;
    };
    let pinned = mcp_remote_package_spec();
    if args[idx] == pinned {
        return false;
    }
    args[idx] = pinned;
    true
}

fn sanitize_mcp_install_id(raw: &str) -> String {
    let cleaned = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>()
        .to_ascii_lowercase();
    if cleaned.is_empty() {
        "mcp".into()
    } else {
        cleaned
    }
}

/// Drop legacy `sm-${qualifiedName}` install ids in favor of the bare qualifiedName.
fn migrate_legacy_smithery_install_id(
    server: &mut McpServerConfig,
    occupied: &std::collections::HashSet<String>,
) -> bool {
    let Some(qn) = server
        .qualified_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let preferred = sanitize_mcp_install_id(qn);
    if preferred.is_empty() || server.id == preferred {
        return false;
    }
    let legacy = sanitize_mcp_install_id(&format!("sm-{qn}"));
    if server.id != legacy || occupied.contains(&preferred) {
        return false;
    }
    server.id = preferred;
    true
}

/// Normalize every server's mcp-remote package pin (used on settings load/save).
pub fn normalize_mcp_servers(servers: &mut [McpServerConfig]) -> bool {
    let mut changed = false;
    let occupied = servers
        .iter()
        .map(|server| server.id.clone())
        .collect::<std::collections::HashSet<_>>();
    // Migrate ids first so later passes see the canonical install identity.
    let mut next_occupied = occupied;
    for server in servers.iter_mut() {
        let previous = server.id.clone();
        if migrate_legacy_smithery_install_id(server, &next_occupied) {
            next_occupied.remove(&previous);
            next_occupied.insert(server.id.clone());
            changed = true;
        }
        if pin_mcp_remote_args(&mut server.args) {
            changed = true;
        }
    }
    changed
}

/// Match mcp-remote's `getServerUrlHash(serverUrl)` for installs without custom headers.
pub fn server_url_hash(server_url: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(server_url.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn token_file_name(hash: &str) -> String {
    format!("{hash}_tokens.json")
}

/// Whether mcp-remote has a persisted OAuth token file for this server URL.
pub fn has_saved_credentials(config: &McpServerConfig) -> bool {
    let Some(url) = mcp_remote_server_url(config) else {
        return false;
    };
    let hash = server_url_hash(&url);
    let file = token_file_name(&hash);

    // Prefer our stable config root (includes versioned subdirs mcp-remote creates).
    if let Some(root) = mcp_remote_config_dir() {
        if find_named_file(root, &file) {
            return true;
        }
    }

    // Fall back to the default ~/.mcp-auth tree (older installs / pre-migration).
    if let Some(home) = dirs_home() {
        if find_named_file(&home.join(".mcp-auth"), &file) {
            return true;
        }
    }
    false
}

fn dirs_home() -> Option<PathBuf> {
    env_home().map(PathBuf::from)
}

fn env_home() -> Option<std::ffi::OsString> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
}

fn find_named_file(root: &Path, file_name: &str) -> bool {
    if !root.exists() {
        return false;
    }
    // Tokens live in `root/mcp-remote-{ver}/{hash}_tokens.json` (and occasionally flat).
    let direct = root.join(file_name);
    if direct.is_file() {
        return true;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(file_name).is_file() {
            return true;
        }
    }
    false
}

/// Delete OAuth client/token files for this server so the next connect re-runs the browser flow.
pub fn clear_saved_credentials(config: &McpServerConfig) -> Result<usize, String> {
    let url = mcp_remote_server_url(config)
        .ok_or_else(|| "server is not an mcp-remote bridge".to_string())?;
    let hash = server_url_hash(&url);
    let suffixes = [
        "tokens.json",
        "client_info.json",
        "code_verifier.txt",
        "lock.json",
    ];
    let mut removed = 0usize;

    let mut roots = Vec::new();
    if let Some(root) = mcp_remote_config_dir() {
        roots.push(root.to_path_buf());
    }
    if let Some(home) = dirs_home() {
        roots.push(home.join(".mcp-auth"));
    }

    for root in roots {
        removed += clear_hash_files_under(&root, &hash, &suffixes);
    }
    Ok(removed)
}

fn clear_hash_files_under(root: &Path, hash: &str, suffixes: &[&str]) -> usize {
    if !root.exists() {
        return 0;
    }
    let mut removed = 0usize;
    let mut dirs = vec![root.to_path_buf()];
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    for dir in dirs {
        for suffix in suffixes {
            let path = dir.join(format!("{hash}_{suffix}"));
            if path.is_file() && fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRuntimeStatus {
    pub id: String,
    pub enabled: bool,
    /// Server is bridged through mcp-remote (OAuth may be required).
    pub uses_remote_auth: bool,
    pub connected: bool,
    pub has_saved_credentials: bool,
    /// `disabled` | `connected` | `authenticated` | `needs_auth` | `local`
    pub state: String,
}

pub fn runtime_status_for(config: &McpServerConfig, connected: bool) -> McpServerRuntimeStatus {
    let uses_remote_auth = uses_mcp_remote(config);
    let credentials_saved = uses_remote_auth && has_saved_credentials(config);
    let state = if !config.enabled {
        "disabled".to_string()
    } else if connected {
        "connected".to_string()
    } else if !uses_remote_auth {
        "local".to_string()
    } else if credentials_saved {
        "authenticated".to_string()
    } else {
        "needs_auth".to_string()
    };

    McpServerRuntimeStatus {
        id: config.id.clone(),
        enabled: config.enabled,
        uses_remote_auth,
        connected,
        has_saved_credentials: credentials_saved,
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_floating_and_latest_package_args() {
        let mut args = vec![
            "-y".into(),
            "mcp-remote".into(),
            "https://example.com/mcp".into(),
        ];
        assert!(pin_mcp_remote_args(&mut args));
        assert_eq!(args[1], mcp_remote_package_spec());

        let mut latest = vec!["-y".into(), "mcp-remote@latest".into(), "https://x".into()];
        assert!(pin_mcp_remote_args(&mut latest));
        assert_eq!(latest[1], mcp_remote_package_spec());

        assert!(!pin_mcp_remote_args(&mut latest));
    }

    #[test]
    fn extracts_deployment_url() {
        let config = McpServerConfig {
            id: "gmail".into(),
            command: "npx".into(),
            args: vec![
                "-y".into(),
                mcp_remote_package_spec(),
                "https://server.smithery.ai/gmail".into(),
            ],
            enabled: true,
            ..Default::default()
        };
        assert_eq!(
            mcp_remote_server_url(&config).as_deref(),
            Some("https://server.smithery.ai/gmail")
        );
        assert_eq!(
            server_url_hash("https://server.smithery.ai/gmail"),
            format!("{:x}", Md5::digest(b"https://server.smithery.ai/gmail"))
        );
    }

    #[test]
    fn migrates_legacy_sm_prefixed_install_ids() {
        let mut servers = vec![McpServerConfig {
            id: "sm-gmail".into(),
            command: "npx".into(),
            args: vec![
                "-y".into(),
                mcp_remote_package_spec(),
                "https://server.smithery.ai/gmail".into(),
            ],
            enabled: true,
            qualified_name: Some("gmail".into()),
            source: Some("smithery".into()),
            ..Default::default()
        }];
        assert!(normalize_mcp_servers(&mut servers));
        assert_eq!(servers[0].id, "gmail");
        // Idempotent.
        assert!(!normalize_mcp_servers(&mut servers));
    }
}
