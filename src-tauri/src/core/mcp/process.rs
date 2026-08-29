use crate::core::tools::error::ToolError;
use crate::models::settings::McpServerConfig;
use crate::runtime::terminal::prepare_command;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::command::{build_mcp_command, forward_mcp_stderr, kill_process_tree, mcp_remote_uses_smithery_key_auth, spawn_config};
use super::remote_auth;

pub(super) struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    pub(in crate::core::mcp) fn spawn(config: &McpServerConfig) -> Result<Self, ToolError> {
        let config = spawn_config(config);
        let keyed = mcp_remote_uses_smithery_key_auth(&config);
        let needs_oauth = remote_auth::uses_mcp_remote(&config)
            && !keyed
            && !remote_auth::has_saved_credentials(&config);
        let timeout = if needs_oauth {
            Duration::from_secs(200)
        } else {
            Duration::from_secs(60)
        };

        let pid_slot: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let pid_for_thread = Arc::clone(&pid_slot);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = Self::spawn_and_handshake(config, pid_for_thread);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => {
                if let Ok(guard) = pid_slot.lock() {
                    if let Some(pid) = *guard {
                        kill_process_tree(pid);
                    }
                }
                Err(ToolError::new(
                    "MCP connect timed out waiting for OAuth/handshake. If no browser opened, click Connect again, check the server URL/network, or re-authenticate.",
                ))
            }
        }
    }

    fn spawn_and_handshake(
        config: McpServerConfig,
        pid_slot: Arc<Mutex<Option<u32>>>,
    ) -> Result<Self, ToolError> {
        let (mut cmd, resolved) = build_mcp_command(&config)?;
        eprintln!(
            "MCP `{}` launching via: {resolved} {:?}",
            config.command, config.args
        );
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        prepare_command(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| {
            ToolError::new(format!(
                "failed to start MCP `{}` via `{resolved}`: {e}",
                config.command
            ))
        })?;
        if let Ok(mut guard) = pid_slot.lock() {
            *guard = Some(child.id());
        }
        if let Some(stderr) = child.stderr.take() {
            let open_oauth = !mcp_remote_uses_smithery_key_auth(&config);
            forward_mcp_stderr(stderr, config.id.clone(), open_oauth);
        }
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::new("MCP stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::new("MCP stdout unavailable"))?;
        let mut proc = Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        };
        let _ = proc.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "Anya", "version": "0.1.2" }
            }),
        )?;
        proc.notify("notifications/initialized", json!({}))?;
        Ok(proc)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, ToolError> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_message(&msg)?;
        loop {
            let response = self.read_message()?;
            if response.get("id").and_then(|v| v.as_u64()) == Some(id)
                || response.get("id").and_then(|v| v.as_i64()) == Some(id as i64)
            {
                if let Some(error) = response.get("error") {
                    return Err(ToolError::new(format!("MCP error: {error}")));
                }
                return Ok(response.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), ToolError> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn write_message(&mut self, msg: &Value) -> Result<(), ToolError> {
        let mut body = serde_json::to_vec(msg)?;
        if body.contains(&b'\n') {
            return Err(ToolError::new(
                "MCP message must not contain embedded newlines",
            ));
        }
        body.push(b'\n');
        self.stdin
            .write_all(&body)
            .map_err(|e| ToolError::new(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| ToolError::new(e.to_string()))
    }

    fn read_message(&mut self) -> Result<Value, ToolError> {
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| ToolError::new(e.to_string()))?;
            if n == 0 {
                return Err(ToolError::new("MCP stdout closed"));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("content-length:") {
                continue;
            }
            return serde_json::from_str(trimmed).map_err(|e| {
                ToolError::new(format!("invalid MCP JSON line: {e}; line={trimmed}"))
            });
        }
    }

    pub(in crate::core::mcp) fn list_tools(&mut self) -> Result<Vec<Value>, ToolError> {
        let result = self.request("tools/list", json!({}))?;
        Ok(result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub(super) fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String, ToolError> {
        let result = self.request(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )?;
        let pretty = serde_json::to_string_pretty(&result)?;
        Ok(rewrite_broken_arcade_google_auth(&pretty))
    }
}

/// Arcade Gmail tools often return a Google OAuth URL whose final redirect is
/// `example.com/?flow_id=…` (placeholder verifier). That flow cannot complete in
/// Anya — rewrite the tool payload so the model steers users to Smithery instead.
fn rewrite_broken_arcade_google_auth(payload: &str) -> String {
    let lower = payload.to_ascii_lowercase();
    let looks_arcade_google = lower.contains("accounts.google.com")
        && (lower.contains("cloud.arcade.dev") || lower.contains("arcade.dev"));
    if !looks_arcade_google {
        return payload.to_string();
    }
    format!(
        "{payload}\n\n\
         [Anya notice] The Google→Arcade authorization link above cannot finish in this app \
         (callback lands on example.com and never confirms). Do NOT ask the user to open that \
         Google URL again. Instruct them to open Settings → MCP → the Gmail server → \
         「去 Smithery 授权」, complete OAuth on smithery.ai until oauth-callback?success=true, \
         then click 「连接」, and retry the mail request."
    )
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
