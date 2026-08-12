//! Minimal LSP over stdio JSON-RPC client.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};

use crate::core::tools::error::ToolError;
use crate::models::settings::{AppSettings, LspServerConfig};
use crate::runtime::terminal::prepare_command;

struct LspProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
    diagnostics_cache: HashMap<String, Value>,
}

impl LspProcess {
    fn spawn(command: &str, args: &[String], root: &Path) -> Result<Self, ToolError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        prepare_command(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::new(format!("failed to start LSP `{command}`: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::new("LSP stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::new("LSP stdout unavailable"))?;
        let mut proc = Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
            diagnostics_cache: HashMap::new(),
        };
        let root_uri = path_to_uri(root);
        proc.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "definition": { "linkSupport": true },
                        "publishDiagnostics": {}
                    }
                }
            }),
        )?;
        proc.notify("initialized", json!({}))?;
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
            self.capture_notification(&response);
            if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(error) = response.get("error") {
                    return Err(ToolError::new(format!("LSP error: {error}")));
                }
                return Ok(response.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), ToolError> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.write_message(&msg)
    }

    fn write_message(&mut self, msg: &Value) -> Result<(), ToolError> {
        let body = serde_json::to_vec(msg)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|e| ToolError::new(e.to_string()))?;
        self.stdin
            .write_all(&body)
            .map_err(|e| ToolError::new(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| ToolError::new(e.to_string()))
    }

    fn read_message(&mut self) -> Result<Value, ToolError> {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            self.reader
                .read_line(&mut line)
                .map_err(|e| ToolError::new(e.to_string()))?;
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                content_length = Some(
                    rest.trim()
                        .parse::<usize>()
                        .map_err(|e| ToolError::new(e.to_string()))?,
                );
            }
        }
        let len = content_length.ok_or_else(|| ToolError::new("LSP missing Content-Length"))?;
        let mut buf = vec![0u8; len];
        self.reader
            .read_exact(&mut buf)
            .map_err(|e| ToolError::new(e.to_string()))?;
        serde_json::from_slice(&buf).map_err(|e| ToolError::new(e.to_string()))
    }

    fn capture_notification(&mut self, message: &Value) {
        if message.get("id").is_some() {
            return;
        }
        let Some(method) = message.get("method").and_then(|v| v.as_str()) else {
            return;
        };
        if method != "textDocument/publishDiagnostics" {
            return;
        }
        let Some(params) = message.get("params") else {
            return;
        };
        let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
            return;
        };
        self.diagnostics_cache
            .insert(uri.to_string(), params.clone());
    }

    fn diagnostics_for_uri(&self, uri: &str) -> Option<Value> {
        self.diagnostics_cache.get(uri).cloned()
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub struct LspManager {
    enabled: Mutex<bool>,
    servers: Mutex<Vec<LspServerConfig>>,
    processes: Mutex<HashMap<String, LspProcess>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            enabled: Mutex::new(false),
            servers: Mutex::new(Vec::new()),
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn configure(&self, settings: &AppSettings) {
        if let Ok(mut e) = self.enabled.lock() {
            *e = settings.lsp_enabled;
        }
        if let Ok(mut s) = self.servers.lock() {
            *s = if settings.lsp_servers.is_empty() {
                AppSettings::default().lsp_servers
            } else {
                settings.lsp_servers.clone()
            };
        }
        if let Ok(mut p) = self.processes.lock() {
            p.clear();
        }
    }

    /// Whether LSP is enabled in Settings; the `lsp` tool hides itself when false.
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn language_for_path(path: &str) -> &'static str {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext {
            "rs" => "rust",
            "ts" => "typescript",
            "tsx" => "tsx",
            "js" => "javascript",
            "jsx" => "jsx",
            _ => "plaintext",
        }
    }

    fn ensure_server(&self, language: &str, root: &Path) -> Result<String, ToolError> {
        if !*self.enabled.lock().unwrap_or_else(|e| e.into_inner()) {
            return Err(ToolError::new("LSP is disabled in Settings"));
        }
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?
            .clone();
        let config = servers
            .into_iter()
            .find(|s| s.enabled && s.languages.iter().any(|l| l == language))
            .ok_or_else(|| ToolError::new(format!("no LSP server configured for {language}")))?;
        let key = format!("{}:{}", config.id, root.display());
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        if !procs.contains_key(&key) {
            let proc = LspProcess::spawn(&config.command, &config.args, root)?;
            procs.insert(key.clone(), proc);
        }
        Ok(key)
    }

    pub fn definition(
        &self,
        root: &Path,
        path: &str,
        line: u64,
        character: u64,
    ) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        let result = proc.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }

    pub fn diagnostics(&self, root: &Path, path: &str) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        if let Some(cached) = proc.diagnostics_for_uri(&uri) {
            return Ok(serde_json::to_string_pretty(&cached)?);
        }
        // Pull diagnostics via textDocument/diagnostic if supported; else empty publish wait.
        match proc.request(
            "textDocument/diagnostic",
            json!({ "textDocument": { "uri": uri } }),
        ) {
            Ok(value) => Ok(serde_json::to_string_pretty(&value)?),
            Err(_) => Ok(json!({
                "note": "server did not support textDocument/diagnostic; ensure the file is open in an editor with publishDiagnostics or use a server that supports pull diagnostics",
                "path": path
            })
            .to_string()),
        }
    }

    pub fn hover(
        &self,
        root: &Path,
        path: &str,
        line: u64,
        character: u64,
    ) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        let result = proc.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }

    pub fn references(
        &self,
        root: &Path,
        path: &str,
        line: u64,
        character: u64,
    ) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        let result = proc.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }

    pub fn workspace_symbol(&self, root: &Path, query: &str) -> Result<String, ToolError> {
        let language = "rust";
        let key = self.ensure_server(language, root)?;
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let result = proc.request(
            "workspace/symbol",
            json!({
                "query": query
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }

    pub fn rename(
        &self,
        root: &Path,
        path: &str,
        line: u64,
        character: u64,
        new_name: &str,
    ) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        let result = proc.request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "newName": new_name
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }

    pub fn code_action(
        &self,
        root: &Path,
        path: &str,
        line: u64,
        character: u64,
    ) -> Result<String, ToolError> {
        let language = Self::language_for_path(path);
        let key = self.ensure_server(language, root)?;
        let abs = root.join(path);
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let uri = path_to_uri(&abs);
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("lsp lock"))?;
        let proc = procs
            .get_mut(&key)
            .ok_or_else(|| ToolError::new("LSP process missing"))?;
        let _ = proc.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language,
                    "version": 1,
                    "text": text
                }
            }),
        );
        let result = proc.request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": line, "character": character },
                    "end": { "line": line, "character": character }
                },
                "context": { "diagnostics": [] }
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

fn path_to_uri(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

pub fn shared_lsp_manager() -> Arc<LspManager> {
    static MANAGER: OnceLock<Arc<LspManager>> = OnceLock::new();
    Arc::clone(MANAGER.get_or_init(|| Arc::new(LspManager::new())))
}
