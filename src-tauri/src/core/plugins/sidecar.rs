//! Plugin OS host: PTY + Deno spawn. Runs in-process or as `anya-plugin-host`.

use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::deno::DenoBridge;
use super::grant::validate_token;
use super::job::PluginJob;
use super::pty::{HostEventFn, PtyHub};
use crate::core::tools::error::ToolError;

pub struct HostEngine {
    plugin_id: String,
    token: String,
    host_pid: u32,
    authed: bool,
    pty: PtyHub,
    deno: Mutex<Option<DenoBridge>>,
    #[allow(dead_code)]
    job: PluginJob,
}

impl HostEngine {
    pub fn new(plugin_id: String, events: HostEventFn) -> Self {
        Self {
            plugin_id,
            token: String::new(),
            host_pid: std::process::id(),
            authed: false,
            pty: PtyHub::new(events),
            deno: Mutex::new(None),
            job: PluginJob::new(),
        }
    }

    pub fn dispatch(&mut self, method: &str, params: &Value) -> Result<Value, ToolError> {
        match method {
            "hello" => self.hello(params),
            "pty.open" => {
                self.require_auth()?;
                let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                let cwd = params.get("cwd").and_then(|v| v.as_str());
                let shell = params.get("shell").and_then(|v| v.as_str());
                let session = self.pty.open(&self.plugin_id, cols, rows, cwd, shell)?;
                Ok(json!({ "session": session }))
            }
            "pty.write" => {
                self.require_auth()?;
                let session = params
                    .get("session")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::new("session required"))?;
                let data = params.get("data").and_then(|v| v.as_str()).unwrap_or("");
                self.pty.write(session, data)?;
                Ok(json!({ "ok": true }))
            }
            "pty.resize" => {
                self.require_auth()?;
                let session = params
                    .get("session")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::new("session required"))?;
                let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                self.pty.resize(session, cols, rows)?;
                Ok(json!({ "ok": true }))
            }
            "pty.kill" => {
                self.require_auth()?;
                let session = params
                    .get("session")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::new("session required"))?;
                self.pty.kill(session)?;
                Ok(json!({ "ok": true }))
            }
            "shutdown" => {
                self.shutdown();
                Ok(json!({ "ok": true }))
            }
            _ => Err(ToolError::new(format!("unknown host method `{method}`"))),
        }
    }

    fn hello(&mut self, params: &Value) -> Result<Value, ToolError> {
        let plugin_id = params
            .get("pluginId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let token = params.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let host_pid = params.get("hostPid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        if plugin_id != self.plugin_id && !self.plugin_id.is_empty() {
            // Sidecar started for one plugin; in-process engine is created per plugin.
        }
        if !self.plugin_id.is_empty() && plugin_id != self.plugin_id {
            return Err(ToolError::new("plugin id mismatch"));
        }
        if self.plugin_id.is_empty() {
            self.plugin_id = plugin_id.to_string();
        }
        validate_sidecar_or_kernel(plugin_id, token, host_pid)?;
        self.token = token.to_string();
        self.host_pid = host_pid;
        self.authed = true;
        Ok(json!({ "ok": true, "pluginId": self.plugin_id }))
    }

    fn require_auth(&self) -> Result<(), ToolError> {
        if self.authed {
            Ok(())
        } else {
            Err(ToolError::new("plugin host not authenticated"))
        }
    }

    #[allow(dead_code)]
    pub fn attach_deno(&self, bridge: DenoBridge) -> Result<(), ToolError> {
        if let Ok(mut slot) = self.deno.lock() {
            if let Some(mut old) = slot.take() {
                old.kill();
            }
            *slot = Some(bridge);
        }
        let _ = &self.job;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.pty.kill_all();
        if let Ok(mut slot) = self.deno.lock() {
            if let Some(mut deno) = slot.take() {
                deno.kill();
            }
        }
        self.authed = false;
    }
}

