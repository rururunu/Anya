use std::collections::{HashMap, VecDeque};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use uuid::Uuid;

use super::protocol::DEFAULT_PORT;

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

const MAX_PREVIEWS: usize = 32;

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
struct GatewayPrefs {
    enabled: bool,
    #[serde(default = "default_gateway_port")]
    port: u16,
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

struct RuntimeInner {
    stop_tx: Option<oneshot::Sender<()>>,
    pairing: Option<ActivePairing>,
    devices: HashMap<String, PairedDevice>,
    connected: HashMap<String, SocketAddr>,
    /// Latest connection for a device; sending on the previous sender kicks the old WS.
    session_kicks: HashMap<String, oneshot::Sender<()>>,
    pub(crate) tunnel: Option<TunnelRuntime>,
    devices_path: PathBuf,
    prefs_path: PathBuf,
    pub(crate) prefs: GatewayPrefs,
    previews: HashMap<String, PreviewOrigin>,
    preview_order: VecDeque<String>,
    last_preview_id: Option<String>,
}

pub struct RemoteGatewayState {
    inner: Mutex<RuntimeInner>,
    running: AtomicBool,
    port: AtomicU16,
}

impl RemoteGatewayState {
    fn new(devices_path: PathBuf) -> Self {
        let prefs_path = devices_path.with_file_name("remote_gateway.json");
        let devices = load_devices(&devices_path);
        let prefs = load_prefs(&prefs_path).unwrap_or_else(|| {
            // Migrate: if phones were already paired before prefs existed, keep
            // the gateway available across restarts.
            GatewayPrefs {
                enabled: !devices.is_empty(),
                port: DEFAULT_PORT,
                tunnel: TunnelPrefs::default(),
            }
        });
        Self {
            inner: Mutex::new(RuntimeInner {
                stop_tx: None,
                pairing: None,
                devices,
                connected: HashMap::new(),
                session_kicks: HashMap::new(),
                tunnel: None,
                devices_path,
                prefs_path,
                prefs,
                previews: HashMap::new(),
                preview_order: VecDeque::new(),
                last_preview_id: None,
            }),
            running: AtomicBool::new(false),
            port: AtomicU16::new(DEFAULT_PORT),
        }
    }

    pub fn register_preview(&self, origin: PreviewOrigin) {
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        let id = origin.id.clone();
        if guard.previews.insert(id.clone(), origin).is_none() {
            guard.preview_order.push_back(id.clone());
            while guard.preview_order.len() > MAX_PREVIEWS {
                if let Some(old) = guard.preview_order.pop_front() {
                    guard.previews.remove(&old);
                }
            }
        }
        guard.last_preview_id = Some(id);
    }

