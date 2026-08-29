//! Gateway start/stop, pairing info, and process-wide remote state.
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use super::types::{
    ActivePairing, GatewayStatus, PairedDevice, PairingSessionInfo, RemoteGatewayState,
};

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
    crate::core::remote::tunnel::stop_cloudflared_tunnel(app);
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
    if let Err(error) = crate::core::remote::gateway::start_gateway(app.clone(), Some(port)) {
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
    let lan_hosts: Vec<String> = crate::core::remote::pairing::local_ipv4_hosts()
        .into_iter()
        .filter(|h| !crate::core::remote::pairing::is_loopback_host(h))
        .collect();
    let mut hosts = lan_hosts.clone();
    let mut host = hosts.first().cloned().unwrap_or_default();
    let mut port = local_port;
    let mut scheme = "ws".to_string();

    // Public pairing: Cloudflare tunnel provides a stable wss:// host.
    let tunnel_enabled = state.tunnel_prefs().cloudflared_enabled;
    if tunnel_enabled {
        if let Some(public) = crate::core::remote::tunnel::resolve_tunnel_public(app, local_port, ensure_tunnel) {
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
        .find(|lan| *lan != &host && !crate::core::remote::pairing::is_loopback_host(lan))
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
    let qr_data_url = crate::core::remote::pairing::qr_data_url(&qr_payload)?;
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
