//! Minimal MCP stdio JSON-RPC client (tools/list + tools/call).

mod remote_auth;

pub use remote_auth::{
    clear_saved_credentials, init_mcp_remote_config_dir, normalize_mcp_servers, uses_mcp_remote,
    McpServerRuntimeStatus,
};

use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::models::settings::{AppSettings, McpServerConfig};
use crate::runtime::terminal::prepare_command;

/// Extra dirs GUI apps often miss (nvm, hermes, Volta, Scoop, system Node).
fn known_node_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(pf) = env::var("ProgramFiles") {
        dirs.push(PathBuf::from(pf).join("nodejs"));
    }
    if let Ok(pf86) = env::var("ProgramFiles(x86)") {
        dirs.push(PathBuf::from(pf86).join("nodejs"));
    }
    if let Ok(local) = env::var("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        dirs.push(local.join("Programs").join("nodejs"));
        dirs.push(local.join("hermes").join("node"));
        dirs.push(local.join("fnm"));
        dirs.push(local.join("Volta").join("bin"));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let appdata = PathBuf::from(appdata);
        dirs.push(appdata.join("npm"));
        dirs.push(appdata.join("nvm"));
    }
    if let Ok(userprofile) = env::var("USERPROFILE") {
        let home = PathBuf::from(userprofile);
        dirs.push(home.join(".volta").join("bin"));
        dirs.push(home.join("scoop").join("shims"));
        dirs.push(home.join("AppData").join("Roaming").join("npm"));
        dirs.push(home.join(".local").join("bin"));
    }
    // Common nvm-windows symlink / install roots
    dirs.push(PathBuf::from(r"C:\nvm4w\nodejs"));
    dirs.push(PathBuf::from(r"C:\Program Files\nodejs"));
    dirs
}

fn current_path_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for key in ["PATH", "Path"] {
        if let Ok(value) = env::var(key) {
            out.extend(env::split_paths(&value));
        }
    }
    out
}

fn enriched_path_value() -> Option<std::ffi::OsString> {
    let mut dirs = current_path_dirs();
    for dir in known_node_bin_dirs() {
        if dir.is_dir() && !dirs.iter().any(|d| d == &dir) {
            dirs.push(dir);
        }
    }
    env::join_paths(dirs).ok()
}

