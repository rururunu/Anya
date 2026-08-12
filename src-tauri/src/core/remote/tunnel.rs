use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::runtime::terminal::prepare_command;

use super::state::{remote_state, TunnelPrefs, TunnelPublicInfo, TunnelRuntime};

fn extract_trycloudflare_url(line: &str) -> Option<String> {
    // cloudflared generally prints a URL like:
    //   https://xxxxx.trycloudflare.com
    // We avoid regex to keep dependencies low.
    let lower = line.to_ascii_lowercase();
    if !lower.contains(".trycloudflare.com") {
        return None;
    }

    for prefix in ["https://", "http://", "wss://", "ws://"] {
        if let Some(start) = lower.find(prefix) {
            let rest = &line[start + prefix.len()..];
            let end = rest
                .find(|c: char| {
                    c == '/'
                        || c.is_whitespace()
                        || c == '"'
                        || c == '|'
                        || c == ','
                        || c == ')'
                })
                .unwrap_or(rest.len());
            let host = rest[..end].trim();
            if host.contains(".trycloudflare.com") {
                return Some(host.to_string());
            }
        }
    }

    // Fallback: extract `<subdomain>.trycloudflare.com` from boxed log lines.
    if let Some(idx) = lower.find(".trycloudflare.com") {
        let end = idx + ".trycloudflare.com".len();
        let start = lower[..idx]
            .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .map(|i| i + 1)
            .unwrap_or(0);
        let host = line[start..end].trim();
        if host.contains('.') && host.ends_with(".trycloudflare.com") {
            return Some(host.to_string());
        }
    }

    None
}

fn scan_logs_for_host(logs: &[String]) -> Option<String> {
    logs.iter().find_map(|line| extract_trycloudflare_url(line))
}

fn tunnel_scheme() -> String {
    // Phone companion expects WebSocket scheme.
    "wss".to_string()
}

fn tunnel_default_port() -> u16 {
    443
}

fn cloudflared_base_url(local_port: u16) -> String {
    format!("http://127.0.0.1:{}", local_port)
}

fn is_quick_tunnel(prefs: &TunnelPrefs) -> bool {
    let token = prefs
        .cloudflared_token
        .as_deref()
        .unwrap_or("")
        .trim();
    prefs.use_quick_tunnel || token.is_empty()
}

fn hostname_override(prefs: &TunnelPrefs) -> Option<String> {
    // Quick Tunnel hostnames are ephemeral and come from process logs.
    // Only named tunnels may use a saved public hostname.
    if is_quick_tunnel(prefs) {
        return None;
    }
    prefs
        .cloudflared_hostname
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn public_info_from_host(host: String) -> TunnelPublicInfo {
    TunnelPublicInfo {
        host,
        port: tunnel_default_port(),
        scheme: tunnel_scheme(),
    }
}

fn cloudflared_command(prefs: &TunnelPrefs, local_port: u16) -> Result<Command, String> {
    let binary = prefs.cloudflared_binary.trim().to_string();
    if binary.is_empty() {
        return Err("cloudflared binary name is empty".into());
    }

    let origin = cloudflared_base_url(local_port);
    let mut cmd = Command::new(binary);
    cmd.args([
        // Keep tunnel stable across repeated runs.
        "--no-autoupdate",
        "--loglevel",
        "info",
        "tunnel",
    ]);

    let token = prefs
        .cloudflared_token
        .as_deref()
        .unwrap_or("")
        .trim();

    if prefs.use_quick_tunnel || token.is_empty() {
        // Quick tunnel: `cloudflared tunnel --url http://localhost:PORT`
        cmd.args(["--url", origin.as_str()]);
        return Ok(cmd);
    }

    // Named tunnel using token, but still override local origin with --url.
    cmd.args(["--url", origin.as_str(), "run", "--token", token]);
    Ok(cmd)
}

fn spawn_cloudflared(prefs: &TunnelPrefs, local_port: u16) -> Result<Child, String> {
    let binary_hint = prefs.cloudflared_binary.clone();
    let mut cmd = cloudflared_command(prefs, local_port)?;
    // Anya is a GUI app: without this, Windows flashes a visible console
    // window for cloudflared every time the tunnel (re)starts.
    prepare_command(&mut cmd);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "failed to start cloudflared: program not found. Please set `cloudflared 路径/名称` to the full path of `cloudflared.exe` in the 公网连接设置 dialog. Current: {}",
                binary_hint
            )
        } else {
            format!("failed to start cloudflared: {e}")
        }
    })
}

fn attach_output_drainers(child: &mut Child) {
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for _line in reader.lines().flatten() {}
        });
    }
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for _line in reader.lines().flatten() {}
        });
    }
}