    pub fn lookup_preview(&self, id: &str) -> Option<PreviewOrigin> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.previews.get(id).cloned())
    }

    pub fn last_preview_id(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|g| g.last_preview_id.clone())
    }

    pub fn clear_previews(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.previews.clear();
            guard.preview_order.clear();
            guard.last_preview_id = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn port(&self) -> u16 {
        self.port.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, running: bool, port: u16) {
        self.running.store(running, Ordering::SeqCst);
        self.port.store(port, Ordering::SeqCst);
    }

    pub fn preferred_port(&self) -> u16 {
        self.inner
            .lock()
            .map(|g| g.prefs.port)
            .unwrap_or(DEFAULT_PORT)
    }

    pub fn should_auto_start(&self) -> bool {
        self.inner
            .lock()
            .map(|g| g.prefs.enabled)
            .unwrap_or(false)
    }

    pub fn set_enabled_preference(&self, enabled: bool, port: Option<u16>) {
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        guard.prefs.enabled = enabled;
        if let Some(port) = port {
            guard.prefs.port = port;
        }
        let path = guard.prefs_path.clone();
        let prefs = guard.prefs.clone();
        drop(guard);
        let _ = save_prefs(&path, &prefs);
    }

    pub fn tunnel_prefs(&self) -> TunnelPrefs {
        self.inner
            .lock()
            .map(|g| g.prefs.tunnel.clone())
            .unwrap_or(TunnelPrefs {
                cloudflared_enabled: false,
                cloudflared_token: None,
                cloudflared_hostname: None,
                cloudflared_binary: "cloudflared".to_string(),
                use_quick_tunnel: true,
            })
    }

    pub fn set_tunnel_prefs(&self, next: TunnelPrefs) -> Result<(), String> {
        // Drop any running tunnel so the next pairing refresh can start fresh
        // (especially important for Quick Tunnel hostname changes).
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(mut runtime) = guard.tunnel.take() {
                let _ = runtime.child.kill();
            }
            guard.prefs.tunnel = next;
            let path = guard.prefs_path.clone();
            let prefs = guard.prefs.clone();
            drop(guard);
            return save_prefs(&path, &prefs);
        }
        Err("remote gateway lock poisoned".into())
    }

    pub(crate) fn tunnel_public_info(&self) -> Option<TunnelPublicInfo> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.tunnel.as_ref().map(|rt| rt.public.clone()))
    }

    pub(crate) fn take_tunnel_runtime(&self) -> Option<TunnelRuntime> {
        self.inner.lock().ok().and_then(|mut g| g.tunnel.take())
    }

    /// 隧道子进程健康探测（供看护线程轮询）。
    pub(crate) fn tunnel_child_health(&self) -> TunnelChildHealth {
        let Ok(mut guard) = self.inner.lock() else {
            return TunnelChildHealth::Missing;
        };
        match guard.tunnel.as_mut() {
            None => TunnelChildHealth::Missing,
            Some(runtime) => match runtime.child.try_wait() {
                Ok(None) => TunnelChildHealth::Running,
                // 已退出或状态不可查都按退出处理，交给看护重建。
                Ok(Some(_)) | Err(_) => TunnelChildHealth::Exited,
            },
        }
    }

    pub(crate) fn set_tunnel_runtime(&self, runtime: TunnelRuntime) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.tunnel = Some(runtime);
        }
    }

    pub fn take_stop_sender(&self) -> Option<oneshot::Sender<()>> {
        self.inner.lock().ok()?.stop_tx.take()
    }

    pub fn set_stop_sender(&self, tx: oneshot::Sender<()>) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.stop_tx = Some(tx);
        }
    }

    pub fn set_pairing(&self, pairing: Option<ActivePairing>) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.pairing = pairing;
        }
    }

    pub fn pairing_snapshot(&self) -> Option<ActivePairing> {
        self.inner.lock().ok()?.pairing.clone()
    }

    pub fn authorize(&self, device_id: &str, credential: &str) -> Result<PairedDevice, String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "remote gateway lock poisoned".to_string())?;
        let now = now_ms();

        if let Some(device) = guard.devices.get_mut(device_id) {
            if device.credential == credential {
                device.last_seen_epoch_ms = now;
                let cloned = device.clone();
                let path = guard.devices_path.clone();
                let devices = guard.devices.clone();
                drop(guard);
                let _ = save_devices(&path, &devices);
                return Ok(cloned);
            }
            return Err("invalid device credential".into());
        }

        let pairing = guard.pairing.clone();
        let Some(pairing) = pairing else {
            return Err("unknown device; open Connect Phone and generate a new code".into());
        };
        if pairing.expires_at < SystemTime::now() {
            guard.pairing = None;
            return Err("pairing code expired".into());
        }
        if credential != pairing.token && credential != pairing.pairing_code {
            return Err("invalid pairing token".into());
        }

        let device = PairedDevice {
            device_id: device_id.to_string(),
            // Persist whatever the phone presented so reconnect keeps working
            // whether it used the QR token or the short pairing code.
            credential: credential.to_string(),
            device_name: None,
            paired_at_epoch_ms: now,
            last_seen_epoch_ms: now,
        };
        guard.devices.insert(device_id.to_string(), device.clone());
        // One-shot pairing session — force refresh for the next phone.
        guard.pairing = None;
        let path = guard.devices_path.clone();
        let devices = guard.devices.clone();
        drop(guard);
        let _ = save_devices(&path, &devices);
        Ok(device)
    }

    pub fn mark_connected(&self, device_id: String, addr: SocketAddr) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.connected.insert(device_id, addr);
        }
    }

    /// Close any existing Companion session for this device. The new connection
    /// holds `rx` and should exit when it is signalled (a newer socket claimed the slot).
    pub fn claim_session(&self, device_id: &str) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(previous) = guard.session_kicks.insert(device_id.to_string(), tx) {
                let _ = previous.send(());
            }
        }
        rx
    }

    /// Drop the device from the connected set only if `addr` is still the recorded peer
    /// (a newer connection from the same device must not be unmarked).
    pub fn mark_disconnected(&self, device_id: &str, addr: SocketAddr) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.connected.get(device_id) == Some(&addr) {
                guard.connected.remove(device_id);
            }
        }
    }

    pub fn connected_peer(&self, device_id: &str) -> Option<SocketAddr> {
        self.inner
            .lock()
            .ok()?
            .connected
            .get(device_id)
            .copied()
    }

    pub fn connected_count(&self) -> usize {
        self.inner
            .lock()
            .map(|g| g.connected.len())
            .unwrap_or(0)
    }

    pub fn list_devices(&self) -> Vec<PairedDevice> {
        self.inner
            .lock()
            .map(|g| {
                let mut devices: Vec<_> = g.devices.values().cloned().collect();
                devices.sort_by_key(|d| std::cmp::Reverse(d.last_seen_epoch_ms));
                devices
            })
            .unwrap_or_default()
    }

    pub fn revoke(&self, device_id: &str) -> bool {
        let Ok(mut guard) = self.inner.lock() else {
            return false;
        };
        let removed = guard.devices.remove(device_id).is_some();
        guard.connected.remove(device_id);
        if removed {
            let path = guard.devices_path.clone();
            let devices = guard.devices.clone();
            drop(guard);
            let _ = save_devices(&path, &devices);
        }
        removed
    }
}