impl Drop for HostEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn validate_sidecar_or_kernel(
    plugin_id: &str,
    token: &str,
    host_pid: u32,
) -> Result<(), ToolError> {
    if let Ok(expected) = std::env::var("ANYA_PLUGIN_TOKEN") {
        let env_id = std::env::var("ANYA_PLUGIN_ID").unwrap_or_default();
        let env_pid: u32 = std::env::var("ANYA_HOST_PID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if env_id != plugin_id {
            return Err(ToolError::new("sidecar plugin id mismatch"));
        }
        if expected != token {
            return Err(ToolError::new("sidecar token mismatch"));
        }
        if env_pid != 0 && env_pid != host_pid {
            return Err(ToolError::new("sidecar host pid mismatch"));
        }
        return Ok(());
    }
    validate_token(plugin_id, token, host_pid)
}

/// Stdio NDJSON host used by the `anya-plugin-host` binary.
pub fn run_stdio_host() {
    let plugin_id = std::env::var("ANYA_PLUGIN_ID").unwrap_or_default();
    let stdout = std::io::stdout();
    let events: HostEventFn = Arc::new({
        let stdout = Mutex::new(stdout);
        move |value: Value| {
            if let Ok(mut out) = stdout.lock() {
                if let Ok(mut line) = serde_json::to_vec(&value) {
                    line.push(b'\n');
                    let _ = out.write_all(&line);
                    let _ = out.flush();
                }
            }
        }
    });
    let mut engine = HostEngine::new(plugin_id, events);
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let reply = match engine.dispatch(method, &params) {
            Ok(result) => json!({ "id": id, "ok": true, "result": result }),
            Err(err) => json!({ "id": id, "ok": false, "error": err.to_string() }),
        };
        if let Ok(mut line) = serde_json::to_vec(&reply) {
            line.push(b'\n');
            let _ = std::io::stdout().write_all(&line);
            let _ = std::io::stdout().flush();
        }
        if method == "shutdown" {
            break;
        }
    }
}

pub struct SidecarClient {
    child: std::process::Child,
    stdin: Mutex<std::process::ChildStdin>,
    next_id: std::sync::atomic::AtomicU64,
    pending: Arc<Mutex<std::collections::HashMap<u64, std::sync::mpsc::Sender<Value>>>>,
}

impl SidecarClient {
    pub fn spawn(plugin_id: &str, token: &str, events: HostEventFn) -> Result<Self, ToolError> {
        let host = super::locate::first_existing(&super::locate::plugin_host_candidates())
            .ok_or_else(|| ToolError::new("anya-plugin-host not found"))?;
        let mut cmd = std::process::Command::new(host);
        cmd.env("ANYA_PLUGIN_ID", plugin_id)
            .env("ANYA_PLUGIN_TOKEN", token)
            .env("ANYA_HOST_PID", std::process::id().to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::new(format!("spawn plugin host: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::new("plugin host stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::new("plugin host stdout"))?;
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    tracing::debug!(target: "anya.plugin", "host: {line}");
                }
            });
        }
        let pending: Arc<Mutex<std::collections::HashMap<u64, std::sync::mpsc::Sender<Value>>>> =
            Arc::new(Mutex::new(std::collections::HashMap::new()));
        let pending_reader = Arc::clone(&pending);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                    continue;
                };
                if value.get("event").is_some() {
                    events(value);
                    continue;
                }
                if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                    if let Ok(mut map) = pending_reader.lock() {
                        if let Some(tx) = map.remove(&id) {
                            let _ = tx.send(value);
                        }
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin: Mutex::new(stdin),
            next_id: std::sync::atomic::AtomicU64::new(1),
            pending,
        })
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, ToolError> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending
            .lock()
            .map_err(|_| ToolError::new("sidecar pending lock"))?
            .insert(id, tx);
        let msg = json!({ "id": id, "method": method, "params": params });
        let mut body = serde_json::to_vec(&msg).map_err(|e| ToolError::new(e.to_string()))?;
        body.push(b'\n');
        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|_| ToolError::new("sidecar stdin lock"))?;
            stdin
                .write_all(&body)
                .map_err(|e| ToolError::new(e.to_string()))?;
            stdin.flush().map_err(|e| ToolError::new(e.to_string()))?;
        }
        let value = rx
            .recv_timeout(std::time::Duration::from_secs(8))
            .map_err(|_| ToolError::new("plugin host timed out"))?;
        if value.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            return Err(ToolError::new(
                value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("plugin host error")
                    .to_string(),
            ));
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn kill(&mut self) {
        super::job::kill_process_tree(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for SidecarClient {
    fn drop(&mut self) {
        self.kill();
    }
}
