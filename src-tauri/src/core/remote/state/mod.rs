//! Remote gateway process state, pairing, and device registry.

mod lifecycle;
mod persist;
mod types;

pub use lifecycle::{
    build_pairing_info, gateway_status, list_paired_devices, new_pairing_token, now_ms, pairing_ttl,
    remote_state, restore_gateway_if_enabled, revoke_device, stop_gateway,
};
pub use types::{
    ActivePairing, GatewayStatus, PairedDevice, PairingSessionInfo, PreviewOrigin, RemoteGatewayState,
    TunnelPrefs, TunnelPublicInfo,
};
pub(crate) use types::{TunnelChildHealth, TunnelRuntime};

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use tokio::sync::oneshot;

use super::protocol::DEFAULT_PORT;
use persist::{load_devices, load_prefs, save_devices, save_prefs};
use types::{GatewayPrefs, MAX_PREVIEWS, RuntimeInner};

impl RemoteGatewayState {
    pub(super) fn new(devices_path: PathBuf) -> Self {
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
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.last_preview_id.clone())
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
        self.inner.lock().map(|g| g.prefs.enabled).unwrap_or(false)
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
        self.inner.lock().ok()?.connected.get(device_id).copied()
    }

    pub fn connected_count(&self) -> usize {
        self.inner.lock().map(|g| g.connected.len()).unwrap_or(0)
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