fn file_exists(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
fn windows_shim_candidates(name: &str) -> Vec<String> {
    // Prefer `.cmd`/`.exe` — nvm's bare `npx`/`npm` files are shell scripts and
    // CreateProcess returns os error 193 (%1 is not a valid Win32 application).
    if name.contains('.') {
        vec![name.to_string()]
    } else {
        vec![
            format!("{name}.cmd"),
            format!("{name}.exe"),
            format!("{name}.bat"),
            format!("{name}.com"),
        ]
    }
}

fn look_for_command(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates = windows_shim_candidates(name);
    #[cfg(not(windows))]
    let candidates = vec![name.to_string()];
    for dir in dirs {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if file_exists(&path) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(windows)]
fn prefer_win32_executable(path: PathBuf) -> PathBuf {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com") {
        return path;
    }
    for ext in ["cmd", "exe", "bat"] {
        let sibling = path.with_extension(ext);
        if file_exists(&sibling) {
            return sibling;
        }
    }
    path
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = current_path_dirs();
    dirs.extend(known_node_bin_dirs());
    dirs
}

pub fn find_node_exe() -> Option<PathBuf> {
    look_for_command("node", &search_dirs())
}

/// Resolve npm's JS entry (e.g. `npx-cli.js`) next to `node.exe`.
pub fn find_npm_js_cli(cli_file: &str) -> Option<PathBuf> {
    let node = find_node_exe()?;
    let cli = node
        .parent()?
        .join("node_modules")
        .join("npm")
        .join("bin")
        .join(cli_file);
    file_exists(&cli).then_some(cli)
}

pub fn find_uvx_exe() -> Option<PathBuf> {
    look_for_command("uvx", &search_dirs())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRuntimeSupport {
    /// Can launch npm-based stdio servers (`npx` / `node npx-cli.js`).
    pub npm: bool,
    /// Can launch PyPI-based stdio servers (`uvx`).
    pub pypi: bool,
    pub node_path: Option<String>,
    pub npx_cli_path: Option<String>,
    pub uvx_path: Option<String>,
}

pub fn runtime_support() -> McpRuntimeSupport {
    let node = find_node_exe();
    let npx_cli = find_npm_js_cli("npx-cli.js");
    let uvx = find_uvx_exe();
    McpRuntimeSupport {
        npm: node.is_some() && npx_cli.is_some(),
        pypi: uvx.is_some(),
        node_path: node.map(|p| p.to_string_lossy().into_owned()),
        npx_cli_path: npx_cli.map(|p| p.to_string_lossy().into_owned()),
        uvx_path: uvx.map(|p| p.to_string_lossy().into_owned()),
    }
}

fn resolve_mcp_program(command: &str) -> PathBuf {
    let as_path = PathBuf::from(command);
    if as_path.components().count() > 1 || file_exists(&as_path) {
        #[cfg(windows)]
        {
            return prefer_win32_executable(as_path);
        }
        #[cfg(not(windows))]
        {
            return as_path;
        }
    }

    if let Some(found) = look_for_command(command, &search_dirs()) {
        return found;
    }

    as_path
}

fn apply_mcp_env(cmd: &mut Command, config: &McpServerConfig) {
    if let Some(path) = enriched_path_value() {
        cmd.env("PATH", &path);
        #[cfg(windows)]
        cmd.env("Path", &path);
    }
    for (k, v) in &config.env {
        cmd.env(k, v);
    }
    // Keep OAuth tokens under our app config dir so restarts/reinstalls of the
    // npm package do not silently "forget" a completed browser login.
    // Users can still override via an explicit MCP_REMOTE_CONFIG_DIR env entry.
    if remote_auth::uses_mcp_remote(config)
        && !config
            .env
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("MCP_REMOTE_CONFIG_DIR"))
    {
        if let Some(dir) = remote_auth::mcp_remote_config_dir() {
            cmd.env("MCP_REMOTE_CONFIG_DIR", dir);
        }
    }
}

fn mcp_remote_uses_smithery_key_auth(config: &McpServerConfig) -> bool {
    remote_auth::mcp_remote_server_url(config)
        .map(|url| {
            let lower = url.to_ascii_lowercase();
            lower.contains("api_key=") || remote_auth::is_smithery_connect_proxy_url(&url)
        })
        .unwrap_or(false)
}

/// Clone of a server config with mcp-remote package args pinned for this spawn.
fn spawn_config(config: &McpServerConfig) -> McpServerConfig {
    let mut next = config.clone();
    let _ = remote_auth::pin_mcp_remote_args(&mut next.args);
    // Prefer Smithery API key over local browser OAuth when available.
    let _ = remote_auth::inject_smithery_api_key_args(&mut next.args);
    // Only add a long auth timeout when we still expect a browser OAuth wait
    // (no Smithery API key on the URL and no saved mcp-remote tokens).
    let url = remote_auth::mcp_remote_server_url(&next).unwrap_or_default();
    let keyed = url.to_ascii_lowercase().contains("api_key=")
        || remote_auth::is_smithery_connect_proxy_url(&url);
    if remote_auth::uses_mcp_remote(&next)
        && !keyed
        && !remote_auth::has_saved_credentials(&next)
        && !next
            .args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("--auth-timeout"))
    {
        next.args.push("--auth-timeout".into());
        next.args.push("180".into());
    }
    next
}

fn open_url_in_browser(url: &str) {
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    // Never open placeholder / intercepted callback hosts.
    let lower = url.to_ascii_lowercase();
    if lower.contains("example.com") {
        eprintln!("MCP OAuth: refusing to open example.com callback stub → {url}");
        return;
    }
    eprintln!("MCP OAuth: opening browser → {url}");
    #[cfg(windows)]
    {
        let _ = Command::new("cmd.exe")
            .args(["/C", "start", "", url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Real browser consent pages only — not discovery hosts / incomplete authorize stubs.
fn looks_like_authorization_page_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    if lower.contains("example.com") {
        return false;
    }
    // Incomplete Smithery authorize stubs (often missing client_id) just error.
    if lower.contains("/authorize") && !lower.contains("client_id=") {
        return false;
    }
    // Prefer real consent hosts; skip bare smithery server catalog pages.
    if lower.contains("smithery.ai/servers/") {
        return false;
    }
    lower.contains("oauth/authorize")
        || lower.contains("/authorize?")
        || lower.contains("/authorize/")
        || lower.contains("client_id=")
        || lower.contains("authorization_session_id=")
        || lower.contains("authk.smithery.ai")
        || lower.contains("accounts.google.com")
        || lower.contains("login.microsoftonline.com")
        || lower.contains("github.com/login/oauth")
        || lower.contains("cloud.arcade.dev")
}

fn line_announces_auth_url(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("please authorize")
        || lower.contains("visiting:")
        || lower.contains("authorization url")
        || lower.contains("open this url")
        || lower.contains("could not open browser")
}

fn extract_urls(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        let http = rest.find("http://");
        let https = rest.find("https://");
        let start = match (http, https) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };
        let slice = &rest[start..];
        let end = slice
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\'' | '<' | '>'))
            .unwrap_or(slice.len());
        let url = slice[..end].trim_end_matches(['.', ',', ';']).to_string();
        if !url.is_empty() {
            out.push(url);
        }
        rest = &slice[end.max(1)..];
    }
    out
}

