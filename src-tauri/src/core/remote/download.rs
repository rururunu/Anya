//! Desktop → Companion file downloads over HTTP Range.
//!
//! The phone requests a short-lived download URL over WebSocket
//! (`file.download.begin`); desktop mints an unguessable ticket bound to a
//! resolved file, and the phone streams the file over HTTP with `Range`
//! (continuous streaming + resumable, instead of one WS slice per RPC).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;
use uuid::Uuid;

use super::pairing::local_ipv4_hosts;
use super::state::remote_state;

const TICKET_TTL_SECS: u64 = 10 * 60;

#[derive(Debug, Clone)]
pub struct DownloadTicket {
    pub file: PathBuf,
    pub name: String,
    pub mime: String,
    pub size: u64,
    expires_at: u64,
}

impl DownloadTicket {
    fn expired(&self) -> bool {
        self.expires_at <= now_secs()
    }
}

fn store() -> &'static Mutex<HashMap<String, DownloadTicket>> {
    static STORE: OnceLock<Mutex<HashMap<String, DownloadTicket>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a resolved file for HTTP download; returns the ticket id.
pub fn mint(file: PathBuf, name: String, mime: String, size: u64) -> Result<String, String> {
    let id = Uuid::new_v4().simple().to_string();
    let mut map = store().lock().map_err(|e| e.to_string())?;
    map.insert(
        id.clone(),
        DownloadTicket {
            file,
            name,
            mime,
            size,
            expires_at: now_secs() + TICKET_TTL_SECS,
        },
    );
    Ok(id)
}

pub fn lookup(id: &str) -> Option<DownloadTicket> {
    let mut map = store().lock().ok()?;
    if let Some(ticket) = map.get(id) {
        if !ticket.expired() {
            return Some(ticket.clone());
        }
        map.remove(id);
    }
    None
}

/// Public HTTP URL for a download ticket (tunnel if available, else LAN).
pub fn public_download_url(app: &AppHandle, id: &str) -> String {
    let state = remote_state(app);
    let port = if state.is_running() {
        state.port()
    } else {
        state.preferred_port()
    };
    if let Some(public) = state.tunnel_public_info() {
        let scheme = if public.scheme.is_empty() {
            "https"
        } else {
            public.scheme.as_str()
        };
        return format!("{scheme}://{}/f/{id}", public.host.trim_end_matches('/'));
    }
    let lan = local_ipv4_hosts()
        .into_iter()
        .next()
        .unwrap_or_else(|| "127.0.0.1".into());
    format!("http://{lan}:{port}/f/{id}")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
