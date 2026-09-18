/** Official plugins shipped with Anya and copied into the user plugins dir. */
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Ids rewritten on every launch; importing over them would be wiped.
pub const OFFICIAL_PLUGIN_IDS: &[&str] = &["terminal", "computer-use", "opencli"];

use super::grant::{clear_grant, has_grant, save_grant, PluginGrant};
use super::manifest::load_manifest;
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;

const TERMINAL_MANIFEST: &str = include_str!("../../../plugins/terminal/plugin.json");
const TERMINAL_ACTIVATE: &str = include_str!("../../../plugins/terminal/ui/src/activate.js");
const TERMINAL_XTERM_JS: &str = include_str!("../../../plugins/terminal/ui/src/vendor/xterm.js");
const TERMINAL_XTERM_CSS: &str = include_str!("../../../plugins/terminal/ui/src/vendor/xterm.css");
const TERMINAL_HOST: &str = include_str!("../../../plugins/terminal/host/main.ts");
const TERMINAL_HTML: &str = include_str!("../../../plugins/terminal/ui/index.html");
const TERMINAL_ICON: &str = include_str!("../../../plugins/terminal/ui/icon.svg");
const TERMINAL_ABOUT: &str = include_str!("../../../plugins/terminal/ui/about.md");

const CU_MANIFEST: &str = include_str!("../../../plugins/computer-use/plugin.json");
const CU_HOST: &str = include_str!("../../../plugins/computer-use/host/main.ts");
const CU_ICON: &str = include_str!("../../../plugins/computer-use/ui/icon.svg");
const CU_ABOUT: &str = include_str!("../../../plugins/computer-use/ui/about.md");
const CU_SKILL_WINDOWS: &str = include_str!("../../../plugins/computer-use/skills/windows.md");

const OC_MANIFEST: &str = include_str!("../../../plugins/opencli/plugin.json");
const OC_HOST: &str = include_str!("../../../plugins/opencli/host/main.ts");
const OC_ICON: &str = include_str!("../../../plugins/opencli/ui/icon.svg");
const OC_ABOUT: &str = include_str!("../../../plugins/opencli/ui/about.md");
const OC_SKILL_BROWSER: &str = include_str!("../../../plugins/opencli/skills/browser.md");
const OC_SKILL_USAGE: &str = include_str!("../../../plugins/opencli/skills/usage.md");

pub fn ensure_bundled_plugins() -> Result<(), ToolError> {
    retire_unshipped_plugin("anya-cli");
    retire_unshipped_plugin("system-terminal");
    retire_unshipped_plugin("pet-gallery");
    write_official_terminal(plugins_dir().join("terminal"))?;
    write_official_computer_use(plugins_dir().join("computer-use"))?;
    write_official_opencli(plugins_dir().join("opencli"))?;
    ensure_official_grant("terminal")?;
    ensure_official_grant_disabled("computer-use")?;
    ensure_official_grant_disabled("opencli")
}

fn write_official_terminal(dir: PathBuf) -> Result<(), ToolError> {
    fs::create_dir_all(dir.join("ui").join("src").join("vendor"))
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::create_dir_all(dir.join("host")).map_err(|e| ToolError::new(e.to_string()))?;
    fs::create_dir_all(dir.join("ui").join(".anya")).map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("plugin.json"), TERMINAL_MANIFEST)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join("src").join("activate.js"),
        TERMINAL_ACTIVATE,
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join(".anya").join("activate.js"),
        TERMINAL_ACTIVATE,
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join("src").join("vendor").join("xterm.js"),
        TERMINAL_XTERM_JS,
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join("src").join("vendor").join("xterm.css"),
        TERMINAL_XTERM_CSS,
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("ui").join("index.html"), TERMINAL_HTML)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("ui").join("icon.svg"), TERMINAL_ICON)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("ui").join("about.md"), TERMINAL_ABOUT)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("host").join("main.ts"), TERMINAL_HOST)
        .map_err(|e| ToolError::new(e.to_string()))?;
    Ok(())
}

fn write_official_computer_use(dir: PathBuf) -> Result<(), ToolError> {
    write_plugin_file(&dir, "plugin.json", CU_MANIFEST)?;
    write_plugin_file(&dir, "ui/icon.svg", CU_ICON)?;
    write_plugin_file(&dir, "ui/about.md", CU_ABOUT)?;
    write_plugin_file(&dir, "host/main.ts", CU_HOST)?;
    write_plugin_file(&dir, "skills/windows.md", CU_SKILL_WINDOWS)?;
    let _ = fs::remove_file(dir.join("ui").join("src").join("activate.js"));
    let _ = fs::remove_file(dir.join("ui").join(".anya").join("activate.js"));
    let _ = fs::remove_file(dir.join("ui").join("index.html"));
    Ok(())
}

fn write_official_opencli(dir: PathBuf) -> Result<(), ToolError> {
    write_plugin_file(&dir, "plugin.json", OC_MANIFEST)?;
    write_plugin_file(&dir, "ui/icon.svg", OC_ICON)?;
    write_plugin_file(&dir, "ui/about.md", OC_ABOUT)?;
    write_plugin_file(&dir, "host/main.ts", OC_HOST)?;
    write_plugin_file(&dir, "skills/browser.md", OC_SKILL_BROWSER)?;
    write_plugin_file(&dir, "skills/usage.md", OC_SKILL_USAGE)?;
    let _ = fs::remove_file(dir.join("ui").join("src").join("activate.js"));
    let _ = fs::remove_file(dir.join("ui").join(".anya").join("activate.js"));
    let _ = fs::remove_file(dir.join("ui").join("index.html"));
    Ok(())
}

/// Drop a previously bundled plugin so it does not linger as a user install.
fn retire_unshipped_plugin(id: &str) {
    let _ = fs::remove_dir_all(plugins_dir().join(id));
    let _ = clear_grant(id);
}

fn write_plugin_file(dir: &Path, rel: &str, contents: &str) -> Result<(), ToolError> {
    write_plugin_bytes(dir, rel, contents.as_bytes())
}

fn write_plugin_bytes(dir: &Path, rel: &str, contents: &[u8]) -> Result<(), ToolError> {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
    }
    fs::write(path, contents).map_err(|e| ToolError::new(e.to_string()))
}

/// First launch: enable the official plugin. A later Disable is remembered.
fn ensure_official_grant(id: &str) -> Result<(), ToolError> {
    if has_grant(id) {
        return Ok(());
    }
    let manifest = load_manifest(id)?;
    let granted_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    save_grant(
        id,
        PluginGrant {
            enabled: true,
            permissions: manifest.permissions,
            granted_at,
        },
    )
}

/// Listed official plugin that stays off until the user Enable-grants it.
fn ensure_official_grant_disabled(id: &str) -> Result<(), ToolError> {
    if has_grant(id) {
        return Ok(());
    }
    let manifest = load_manifest(id)?;
    let granted_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    save_grant(
        id,
        PluginGrant {
            enabled: false,
            permissions: manifest.permissions,
            granted_at,
        },
    )
}