fn forward_mcp_stderr(stderr: std::process::ChildStderr, server_id: String, open_oauth_urls: bool) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut expect_auth_url = false;
        let mut opened = false;
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            eprintln!("[mcp:{server_id}] {line}");
            if !open_oauth_urls || opened {
                continue;
            }
            if line_announces_auth_url(&line) {
                expect_auth_url = true;
            }
            let urls = extract_urls(&line);
            if urls.is_empty() {
                continue;
            }
            for url in urls {
                let lower = url.to_ascii_lowercase();
                // Never open truncated authorize stubs (missing client_id → Smithery invalid_request).
                if lower.contains("/authorize") && !lower.contains("client_id=") {
                    eprintln!(
                        "[mcp:{server_id}] skip incomplete authorize URL (missing client_id)"
                    );
                    continue;
                }
                if lower.contains("example.com") {
                    eprintln!("[mcp:{server_id}] skip example.com OAuth stub");
                    continue;
                }
                if expect_auth_url || looks_like_authorization_page_url(&url) {
                    open_url_in_browser(&url);
                    expect_auth_url = false;
                    opened = true;
                    break;
                }
            }
        }
    });
}

fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

fn quote_cmd_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if arg.chars().any(|c| c.is_whitespace() || c == '"') {
        format!("\"{}\"", arg.replace('"', "\"\""))
    } else {
        arg.to_string()
    }
}

