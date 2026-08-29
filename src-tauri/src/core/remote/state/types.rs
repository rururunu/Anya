//! Remote gateway types, pairing records, and in-memory runtime inner state.
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU16};
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use super::super::protocol::DEFAULT_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelPrefs {
    /// When true, pairing connection details will be derived from a public tunnel.
    #[serde(default)]
    pub cloudflared_enabled: bool,

    /// Optional tunnel token (named tunnel).
    /// If empty, we use a quick tunnel.
    #[serde(default)]
    pub cloudflared_token: Option<String>,

    /// Optional fixed public hostname override (e.g. `xxxx.trycloudflare.com`).
    /// If set, we avoid relying on parsing logs.
    #[serde(default)]
    pub cloudflared_hostname: Option<String>,

    /// cloudflared binary name or absolute path.
    #[serde(default = "default_cloudflared_binary")]
    pub cloudflared_binary: String,

    /// Force quick tunnel even if token is provided.
    #[serde(default)]
    pub use_quick_tunnel: bool,
}

fn default_cloudflared_binary() -> String {
    // On Windows the executable is typically `cloudflared.exe`.
    // Using the extension improves "program not found" behavior when PATH differs
    // between the user's shell and the Tauri process environment.
    #[cfg(target_os = "windows")]
    {
        "cloudflared.exe".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "cloudflared".to_string()
    }
}

impl Default for TunnelPrefs {
    fn default() -> Self {
        Self {
            cloudflared_enabled: false,
            cloudflared_token: None,
            cloudflared_hostname: None,
            cloudflared_binary: default_cloudflared_binary(),
            use_quick_tunnel: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TunnelPublicInfo {
    pub host: String,
    pub port: u16,
    pub scheme: String,
}

/// Loopback origin registered for `/p/{id}/` reverse-proxy.
#[derive(Debug, Clone)]
pub struct PreviewOrigin {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub origin_url: String,
    #[allow(dead_code)]
    pub session_id: String,
}

pub(super) const MAX_PREVIEWS: usize = 32;

pub(crate) struct TunnelRuntime {
    pub child: Child,
    pub public: TunnelPublicInfo,
}

/// 看护线程眼中的隧道子进程状态。
pub(crate) enum TunnelChildHealth {
    Running,
    Exited,
    /// 运行时已被移除（主动停止或已被重建）。
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub device_id: String,
    pub credential: String,
    pub device_name: Option<String>,
    pub paired_at_epoch_ms: i64,
    pub last_seen_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingSessionInfo {
    pub token: String,
    pub pairing_code: String,
    pub host: String,
    pub hosts: Vec<String>,
    pub port: u16,
    pub scheme: String,
    /// LAN addresses stay reachable even when a public tunnel replaces `host`,
    /// so the UI can offer both entry points at once.
    pub lan_hosts: Vec<String>,
    pub lan_port: u16,
    pub qr_payload: String,
    pub qr_data_url: String,
    pub expires_at_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub running: bool,
    pub port: u16,
    pub connected_clients: usize,
    pub pairing_active: bool,
    pub pairing: Option<PairingSessionInfo>,
    pub devices: Vec<PairedDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayPrefs {
    pub(crate) enabled: bool,
    #[serde(default = "default_gateway_port")]
    pub(crate) port: u16,
    #[serde(default)]
    pub(crate) tunnel: TunnelPrefs,
}

fn default_gateway_port() -> u16 {
    DEFAULT_PORT
}

#[derive(Debug, Clone)]
pub struct ActivePairing {
    pub token: String,
    pub pairing_code: String,
    pub expires_at: SystemTime,
}

pub(crate) struct RuntimeInner {
    pub(crate) stop_tx: Option<oneshot::Sender<()>>,
    pub(crate) pairing: Option<ActivePairing>,
    pub(crate) devices: HashMap<String, PairedDevice>,
    pub(crate) connected: HashMap<String, SocketAddr>,
    /// Latest connection for a device; sending on the previous sender kicks the old WS.
    pub(crate) session_kicks: HashMap<String, oneshot::Sender<()>>,
    pub(crate) tunnel: Option<TunnelRuntime>,
    pub(crate) devices_path: PathBuf,
    pub(crate) prefs_path: PathBuf,
    pub(crate) prefs: GatewayPrefs,
    pub(crate) previews: HashMap<String, PreviewOrigin>,
    pub(crate) preview_order: VecDeque<String>,
    pub(crate) last_preview_id: Option<String>,
}

pub struct RemoteGatewayState {
    pub(crate) inner: Mutex<RuntimeInner>,
    pub(crate) running: AtomicBool,
    pub(crate) port: AtomicU16,
}
