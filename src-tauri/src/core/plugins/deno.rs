//! Deno JSON-RPC bridge for a plugin `host/main.ts`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::grant::get_grant;
use super::locate::{deno_command_candidates, first_existing};
use crate::core::tools::error::ToolError;

const RPC_TIMEOUT: Duration = Duration::from_secs(8);
const HOOK_TIMEOUT: Duration = Duration::from_secs(2);

pub struct DenoBridge {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl DenoBridge {
    pub fn spawn(
        plugin_id: &str,
        plugin_dir: &Path,
        entry: &str,
        workspace: Option<&Path>,
    ) -> Result<Self, ToolError> {
        let deno = first_existing(&deno_command_candidates()).ok_or_else(|| {
            ToolError::new("Deno sidecar not found; run `pnpm fetch-deno` or install deno on PATH")
        })?;
        let script = plugin_dir.join(entry);
        if !script.is_file() {
            return Err(ToolError::new(format!(
                "host entry missing: {}",
                script.display()
            )));
        }
        let cache = super::locate::deno_cache_dir();
        std::fs::create_dir_all(&cache).map_err(|e| ToolError::new(format!("deno cache: {e}")))?;
        let mut cmd = Command::new(&deno);
        cmd.env("DENO_DIR", &cache);
        cmd.arg("run")
            .arg("--quiet")
            .arg("--no-prompt")
            .arg(format!("--allow-read={}", plugin_dir.display()))
            .arg(format!("--allow-write={}", plugin_dir.display()))
            .arg(format!("--allow-read={}", cache.display()))
            .arg(format!("--allow-write={}", cache.display()));
        apply_allow_flags(&mut cmd, plugin_id, plugin_dir, workspace);
        cmd.arg(&script)
            .current_dir(plugin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::new(format!("spawn deno: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::new("deno stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::new("deno stdout unavailable"))?;
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    tracing::debug!(target: "anya.plugin", "deno: {line}");
                }
            });
        }
        let mut bridge = Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        };
        // Drain optional ready event.
        let _ = bridge.read_line_timeout(Duration::from_millis(400));
        Ok(bridge)
    }

    pub fn describe(&mut self) -> Result<Value, ToolError> {
        self.request("describe", json!({}), RPC_TIMEOUT)
    }

    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, ToolError> {
        self.request("tool", json!({ "name": name, "args": args }), RPC_TIMEOUT)
    }

    pub fn call_hook(&mut self, hook: &str, payload: Value) -> Result<Value, ToolError> {
        self.request(
            "hook",
            json!({ "hook": hook, "payload": payload }),
            HOOK_TIMEOUT,
        )
    }

    /// Forward a workbench `ctx.host.rpc` method into `host/main.ts`.
    pub fn call_host(&mut self, method: &str, params: Value) -> Result<Value, ToolError> {
        self.request(method, params, RPC_TIMEOUT)
    }

    fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, ToolError> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = json!({ "id": id, "method": method, "params": params });
        let mut body = serde_json::to_vec(&msg).map_err(|e| ToolError::new(e.to_string()))?;
        body.push(b'\n');
        self.stdin
            .write_all(&body)
            .map_err(|e| ToolError::new(format!("deno write: {e}")))?;
        self.stdin
            .flush()
            .map_err(|e| ToolError::new(format!("deno flush: {e}")))?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ToolError::new(format!("plugin host `{method}` timed out")));
            }
            let line = self.read_line_timeout(remaining)?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|e| ToolError::new(format!("deno json: {e}")))?;
            if value.get("id").and_then(|v| v.as_u64()) != Some(id) {
                continue;
            }
            if let Some(err) = value.get("error") {
                return Err(ToolError::new(format!("plugin host: {err}")));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn read_line_timeout(&mut self, timeout: Duration) -> Result<String, ToolError> {
        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() >= deadline {
                return Err(ToolError::new("deno read timed out"));
            }
            self.reader
                .fill_buf()
                .map_err(|e| ToolError::new(e.to_string()))?;
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| ToolError::new(e.to_string()))?;
            if n == 0 {
                return Err(ToolError::new("deno stdout closed"));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            return Ok(trimmed.to_string());
        }
    }

    pub fn kill(&mut self) {
        let pid = self.child.id();
        let _ = self.child.kill();
        let _ = self.child.wait();
        super::job::kill_process_tree(pid);
    }
}

impl Drop for DenoBridge {
    fn drop(&mut self) {
        self.kill();
    }
}

fn apply_allow_flags(
    cmd: &mut Command,
    plugin_id: &str,
    plugin_dir: &Path,
    workspace: Option<&Path>,
) {
    let grant = get_grant(plugin_id);
    let has = |name: &str| grant.permissions.iter().any(|p| p == name);
    if has("run") {
        cmd.arg("--allow-run");
    }
    if has("net") {
        cmd.arg("--allow-net");
        cmd.arg("--allow-env");
    }
    if has("fs.workspace") {
        if let Some(root) = workspace {
            cmd.arg(format!("--allow-read={}", root.display()));
            cmd.arg(format!("--allow-write={}", root.display()));
        }
    }
    let _ = plugin_dir;
}

#[allow(dead_code)]
pub fn deno_available() -> bool {
    first_existing(&deno_command_candidates()).is_some()
}

#[allow(dead_code)]
pub fn deno_path() -> Option<PathBuf> {
    first_existing(&deno_command_candidates())
}