/// Build a spawnable command. On Windows, prefer `node.exe npx-cli.js` so we never
/// CreateProcess the extensionless nvm `npx` shim (os error 193).
fn build_mcp_command(config: &McpServerConfig) -> Result<(Command, String), ToolError> {
    let command = config.command.trim();
    let lower = command.to_ascii_lowercase();

    #[cfg(windows)]
    {
        if lower == "npx" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npx-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
        if lower == "npm" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npm-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
    }

    #[cfg(not(windows))]
    {
        if lower == "npx" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npx-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
    }

    let program = resolve_mcp_program(command);

    #[cfg(windows)]
    {
        let ext = program
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let need_cmd = matches!(ext.as_str(), "cmd" | "bat")
            || matches!(
                lower.as_str(),
                "npx" | "npm" | "uvx" | "pnpm" | "yarn" | "bun" | "deno"
            );
        if need_cmd {
            // Never pass an extensionless shim path into CreateProcess.
            let launch = if matches!(ext.as_str(), "cmd" | "bat" | "exe" | "com") {
                program.clone()
            } else if let Some(found) = look_for_command(command, &search_dirs()) {
                found
            } else {
                PathBuf::from(format!("{command}.cmd"))
            };
            let mut parts = vec![quote_cmd_arg(&launch.to_string_lossy())];
            parts.extend(config.args.iter().map(|a| quote_cmd_arg(a)));
            let line = parts.join(" ");
            let summary = format!("cmd.exe /C {line}");
            let mut cmd = Command::new("cmd.exe");
            cmd.args(["/D", "/S", "/C", &line]);
            apply_mcp_env(&mut cmd, config);
            return Ok((cmd, summary));
        }
    }

    if !file_exists(&program) && program.components().count() == 1 {
        return Err(ToolError::new(format!(
            "cannot find MCP program `{command}` on PATH (looked in Node/nvm dirs). Install Node.js or set a full path in MCP settings."
        )));
    }

    let summary = program.display().to_string();
    let mut cmd = Command::new(&program);
    cmd.args(&config.args);
    apply_mcp_env(&mut cmd, config);
    Ok((cmd, summary))
}

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    fn spawn(config: &McpServerConfig) -> Result<Self, ToolError> {
        // Pin at spawn-time so older settings.json entries still reuse one token dir.
        let config = spawn_config(config);
        let keyed = mcp_remote_uses_smithery_key_auth(&config);
        let needs_oauth = remote_auth::uses_mcp_remote(&config)
            && !keyed
            && !remote_auth::has_saved_credentials(&config);
        // OAuth may wait on the browser; discovery/network hangs must not block forever.
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
            // API-key Smithery connections: do not auto-open local OAuth tabs.
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
        // MCP stdio transport: newline-delimited JSON (not LSP Content-Length framing).
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
            // Defensive: ignore accidental LSP-style headers if a server emits them.
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("content-length:") {
                continue;
            }
            return serde_json::from_str(trimmed).map_err(|e| {
                ToolError::new(format!("invalid MCP JSON line: {e}; line={trimmed}"))
            });
        }
    }

    fn list_tools(&mut self) -> Result<Vec<Value>, ToolError> {
        let result = self.request("tools/list", json!({}))?;
        Ok(result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String, ToolError> {
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

struct NamedMcpTool {
    full_name: String,
    server_id: String,
    local_name: String,
    description: String,
    input_schema: Value,
}

impl Tool for NamedMcpTool {
    fn name(&self) -> &str {
        &self.full_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }
    /// Hidden from model-facing schemas when the owning server is disconnected.
    fn available(&self) -> bool {
        shared_mcp_manager().is_server_connected(&self.server_id)
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        shared_mcp_manager().call(&self.server_id, &self.local_name, args)
    }
}

pub struct McpManager {
    servers: Mutex<Vec<McpServerConfig>>,
    processes: Mutex<HashMap<String, McpProcess>>,
    /// Serialize connect/reconnect so settings-save registration and UI actions
    /// cannot open two OAuth browser flows for the same server at once.
    connect_lock: Mutex<()>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(Vec::new()),
            processes: Mutex::new(HashMap::new()),
            connect_lock: Mutex::new(()),
        }
    }

    pub fn configure(&self, settings: &AppSettings) {
        remote_auth::configure_smithery_api_key(&settings.smithery_api_key);
        let mut servers = settings.mcp_servers.clone();
        let _ = remote_auth::normalize_mcp_servers(&mut servers);
        if let Ok(mut s) = self.servers.lock() {
            *s = servers;
        }
        if let Ok(mut p) = self.processes.lock() {
            p.clear();
        }
    }

    pub fn list_runtime_statuses(&self) -> Vec<remote_auth::McpServerRuntimeStatus> {
        let servers = self
            .servers
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        servers
            .iter()
            .map(|server| {
                remote_auth::runtime_status_for(server, self.is_server_connected(&server.id))
            })
            .collect()
    }

    pub fn runtime_status_by_id(&self, id: &str) -> Option<remote_auth::McpServerRuntimeStatus> {
        let servers = self.servers.lock().ok()?;
        let server = servers.iter().find(|s| s.id == id)?;
        Some(remote_auth::runtime_status_for(
            server,
            self.is_server_connected(&server.id),
        ))
    }

    pub fn find_server(&self, id: &str) -> Option<McpServerConfig> {
        self.servers
            .lock()
            .ok()
            .and_then(|servers| servers.iter().find(|s| s.id == id).cloned())
    }

    /// Whether at least one enabled MCP server is configured; the `connect_tools`
    /// tool hides itself when there is nothing to connect.
    pub fn has_enabled_servers(&self) -> bool {
        self.servers
            .lock()
            .ok()
            .is_some_and(|servers| servers.iter().any(|server| server.enabled))
    }

    /// Whether a server's process is currently alive (its tools are usable).
    pub fn is_server_connected(&self, server_id: &str) -> bool {
        self.processes
            .lock()
            .ok()
            .is_some_and(|processes| processes.contains_key(server_id))
    }

    pub fn register_enabled(&self, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;

        // Drop stale dynamic tools / processes before reconnecting.
        registry.unregister_dynamic_prefix("mcp__");
        if let Ok(mut procs) = self.processes.lock() {
            procs.clear();
        }

        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let mut count = 0usize;
        let mut budget = crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS;
        for server in servers.into_iter().filter(|s| s.enabled) {
            // Never auto-connect mcp-remote bridges on startup/settings-save.
            // Smithery OAuth via local browser is unreliable; users connect explicitly
            // (preferably with a Smithery API key so no browser OAuth is needed).
            if remote_auth::uses_mcp_remote(&server) {
                eprintln!(
                    "MCP server `{}` skipped auto-connect (mcp-remote; use Connect in Settings)",
                    server.id
                );
                continue;
            }
            if budget == 0 {
                eprintln!(
                    "MCP registration stopped: reached MCP_MAX_TOTAL_TOOLS ({})",
                    crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS
                );
                break;
            }
            match self.connect_server_with_budget(&server, registry, budget) {
                Ok(n) => {
                    count += n;
                    budget = budget.saturating_sub(n);
                }
                Err(error) => {
                    eprintln!("MCP server `{}` failed to connect: {error}", server.id);
                }
            }
        }
        Ok(count)
    }

    pub fn connect_server(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
    ) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;
        self.connect_server_with_budget(
            server,
            registry,
            crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS,
        )
    }

    fn connect_server_with_budget(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
        remaining_budget: usize,
    ) -> Result<usize, ToolError> {
        let mut proc = McpProcess::spawn(server)?;
        let tools = proc.list_tools()?;
        {
            let mut procs = self
                .processes
                .lock()
                .map_err(|_| ToolError::new("mcp lock"))?;
            procs.insert(server.id.clone(), proc);
        }
        let per_server_cap =
            crate::core::chat::limits::MCP_MAX_TOOLS_PER_SERVER.min(remaining_budget);
        let mut registered = 0usize;
        let mut skipped = 0usize;
        for tool in tools {
            if registered >= per_server_cap {
                skipped += 1;
                continue;
            }
            let name = tool
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let description = truncate_mcp_text(
                tool.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("MCP tool"),
                crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS / 4,
            );
            let input_schema = truncate_mcp_schema(
                tool.get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            );
            let full_name = format!("mcp__{}__{name}", server.id);
            registry.register_dynamic(Arc::new(NamedMcpTool {
                full_name,
                server_id: server.id.clone(),
                local_name: name,
                description,
                input_schema,
            }));
            registered += 1;
        }
        if skipped > 0 {
            eprintln!(
                "MCP server `{}`: registered {registered} tools, skipped {skipped} (cap {})",
                server.id, per_server_cap
            );
        }
        Ok(registered)
    }

    pub fn connect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ToolError::new(format!("unknown MCP server `{id}`")))?;
        self.connect_server(&server, registry)
    }

    pub fn disconnect_by_id(&self, id: &str, registry: &ToolRegistry) {
        if let Ok(mut procs) = self.processes.lock() {
            procs.remove(id);
        }
        registry.unregister_dynamic_prefix(&format!("mcp__{id}__"));
    }

    pub fn reconnect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let _connect_guard = self
            .connect_lock
            .lock()
            .map_err(|_| ToolError::new("mcp connect lock"))?;
        self.disconnect_by_id(id, registry);
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ToolError::new(format!("unknown MCP server `{id}`")))?;
        self.connect_server_with_budget(
            &server,
            registry,
            crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS,
        )
    }

    pub fn call(&self, server_id: &str, tool_name: &str, args: Value) -> Result<String, ToolError> {
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?;
        let proc = procs
            .get_mut(server_id)
            .ok_or_else(|| ToolError::new(format!("MCP server `{server_id}` is not connected")))?;
        proc.call_tool(tool_name, args)
    }
}

