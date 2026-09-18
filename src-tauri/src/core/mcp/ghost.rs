//! Download and register NORTHTEKDevs Ghost as an Anya MCP server.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::core::tools::registry::ToolRegistry;
use crate::models::settings::McpServerConfig;
use crate::services::settings_store::{get_settings, set_settings};

const GHOST_RELEASE_TAG: &str = "v0.23.4";
const GHOST_ZIP_URL: &str =
    "https://github.com/NORTHTEKDevs/ghost/releases/download/v0.23.4/ghost-windows-x64.zip";
pub const GHOST_SERVER_ID: &str = "ghost";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GhostMcpInstallResult {
    pub server: McpServerConfig,
    pub exe_path: String,
    pub downloaded: bool,
    pub tool_count: usize,
}

/// Ensure `ghost-mcp.exe` is on disk, registered in settings, and connected.
pub fn ensure_ghost_mcp(
    app: &AppHandle,
    registry: &ToolRegistry,
) -> Result<GhostMcpInstallResult, String> {
    #[cfg(not(windows))]
    {
        let _ = (app, registry);
        return Err("Ghost MCP install is currently Windows-only".into());
    }
    #[cfg(windows)]
    {
        ensure_ghost_mcp_windows(app, registry)
    }
}

#[cfg(windows)]
fn ensure_ghost_mcp_windows(
    app: &AppHandle,
    registry: &ToolRegistry,
) -> Result<GhostMcpInstallResult, String> {
    let dir = ghost_install_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create ghost dir: {e}"))?;
    let exe = dir.join("ghost-mcp.exe");
    let downloaded = if exe.is_file() {
        false
    } else {
        download_and_extract(&dir)?;
        if !exe.is_file() {
            return Err(format!(
                "ghost-mcp.exe missing after extract in {}",
                dir.display()
            ));
        }
        true
    };

    let exe_path = exe
        .canonicalize()
        .unwrap_or(exe.clone())
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();

    let server = McpServerConfig {
        id: GHOST_SERVER_ID.into(),
        title: Some("Ghost Desktop".into()),
        description: Some(format!(
            "Verified Windows desktop control via UI Automation ({GHOST_RELEASE_TAG}). Prefer ghost_see / ghost_act over pixel clicks."
        )),
        command: exe_path.clone(),
        args: Vec::new(),
        env: vec![
            // Keep shell off for the first trial — GUI verbs only.
            ("GHOST_SHELL".into(), "off".into()),
        ],
        enabled: true,
        icon_url: None,
        qualified_name: Some("io.github.NORTHTEKDevs/ghost".into()),
        registry_id: None,
        homepage: Some("https://github.com/NORTHTEKDevs/ghost".into()),
        source: Some("curated".into()),
    };

    let mut settings = get_settings(app)?;
    if let Some(existing) = settings
        .mcp_servers
        .iter_mut()
        .find(|item| item.id == GHOST_SERVER_ID)
    {
        *existing = server.clone();
    } else {
        settings.mcp_servers.push(server.clone());
    }
    set_settings(app, settings)?;

    let tool_count = super::shared_mcp_manager()
        .reconnect_by_id(GHOST_SERVER_ID, registry)
        .map_err(|e| e.to_string())?;

    Ok(GhostMcpInstallResult {
        server,
        exe_path,
        downloaded,
        tool_count,
    })
}

#[cfg(windows)]
fn ghost_install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|p| p.join("tools").join("ghost"))
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn download_and_extract(dir: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("AnyaGhostInstall/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let response = client
        .get(GHOST_ZIP_URL)
        .send()
        .map_err(|e| format!("download Ghost: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "download Ghost failed: HTTP {}",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .map_err(|e| format!("read Ghost zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_ref()))
        .map_err(|e| format!("open Ghost zip: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("zip entry: {e}"))?;
        let name = file
            .enclosed_name()
            .ok_or_else(|| "unsafe zip path".to_string())?
            .to_path_buf();
        let out = dir.join(&name);
        if file.is_dir() {
            fs::create_dir_all(&out).map_err(|e| format!("mkdir: {e}"))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let mut dest =
            fs::File::create(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
        std::io::copy(&mut file, &mut dest)
            .map_err(|e| format!("extract {}: {e}", out.display()))?;
    }
    Ok(())
}
