//! Private stdio connection to the official Ghost MCP executable.
use std::{io, path::Path, process::Stdio, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};

pub(super) struct GhostProcess {
    child: Child,
    input: Option<ChildStdin>,
    output: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    broken: bool,
}

impl GhostProcess {
    pub(super) async fn start(exe: &Path, version: &str) -> Result<Self, String> {
        let mut child = Command::new(exe)
            .current_dir(
                exe.parent()
                    .ok_or("Ghost executable has no parent directory")?,
            )
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .env("GHOST_SHELL", "off")
            .env("GHOST_FOCUS_POLICY", "background")
            .env("GHOST_FOCUS_LOCK", "off")
            .env_remove("GHOST_HTTP_ALLOW_PRIVATE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| format!("Start Ghost MCP: {err}"))?;
        let mut process = Self {
            input: child.stdin.take(),
            output: BufReader::new(child.stdout.take().ok_or("Ghost MCP stdout unavailable")?)
                .lines(),
            child,
            next_id: 0,
            broken: false,
        };
        let handshake =
            tokio::time::timeout(Duration::from_secs(8), process.handshake(version)).await;
        match handshake {
            Ok(Ok(())) => Ok(process),
            failure => {
                process.stop(false).await;
                Err(match failure {
                    Ok(Err(err)) => err,
                    _ => "Ghost MCP handshake timed out".into(),
                })
            }
        }
    }

    async fn handshake(&mut self, version: &str) -> Result<(), String> {
        let protocol = "2024-11-05";
        let info = self
            .rpc(
                "initialize",
                json!({
                    "protocolVersion": protocol,
                    "capabilities": {},
                    "clientInfo": {"name": "Anya", "version": env!("CARGO_PKG_VERSION")}
                }),
            )
            .await?;
        if info["protocolVersion"] != protocol
            || info["serverInfo"]["name"] != "ghost"
            || info["serverInfo"]["version"] != version
        {
            return Err(format!(
                "Unexpected Ghost MCP identity for version {version}: {info}"
            ));
        }
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await?;
        let list = self.rpc("tools/list", json!({})).await?;
        let tools = list["tools"]
            .as_array()
            .ok_or("Ghost MCP returned no tool list")?;
        for name in super::ghost_ops::required_tools() {
            if !tools
                .iter()
                .any(|tool| tool["name"] == name && tool["inputSchema"].is_object())
            {
                return Err(format!("Ghost MCP {version} is missing tool {name}"));
            }
        }
        // Ghost's lean tool list omits the legacy pixel click RPC. The
        // required-coordinate error verifies that route without sending input.
        if !tools.iter().any(|tool| tool["name"] == "ghost_click_at") {
            match self.call("ghost_click_at", json!({})).await {
                Err(err) if err == "missing param: x" => {}
                _ => {
                    return Err(format!(
                        "Ghost MCP {version} has no compatible pixel click route"
                    ))
                }
            }
        }
        Ok(())
    }

    pub(super) async fn stop(mut self, interrupt: bool) {
        if interrupt {
            // Ghost handles this notification on its stdin thread and releases
            // modifiers even while the previous tool is still running.
            let _ = tokio::time::timeout(
                Duration::from_millis(200),
                self.send(json!({
                    "jsonrpc":"2.0","method":"tools/call",
                    "params":{"name":"ghost_stop","arguments":{}}
                })),
            )
            .await;
        }
        drop(self.input.take());
        if tokio::time::timeout(Duration::from_millis(500), self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
        }
    }

    pub(super) fn transport_failed(&self) -> bool {
        self.broken
    }

    pub(super) async fn call(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        let result = self
            .rpc("tools/call", json!({"name": name, "arguments": arguments}))
            .await?;
        let content = result["content"]
            .as_array()
            .ok_or("Ghost MCP returned no content")?;
        let mut data = result.get("structuredContent").cloned();
        for item in content {
            if let Some(text) = item["text"].as_str() {
                let body = serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!(text));
                if result["isError"] == true || body["ok"] == false {
                    let error = body["error"].as_str().unwrap_or(text);
                    if error.contains("work was not cancelled") {
                        self.broken = true;
                    }
                    return Err(error.to_string());
                }
                data.get_or_insert_with(|| body.get("data").cloned().unwrap_or(body));
            }
        }
        let mut data = data.unwrap_or_else(|| json!({}));
        if let Some(image) = content.iter().find(|item| item["type"] == "image") {
            let key = match image["mimeType"].as_str() {
                Some("image/png") => "png_base64",
                Some("image/jpeg") => "jpeg_base64",
                _ => return Err("Unsupported Ghost MCP screenshot format".into()),
            };
            if !data.is_object() {
                data = json!({"text": data});
            }
            data[key] = image["data"].clone();
        }
        Ok(data)
    }

    // Consume replies serially. A cancelled read invalidates the entire connection.
    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        if self.broken {
            return Err("Ghost MCP connection is closed".into());
        }
        self.broken = true;
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await?;
        while let Some(line) = self.output.next_line().await.map_err(io_error)? {
            let reply: Value = serde_json::from_str(&line)
                .map_err(|err| format!("Invalid Ghost MCP response: {err}"))?;
            if reply["id"] != id {
                continue;
            }
            self.broken = false;
            if let Some(error) = reply.get("error") {
                return Err(format!("Ghost MCP {method}: {error}"));
            }
            return reply
                .get("result")
                .cloned()
                .ok_or_else(|| format!("Ghost MCP {method}: missing result"));
        }
        Err("Ghost MCP closed its connection. Inspect the target before retrying any dispatched action.".into())
    }

    async fn send(&mut self, message: Value) -> Result<(), String> {
        let mut line = message.to_string();
        line.push('\n');
        let input = self.input.as_mut().ok_or("Ghost MCP stdin unavailable")?;
        input.write_all(line.as_bytes()).await.map_err(io_error)?;
        input.flush().await.map_err(io_error)
    }
}

fn io_error(err: io::Error) -> String {
    format!("Ghost MCP connection failed: {err}")
}
