/** Official plugins shipped with Anya and copied into the user plugins dir. */
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Ids rewritten on every launch; importing over them would be wiped.
pub const OFFICIAL_PLUGIN_IDS: &[&str] = &["opencli"];

pub const BUILTIN_PLUGIN_IDS: &[&str] = &["terminal", "computer-use"];

use super::grant::{clear_grant, has_grant, save_grant, PluginGrant};
use super::manifest::load_manifest;
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;



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
    if let Err(error) = retire_ai_gallery() {
        tracing::warn!("could not retire AI gallery: {error}");
    }
    write_official_opencli(plugins_dir().join("opencli"))?;
    ensure_official_grant_disabled("opencli")
}

fn retire_ai_gallery() -> Result<(), ToolError> {
    let id = "ai-gallery";
    let dir = plugins_dir().join(id);
    // Hide the old plugin from discovery while preserving its archived images.
    match fs::rename(dir.join("plugin.json"), dir.join("plugin.json.retired")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(ToolError::new(error.to_string())),
    }
    if has_grant(id) {
        clear_grant(id)?;
    }
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
