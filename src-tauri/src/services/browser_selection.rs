//! Optional, authenticated loopback bridge for the Chrome/Edge selection extension.
use crate::core::context::{models::WindowInfo, platform::WindowDetector};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct BrowserSelection {
    pub text: String,
    pub url: String,
    window: WindowInfo,
    received: Instant,
}
static SELECTION: OnceLock<Mutex<Option<BrowserSelection>>> = OnceLock::new();
fn cache() -> &'static Mutex<Option<BrowserSelection>> {
    SELECTION.get_or_init(|| Mutex::new(None))
}
#[derive(Serialize, Deserialize)]
struct BridgeConfig {
    port: u16,
    token: String,
}
#[derive(Deserialize)]
struct Packet {
    token: String,
    text: String,
    url: String,
    title: String,
}

fn matches_source(selection: &BrowserSelection, window: &WindowInfo) -> bool {
    selection.window.hwnd == window.hwnd
        && selection.window.pid == window.pid
        && selection.window.title == window.title
        && selection.received.elapsed() < Duration::from_secs(60)
}
pub fn selection_for(window: &WindowInfo) -> Option<BrowserSelection> {
    cache()
        .lock()
        .ok()?
        .as_ref()
        .filter(|selection| matches_source(selection, window))
        .cloned()
}
fn browser_window(window: &WindowInfo) -> bool {
    matches!(
        window.process_name.to_ascii_lowercase().as_str(),
        "chrome.exe" | "msedge.exe" | "brave.exe" | "vivaldi.exe"
    )
}
fn accept_packet(packet: Packet, token: &str) -> bool {
    if packet.token != token
        || packet.text.len() > 262144
        || packet.title.len() > 4096
        || packet.url.len() > 8192
    {
        return false;
    }
    if packet.text.is_empty() {
        if let Ok(mut value) = cache().lock() {
            *value = None;
        }
        return true;
    }
    if !(packet.url.starts_with("https://") || packet.url.starts_with("http://")) {
        return false;
    }
    let Ok(window) = WindowDetector::detect() else {
        return false;
    };
    if !browser_window(&window) || packet.title.is_empty() || !window.title.contains(&packet.title)
    {
        return false;
    }
    if let Ok(mut value) = cache().lock() {
        *value = Some(BrowserSelection {
            text: packet.text,
            url: packet.url,
            window,
            received: Instant::now(),
        });
        return true;
    }
    false
}
fn serve(mut stream: TcpStream, token: &str) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let mut length = [0u8; 4];
    if stream.read_exact(&mut length).is_err() {
        return;
    }
    let length = u32::from_le_bytes(length) as usize;
    if length > 1048576 {
        return;
    }
    let mut bytes = vec![0; length];
    if stream.read_exact(&mut bytes).is_err() {
        return;
    }
    let ok = serde_json::from_slice::<Packet>(&bytes)
        .ok()
        .is_some_and(|packet| accept_packet(packet, token));
    let reply = if ok {
        b"{\"ok\":true}".as_slice()
    } else {
        b"{\"ok\":false}".as_slice()
    };
    let _ = stream.write_all(&(reply.len() as u32).to_le_bytes());
    let _ = stream.write_all(reply);
}
pub fn start() {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let directory = std::path::PathBuf::from(local).join("Anya");
    let Ok(listener) = TcpListener::bind(("127.0.0.1", 0)) else {
        return;
    };
    let Ok(address) = listener.local_addr() else {
        return;
    };
    let config = BridgeConfig {
        port: address.port(),
        token: uuid::Uuid::new_v4().to_string(),
    };
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(&config) else {
        return;
    };
    if std::fs::write(directory.join("browser-bridge.json"), bytes).is_err() {
        return;
    }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            serve(stream, &config.token);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_selection_must_match_exact_source_window_and_title() {
        let window = WindowInfo {
            hwnd: 12,
            pid: 33,
            title: "Page - Browser".into(),
            process_name: "chrome.exe".into(),
        };
        let selection = BrowserSelection {
            text: "selected".into(),
            url: "https://example.com".into(),
            window: window.clone(),
            received: Instant::now(),
        };
        assert!(matches_source(&selection, &window));
        assert!(!matches_source(
            &BrowserSelection {
                received: Instant::now() - Duration::from_secs(61),
                ..selection.clone()
            },
            &window
        ));
        assert!(!matches_source(
            &selection,
            &WindowInfo {
                title: "Another page".into(),
                ..window.clone()
            }
        ));
        assert!(!matches_source(
            &selection,
            &WindowInfo { hwnd: 99, ..window }
        ));
    }
    #[test]
    fn unauthenticated_packets_cannot_clear_selection() {
        assert!(!accept_packet(
            Packet {
                token: "wrong".into(),
                text: String::new(),
                title: String::new(),
                url: String::new()
            },
            "secret"
        ));
    }
}
