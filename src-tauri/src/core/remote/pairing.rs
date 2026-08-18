use std::net::UdpSocket;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use image::Luma;
use qrcode::QrCode;
use tauri::{AppHandle, Emitter};

use super::state::{
    build_pairing_info, new_pairing_token, pairing_ttl, remote_state, ActivePairing,
    PairingSessionInfo,
};

/// Best-effort LAN IPv4 addresses (excludes loopback / link-local).
/// Rejects addresses that peers on the LAN cannot actually reach:
/// loopback, link-local, and 198.18.0.0/15 (benchmark range that proxy
/// TUN adapters like Clash/mihomo fake-ip claim — routing 8.8.8.8 through
/// them makes the probe below report e.g. 198.18.0.1 as "our" address).
fn is_advertisable(v4: &std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    !v4.is_loopback()
        && !v4.is_unspecified()
        && !v4.is_link_local()
        && !(o[0] == 198 && (o[1] & 0xfe) == 18)
}

pub(crate) fn is_loopback_host(host: &str) -> bool {
    let h = host
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase();
    h == "localhost" || h == "::1" || h == "0.0.0.0" || h.starts_with("127.")
}

pub fn local_ipv4_hosts() -> Vec<String> {
    let mut hosts = Vec::new();
    for probe in ["8.8.8.8:80", "1.1.1.1:80", "192.168.0.1:80"] {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect(probe).is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    if let std::net::IpAddr::V4(v4) = addr.ip() {
                        if is_advertisable(&v4) {
                            let s = v4.to_string();
                            if !hosts.contains(&s) {
                                hosts.push(s);
                            }
                        }
                    }
                }
            }
        }
    }
    hosts
}

pub fn qr_data_url(payload: &str) -> Result<String, String> {
    let code = QrCode::new(payload.as_bytes()).map_err(|e| e.to_string())?;
    let image = code
        .render::<Luma<u8>>()
        .quiet_zone(true)
        .min_dimensions(320, 320)
        .build();
    let mut png = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png);
        image::DynamicImage::ImageLuma8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
    }
    Ok(format!("data:image/png;base64,{}", B64.encode(png)))
}

pub fn create_pairing_session(app: &AppHandle) -> Result<PairingSessionInfo, String> {
    let state = remote_state(app);
    if !state.is_running() {
        super::gateway::start_gateway(app.clone(), None)?;
        // Listener bind happens in an async task; wait a moment so the tunnel
        // can reach the local origin reliably.
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let (token, pairing_code) = new_pairing_token();
    let pairing = ActivePairing {
        token,
        pairing_code,
        expires_at: std::time::SystemTime::now() + pairing_ttl(),
    };
    state.set_pairing(Some(pairing.clone()));
    let info = build_pairing_info(app, &pairing, true)?;
    let _ = app.emit("remote-gateway-status", super::state::gateway_status(app));
    Ok(info)
}