static REMOTE: OnceLock<Arc<RemoteGatewayState>> = OnceLock::new();

pub fn remote_state(app: &AppHandle) -> Arc<RemoteGatewayState> {
    REMOTE
        .get_or_init(|| {
            let path = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("remote_devices.json");
            Arc::new(RemoteGatewayState::new(path))
        })
        .clone()
}

pub fn gateway_status(app: &AppHandle) -> GatewayStatus {
    let state = remote_state(app);
    let pairing = state.pairing_snapshot().and_then(|p| {
        if p.expires_at < SystemTime::now() {
            state.set_pairing(None);
            return None;
        }
        build_pairing_info(app, &p, false).ok()
    });
    GatewayStatus {
        running: state.is_running(),
        port: state.port(),
        connected_clients: state.connected_count(),
        pairing_active: pairing.is_some(),
        pairing,
        devices: state.list_devices(),
    }
}

pub fn stop_gateway(app: &AppHandle) {
    // Stop public tunnel (if any) before shutting down local gateway.
    super::tunnel::stop_cloudflared_tunnel(app);
    let state = remote_state(app);
    if let Some(tx) = state.take_stop_sender() {
        let _ = tx.send(());
    }
    state.set_pairing(None);
    state.clear_previews();
    state.set_running(false, state.port());
    state.set_enabled_preference(false, None);
    let _ = app.emit("remote-gateway-status", gateway_status(app));
}

/// Restore the gateway after app launch when the user left it enabled.
pub fn restore_gateway_if_enabled(app: &AppHandle) {
    let state = remote_state(app);
    if !state.should_auto_start() {
        return;
    }
    let port = state.preferred_port();
    if let Err(error) = super::gateway::start_gateway(app.clone(), Some(port)) {
        tracing::warn!(error = %error, "failed to restore remote gateway on launch");
    }
}

