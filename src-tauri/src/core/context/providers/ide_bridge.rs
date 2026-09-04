//! Discover IDE extension bridges and pull selection on demand.
//!
//! VS Code / JetBrains plugins register under `%APPDATA%/Anya/ide-bridges/`
//! (or `~/.config/Anya/ide-bridges`) and expose `GET /context`. Anya pulls when
//! resolving overlay / chat environment context. Legacy push to
//! `POST /api/context/ide` remains supported via [`super::local_api`].

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::core::context::models::IDEContext;

use super::local_api::{clear_ide_context, parse_ide_context_payload, store_ide_context};

const CONTEXT_PATH: &str = "/context";
const PULL_TIMEOUT: Duration = Duration::from_millis(250);
const STALE_BRIDGE_MS: u128 = 5 * 60 * 1000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeRegistration {
    ide: String,
    #[allow(dead_code)]
    pid: u32,
    port: u16,
    updated_at_ms: u128,
}

#[derive(Debug, PartialEq, Eq)]
enum PullOutcome {
    Found(IDEContext),
    ContactedButEmpty,
    Unavailable,
}

/// Prefer a live bridge pull; fall back to the legacy push cache.
pub fn latest() -> Option<IDEContext> {
    match pull_from_bridges() {
        PullOutcome::Found(context) => {
            store_ide_context(context.clone());
            Some(context)
        }
        PullOutcome::ContactedButEmpty => {
            clear_ide_context();
            None
        }
        PullOutcome::Unavailable => super::local_api::latest_cached(),
    }
}

fn pull_from_bridges() -> PullOutcome {
    let mut bridges = list_bridges();
    if bridges.is_empty() {
        return PullOutcome::Unavailable;
    }
    bridges.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));

    let mut contacted = false;
    let mut best: Option<(u128, IDEContext)> = None;
    for bridge in bridges {
        match pull_bridge(&bridge) {
            Ok(Some(context)) => {
                contacted = true;
                let replace = best
                    .as_ref()
                    .map_or(true, |(updated_at, _)| bridge.updated_at_ms > *updated_at);
                if replace {
                    best = Some((bridge.updated_at_ms, context));
                }
            }
            Ok(None) => {
                contacted = true;
            }
            Err(error) => {
                tracing::debug!(
                    provider = "ide_bridge",
                    ide = %bridge.ide,
                    port = bridge.port,
                    error = %error,
                    "bridge pull failed"
                );
            }
        }
    }

    if let Some((_, context)) = best {
        tracing::debug!(
            provider = "ide_bridge",
            ide = %context.ide,
            selection_length = context.selection.as_ref().map_or(0, |text| text.chars().count()),
            "ide context pulled from bridge"
        );
        return PullOutcome::Found(context);
    }
    if contacted {
        PullOutcome::ContactedButEmpty
    } else {
        PullOutcome::Unavailable
    }
}

fn list_bridges() -> Vec<BridgeRegistration> {
    let Some(dir) = bridge_dir() else {
        return Vec::new();
    };
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let now_ms = now_millis();
    let mut bridges = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(bridge) = serde_json::from_str::<BridgeRegistration>(&raw) else {
            let _ = fs::remove_file(&path);
            continue;
        };
        if bridge.port == 0 || bridge.ide.trim().is_empty() {
            let _ = fs::remove_file(&path);
            continue;
        }
        if now_ms.saturating_sub(bridge.updated_at_ms) > STALE_BRIDGE_MS {
            let _ = fs::remove_file(&path);
            continue;
        }
        bridges.push(bridge);
    }
    bridges
}

fn pull_bridge(bridge: &BridgeRegistration) -> Result<Option<IDEContext>, String> {
    let body = http_get_localhost(bridge.port, CONTEXT_PATH, PULL_TIMEOUT)?;
    match body {
        None => Ok(None),
        Some(bytes) if bytes.is_empty() => Ok(None),
        Some(bytes) => parse_ide_context_payload(&bytes),
    }
}

fn http_get_localhost(port: u16, path: &str, timeout: Duration) -> Result<Option<Vec<u8>>, String> {
    let started = Instant::now();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    if started.elapsed() > timeout.saturating_mul(2) {
        return Err("bridge pull timed out".to_string());
    }
    parse_http_body(&response)
}

fn parse_http_body(response: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let header_end = find_header_end(response).ok_or_else(|| "incomplete HTTP response".to_string())?;
    let header = std::str::from_utf8(&response[..header_end])
        .map_err(|_| "invalid HTTP header encoding".to_string())?;
    let mut lines = header.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut status_parts = status_line.split_whitespace();
    let _version = status_parts.next();
    let status = status_parts
        .next()
        .ok_or_else(|| "missing HTTP status".to_string())?;
    match status {
        "200" => {}
        "204" => return Ok(None),
        other => return Err(format!("unexpected HTTP status {other}")),
    }

    let body = &response[header_end..];
    if body.is_empty() {
        return Ok(None);
    }
    Ok(Some(body.to_vec()))
}

fn find_header_end(response: &[u8]) -> Option<usize> {
    response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn bridge_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|app_data| PathBuf::from(app_data).join("Anya").join("ide-bridges"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|base| base.join("Anya").join("ide-bridges"))
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::path::Path;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn parse_http_body_handles_204_and_200() {
        let empty = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";
        assert!(parse_http_body(empty).unwrap().is_none());

        let ok = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(parse_http_body(ok).unwrap().as_deref(), Some(&b"{}"[..]));
    }

    #[test]
    fn pull_bridge_reads_json_payload() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let barrier = Arc::new(Barrier::new(2));
        let server_barrier = Arc::clone(&barrier);
        let server = thread::spawn(move || {
            server_barrier.wait();
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            let body = br#"{"provider":"ide","ide":"vscode","selection":{"text":"hello","start_line":1,"end_line":1}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            );
            stream.write_all(response.as_bytes()).expect("write");
        });
        barrier.wait();

        let bridge = BridgeRegistration {
            ide: "vscode".to_string(),
            pid: 1,
            port,
            updated_at_ms: now_millis(),
        };
        let context = pull_bridge(&bridge).expect("pull").expect("selection");
        assert_eq!(context.ide, "vscode");
        assert_eq!(context.selection.as_deref(), Some("hello"));
        server.join().expect("server");
    }

    #[test]
    fn bridge_dir_points_under_anya() {
        let dir = bridge_dir().expect("bridge dir");
        assert!(dir.ends_with(Path::new("Anya").join("ide-bridges")));
    }
}