/// Resolve a public tunnel endpoint without blocking on Quick Tunnel log parsing.
pub fn resolve_tunnel_public(
    app: &AppHandle,
    local_port: u16,
    ensure_tunnel: bool,
) -> Option<TunnelPublicInfo> {
    if ensure_tunnel {
        match ensure_cloudflared_tunnel(app, local_port, true) {
            Ok(public) => return Some(public),
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "cloudflared ensure failed; trying saved/cached public hostname"
                );
            }
        }
    }
    tunnel_public_info_best_effort(app)
}

/// Non-blocking lookup: runtime cache or saved hostname override.
pub fn tunnel_public_info_best_effort(app: &AppHandle) -> Option<TunnelPublicInfo> {
    let state = remote_state(app);
    if let Some(public) = state.tunnel_public_info() {
        return Some(public);
    }
    let prefs = state.tunnel_prefs();
    if !prefs.cloudflared_enabled {
        return None;
    }
    hostname_override(&prefs).map(public_info_from_host)
}

/// Ensure a Cloudflare tunnel is running and return the public address.
///
/// When `block_for_url` is false, only cached / override hostnames are returned.
pub fn ensure_cloudflared_tunnel(
    app: &AppHandle,
    local_port: u16,
    block_for_url: bool,
) -> Result<TunnelPublicInfo, String> {
    let state = remote_state(app);

    if let Some(public) = state.tunnel_public_info() {
        return Ok(public);
    }

    let prefs = state.tunnel_prefs();

    if !prefs.cloudflared_enabled {
        return Err("public tunnel disabled".into());
    }

    let named_tunnel = !is_quick_tunnel(&prefs);

    // Named tunnel: use saved hostname (required) and spawn connector.
    if let Some(override_host) = hostname_override(&prefs) {
        let public = public_info_from_host(override_host);
        match spawn_cloudflared(&prefs, local_port) {
            Ok(mut child) => {
                attach_output_drainers(&mut child);
                state.set_tunnel_runtime(TunnelRuntime {
                    child,
                    public: public.clone(),
                });
            }
            Err(err) => {
                tracing::warn!(error = %err, "cloudflared spawn failed; using saved hostname");
            }
        }
        return Ok(public);
    }

    if named_tunnel {
        return Err(
            "named Cloudflare tunnel requires a public hostname in settings (e.g. remote.example.com)"
                .into(),
        );
    }

    // Quick Tunnel: spawn cloudflared and parse *.trycloudflare.com from logs.
    if !block_for_url {
        return Err("cloudflared tunnel not ready yet".into());
    }

    let mut child = spawn_cloudflared(&prefs, local_port)?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cloudflared stdout unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "cloudflared stderr unavailable".to_string())?;

    let (tx, rx) = mpsc::channel::<String>();
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));

    {
        let tx = tx.clone();
        let logs = Arc::clone(&logs);
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                logs.lock().ok().map(|mut g| g.push(line.clone()));
                if let Some(host) = extract_trycloudflare_url(&line) {
                    let _ = tx.send(host);
                }
            }
        });
    }
    {
        let tx = tx.clone();
        let logs = Arc::clone(&logs);
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                logs.lock().ok().map(|mut g| g.push(line.clone()));
                if let Some(host) = extract_trycloudflare_url(&line) {
                    let _ = tx.send(host);
                }
            }
        });
    }

    let timeout = Duration::from_secs(25);
    let deadline = Instant::now() + timeout;
    let mut parsed_host: Option<String> = None;

    loop {
        if let Ok(Some(status)) = child.try_wait() {
            if parsed_host.is_none() {
                let log_snapshot = logs.lock().map(|g| g.clone()).unwrap_or_default();
                parsed_host = scan_logs_for_host(&log_snapshot);
                if parsed_host.is_none() {
                    let _ = child.kill();
                    let tail = log_snapshot
                        .into_iter()
                        .rev()
                        .take(4)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join(" | ");
                    return Err(format!(
                        "cloudflared exited before public URL was detected (status={status:?}). Recent logs: {tail}"
                    ));
                }
            }
            break;
        }

        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining.min(Duration::from_millis(250))) {
            Ok(host) => {
                parsed_host = Some(host);
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }

    if parsed_host.is_none() {
        let log_snapshot = logs.lock().map(|g| g.clone()).unwrap_or_default();
        parsed_host = scan_logs_for_host(&log_snapshot);
    }

    let Some(host) = parsed_host else {
        let _ = child.kill();
        let log_snapshot = logs.lock().map(|g| g.clone()).unwrap_or_default();
        let tail = log_snapshot
            .into_iter()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(format!(
            "cloudflared tunnel started but public URL not detected from logs. Check cloudflared path/permissions. Recent logs: {tail}"
        ));
    };

    let public = public_info_from_host(host);

    state.set_tunnel_runtime(TunnelRuntime {
        child,
        public: public.clone(),
    });
    Ok(public)
}

pub fn stop_cloudflared_tunnel(app: &AppHandle) {
    let state = remote_state(app);
    if let Some(mut runtime) = state.take_tunnel_runtime() {
        let _ = runtime.child.kill();
    }
}