fn truncate_mcp_text(text: &str, max_chars: usize) -> String {
    crate::core::chat::limits::truncate_chars(text, max_chars)
}

fn truncate_mcp_schema(schema: Value) -> Value {
    let serialized = schema.to_string();
    if serialized.chars().count() <= crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS {
        return schema;
    }
    // Fall back to a minimal object schema when the upstream schema is enormous.
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true,
        "description": format!(
            "Original inputSchema truncated ({} chars > {}).",
            serialized.chars().count(),
            crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS
        )
    })
}

pub fn shared_mcp_manager() -> Arc<McpManager> {
    static MANAGER: OnceLock<Arc<McpManager>> = OnceLock::new();
    Arc::clone(MANAGER.get_or_init(|| Arc::new(McpManager::new())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_node_and_npx_cli() {
        let node = find_node_exe().expect("node.exe should be discoverable");
        assert!(file_exists(&node), "{node:?}");
        let cli = find_npm_js_cli("npx-cli.js").expect("npx-cli.js next to node");
        assert!(file_exists(&cli), "{cli:?}");
    }

    #[test]
    fn builds_npx_through_node_cli() {
        let config = McpServerConfig {
            id: "test".into(),
            command: "npx".into(),
            args: vec!["--version".into()],
            enabled: true,
            ..Default::default()
        };
        let (mut cmd, summary) = build_mcp_command(&config).expect("build npx command");
        assert!(
            summary.to_ascii_lowercase().contains("npx-cli.js")
                || summary.to_ascii_lowercase().contains("npx.cmd"),
            "unexpected launcher: {summary}"
        );
        let output = cmd.output().expect("spawn npx --version");
        assert!(
            output.status.success(),
            "npx --version failed: status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