pub fn list_paired_devices(app: &AppHandle) -> Vec<PairedDevice> {
    remote_state(app).list_devices()
}

pub fn revoke_device(app: &AppHandle, device_id: &str) -> bool {
    let removed = remote_state(app).revoke(device_id);
    let _ = app.emit("remote-gateway-status", gateway_status(app));
    removed
}

pub fn build_pairing_info(
    app: &AppHandle,
    pairing: &ActivePairing,
    ensure_tunnel: bool,
) -> Result<PairingSessionInfo, String> {
    let state = remote_state(app);
    let local_port = state.port();

    // Default: LAN pairing over plain WS (never advertise loopback as the phone's host).
    let lan_hosts: Vec<String> = super::pairing::local_ipv4_hosts()
        .into_iter()
        .filter(|h| !super::pairing::is_loopback_host(h))
        .collect();
    let mut hosts = lan_hosts.clone();
    let mut host = hosts.first().cloned().unwrap_or_default();
    let mut port = local_port;
    let mut scheme = "ws".to_string();

    // Public pairing: Cloudflare tunnel provides a stable wss:// host.
    let tunnel_enabled = state
        .inner
        .lock()
        .ok()
        .map(|g| g.prefs.tunnel.cloudflared_enabled)
        .unwrap_or(false);
    if tunnel_enabled {
        if let Some(public) =
            super::tunnel::resolve_tunnel_public(app, local_port, ensure_tunnel)
        {
            host = public.host.clone();
            hosts = vec![public.host.clone()];
            port = public.port;
            scheme = public.scheme;
        } else if ensure_tunnel {
            tracing::warn!(
                "public tunnel enabled but no public hostname is available; fill 公网 Hostname or retry refresh"
            );
        }
    }

    if host.is_empty() {
        tracing::warn!(
            "no reachable LAN IP and no public tunnel host; pairing QR has no usable address"
        );
    }

    // When the public tunnel host differs from LAN, advertise the LAN endpoint so
    // the phone can prefer a same-Wi-Fi path (more stable than Cloudflare in CN).
    let lan_qs = lan_hosts
        .iter()
        .find(|lan| *lan != &host && !super::pairing::is_loopback_host(lan))
        .map(|lan| {
            format!(
                "&lanHost={}&lanPort={}",
                urlencoding::encode(lan),
                local_port
            )
        })
        .unwrap_or_default();
    let qr_payload = format!(
        "anya://pair?v=1&host={host}&port={port}&token={token}&scheme={scheme}&code={code}{lan_qs}",
        token = urlencoding::encode(&pairing.token),
        code = urlencoding::encode(&pairing.pairing_code),
    );
    let qr_data_url = super::pairing::qr_data_url(&qr_payload)?;
    let expires_at_epoch_ms = pairing
        .expires_at
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(PairingSessionInfo {
        token: pairing.token.clone(),
        pairing_code: pairing.pairing_code.clone(),
        host,
        hosts,
        port,
        scheme,
        lan_hosts,
        lan_port: local_port,
        qr_payload,
        qr_data_url,
        expires_at_epoch_ms,
    })
}

pub fn new_pairing_token() -> (String, String) {
    let token = Uuid::new_v4().to_string().replace('-', "");
    let pairing_code = token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_uppercase();
    (token, pairing_code)
}

pub fn pairing_ttl() -> Duration {
    Duration::from_secs(10 * 60)
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn load_devices(path: &PathBuf) -> HashMap<String, PairedDevice> {
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str::<Vec<PairedDevice>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|d| (d.device_id.clone(), d))
        .collect()
}

fn save_devices(path: &PathBuf, devices: &HashMap<String, PairedDevice>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let list: Vec<_> = devices.values().cloned().collect();
    let raw = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn load_prefs(path: &PathBuf) -> Option<GatewayPrefs> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_prefs(path: &PathBuf, prefs: &GatewayPrefs) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}
