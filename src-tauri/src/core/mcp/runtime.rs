use std::env;
use std::path::{Path, PathBuf};

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

pub(super) fn enriched_path_value() -> Option<std::ffi::OsString> {
    let mut dirs = current_path_dirs();
    for dir in known_node_bin_dirs() {
        if dir.is_dir() && !dirs.iter().any(|d| d == &dir) {
            dirs.push(dir);
        }
    }
    env::join_paths(dirs).ok()
}

pub(super) fn file_exists(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
fn windows_shim_candidates(name: &str) -> Vec<String> {
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

pub(super) fn look_for_command(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
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
pub(super) fn prefer_win32_executable(path: PathBuf) -> PathBuf {
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

pub(super) fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = current_path_dirs();
    dirs.extend(known_node_bin_dirs());
    dirs
}

/// Locates a `node` executable on the current PATH.
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

/// Locates a `uvx` executable on the current PATH.
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

/// Reports which MCP runtime launchers are available on this machine.
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
