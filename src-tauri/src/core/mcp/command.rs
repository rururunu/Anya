use std::io::BufReader;
use std::io::BufRead;
use std::thread;

use crate::core::tools::error::ToolError;
use crate::models::settings::McpServerConfig;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::remote_auth;
use super::runtime::{
    file_exists, find_node_exe, find_npm_js_cli, look_for_command, search_dirs,
};

fn resolve_mcp_program(command: &str) -> PathBuf {
    let as_path = PathBuf::from(command);
    if as_path.components().count() > 1 || file_exists(&as_path) {
        #[cfg(windows)]
        {
            return super::runtime::prefer_win32_executable(as_path);
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
    if let Some(path) = super::runtime::enriched_path_value() {
        cmd.env("PATH", &path);
        #[cfg(windows)]
        cmd.env("Path", &path);
    }
    for (k, v) in &config.env {
        cmd.env(k, v);
    }
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

pub(super) fn mcp_remote_uses_smithery_key_auth(config: &McpServerConfig) -> bool {
    remote_auth::mcp_remote_server_url(config)
        .map(|url| {
            let lower = url.to_ascii_lowercase();
            lower.contains("api_key=") || remote_auth::is_smithery_connect_proxy_url(&url)
        })
        .unwrap_or(false)
}

/// Clone of a server config with mcp-remote package args pinned for this spawn.
pub(super) fn spawn_config(config: &McpServerConfig) -> McpServerConfig {
    let mut next = config.clone();
    let _ = remote_auth::pin_mcp_remote_args(&mut next.args);
    let _ = remote_auth::inject_smithery_api_key_args(&mut next.args);
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
    if lower.contains("/authorize") && !lower.contains("client_id=") {
        return false;
    }
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

pub(super) fn forward_mcp_stderr(stderr: std::process::ChildStderr, server_id: String, open_oauth_urls: bool) {
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

pub(super) fn kill_process_tree(pid: u32) {
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
pub(crate) fn build_mcp_command(config: &McpServerConfig) -> Result<(Command, String), ToolError> {
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
