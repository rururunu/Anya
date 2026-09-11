//! plugin.json schema and on-disk layout.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::capability::{validate_capability_decl, CapabilityDecl};
use super::grant::{get_grant, PluginGrant};
use super::package::is_official_plugin_id;
use super::permissions::is_known_permission;
use super::scaffold::{self, write_scaffold};
use super::MAX_PLUGIN_FILE_BYTES;
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;

/// Contract versions Anya's runtime currently understands. A manifest declaring an
/// unsupported version is not rejected outright (that would brick old plugins the
/// moment we ship a new version) -- it is loaded but flagged `apiSupported: false`
/// so the UI can warn instead of silently misbehaving.
pub const SUPPORTED_API_VERSIONS: &[&str] = &["1.0"];

pub fn is_supported_api_version(version: &str) -> bool {
    SUPPORTED_API_VERSIONS.contains(&version)
}

const ALLOWED_EXT: &[&str] = &[
    "html", "css", "js", "mjs", "ts", "json", "svg", "png", "jpg", "jpeg", "webp", "gif", "txt",
    "md", "woff2",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWindowSpec {
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 {
    960
}
fn default_height() -> u32 {
    720
}

impl Default for PluginWindowSpec {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUiSpec {
    #[serde(default = "default_ui_entry")]
    pub entry: String,
}

fn default_ui_entry() -> String {
    "ui/activate.js".into()
}

impl Default for PluginUiSpec {
    fn default() -> Self {
        Self {
            entry: default_ui_entry(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginHostSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginAgentContrib {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub prompt: String,
    /// Relative markdown files appended to `prompt` when `agent.prompt` is granted.
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributes {
    #[serde(default)]
    pub sidebar: bool,
    #[serde(default)]
    pub view: bool,
    #[serde(default)]
    pub composer: bool,
    #[serde(default)]
    pub window: bool,
    #[serde(default)]
    pub agent: PluginAgentContrib,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// Contract version this plugin was written against, e.g. "1.0". See
    /// `SUPPORTED_API_VERSIONS`. Defaults to the current version so existing
    /// plugins without the field are treated as up to date.
    #[serde(default = "default_api_version")]
    pub api_version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Relative path to an svg/png/jpg/webp/gif the plugin ships, shown as its
    /// icon in the plugins list and (via `anya-plugin://localhost/<id>/<icon>`)
    /// usable as a sidebar-tab/composer-accessory icon too. Falls back to a
    /// generic glyph when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Relative path to a `.md`/`.html` file the plugin ships, rendered as the
    /// "详情/About" tab on its home page (see `ctx.home` in the workbench SDK
    /// for the sibling "设置/Settings" tab). Falls back to `description` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default)]
    pub ui: PluginUiSpec,
    #[serde(default)]
    pub host: PluginHostSpec,
    /// `ui` | `service` | `agent`. Empty = infer from `contributes`.
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub contributes: PluginContributes,
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Structured capability declarations layered on top of `permissions`; see
    /// `capability.rs`. Optional and empty by default.
    #[serde(default)]
    pub capabilities: Vec<CapabilityDecl>,
    #[serde(default)]
    pub window: PluginWindowSpec,
}

fn default_version() -> String {
    "0.1.0".into()
}
fn default_api_version() -> String {
    "1.0".into()
}
fn default_entry() -> String {
    "ui/index.html".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub api_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    pub description: String,
    pub path: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub capabilities: Vec<CapabilityDecl>,
    pub granted: Vec<String>,
    pub has_ui: bool,
    pub has_host: bool,
    pub role: String,
    pub contributes: PluginContributes,
    #[serde(default)]
    pub official: bool,
}

pub fn contributes_chrome(contributes: &PluginContributes) -> bool {
    contributes.sidebar || contributes.view || contributes.composer || contributes.window
}

/// Workbench `activate.js` is required only when the plugin mounts chrome or declares `ui.workbench`.
pub fn needs_workbench_ui(manifest: &PluginManifest) -> bool {
    contributes_chrome(&manifest.contributes)
        || manifest
            .permissions
            .iter()
            .any(|perm| perm == "ui.workbench")
}

/// `ui` (chrome), `service` (host/OS capability), or `agent` (chat tools).
pub fn resolved_role(manifest: &PluginManifest) -> String {
    match manifest.role.trim() {
        "ui" | "service" | "agent" => manifest.role.trim().to_string(),
        _ => infer_role(&manifest.contributes).into(),
    }
}

fn infer_role(contributes: &PluginContributes) -> &'static str {
    let chrome = contributes_chrome(contributes);
    if contributes.agent.tools && !chrome {
        "agent"
    } else if chrome {
        "ui"
    } else {
        "service"
    }
}

pub fn sanitize_plugin_id(id: &str) -> Result<String, ToolError> {
    let cleaned = id.trim().to_ascii_lowercase();
    if cleaned.is_empty()
        || cleaned.len() > 64
        || !cleaned
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || cleaned.starts_with('-')
        || cleaned.contains("..")
    {
        return Err(ToolError::new(
            "plugin id must be kebab-case (letters, digits, - _), max 64 chars",
        ));
    }
    Ok(cleaned)
}

pub fn plugin_dir(id: &str) -> Result<PathBuf, ToolError> {
    let id = sanitize_plugin_id(id)?;
    Ok(plugins_dir().join(id))
}

pub fn load_manifest(id: &str) -> Result<PluginManifest, ToolError> {
    let dir = plugin_dir(id)?;
    let raw = fs::read_to_string(dir.join("plugin.json"))
        .map_err(|e| ToolError::new(format!("read plugin.json: {e}")))?;
    let mut manifest: PluginManifest = serde_json::from_str(&raw)
        .map_err(|e| ToolError::new(format!("invalid plugin.json: {e}")))?;
    manifest.id = sanitize_plugin_id(&manifest.id)?;
    if manifest.id != sanitize_plugin_id(id)? {
        return Err(ToolError::new("plugin.json id does not match folder name"));
    }
    validate_rel_path(&manifest.entry, "entry")?;
    validate_rel_path(&manifest.ui.entry, "ui.entry")?;
    if let Some(icon) = &manifest.icon {
        if !icon.trim().is_empty() {
            validate_rel_path(icon, "icon")?;
        }
    }
    if let Some(about) = &manifest.about {
        if !about.trim().is_empty() {
            validate_rel_path(about, "about")?;
        }
    }
    if let Some(host) = &manifest.host.entry {
        if !host.trim().is_empty() {
            validate_rel_path(host, "host.entry")?;
        }
    }
    for perm in &manifest.permissions {
        if !is_known_permission(perm) {
            return Err(ToolError::new(format!("unknown permission `{perm}`")));
        }
    }
    for decl in &manifest.capabilities {
        validate_capability_decl(decl)?;
    }
    Ok(manifest)
}

fn validate_rel_path(entry: &str, field: &str) -> Result<(), ToolError> {
    let normalized = entry.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains("..")
        || Path::new(&normalized).is_absolute()
    {
        return Err(ToolError::new(format!(
            "{field} must be a relative path inside the plugin"
        )));
    }
    Ok(())
}

pub fn list_plugins() -> Result<Vec<PluginSummary>, ToolError> {
    let root = plugins_dir();
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| ToolError::new(e.to_string()))? {
        let entry = entry.map_err(|e| ToolError::new(e.to_string()))?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(manifest) = load_manifest(&name) else {
            continue;
        };
        out.push(summary_from(
            &manifest,
            &entry.path(),
            &get_grant(&manifest.id),
        ));
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn summary_from(manifest: &PluginManifest, path: &Path, grant: &PluginGrant) -> PluginSummary {
    let icon = manifest
        .icon
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && path.join(s).is_file())
        .map(str::to_string);
    let about = manifest
        .about
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && path.join(s).is_file())
        .map(str::to_string);
    let host_entry = manifest
        .host
        .entry
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    PluginSummary {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        api_version: manifest.api_version.clone(),
        api_supported: is_supported_api_version(&manifest.api_version),
        icon,
        about,
        description: manifest.description.clone(),
        path: path.display().to_string(),
        enabled: grant.enabled,
        permissions: manifest.permissions.clone(),
        capabilities: manifest.capabilities.clone(),
        granted: grant.permissions.clone(),
        has_ui: needs_workbench_ui(manifest),
        has_host: host_entry.is_some(),
        role: resolved_role(manifest),
        contributes: manifest.contributes.clone(),
        official: is_official_plugin_id(&manifest.id),
    }
}

pub fn create_plugin(id: &str, name: &str, description: &str) -> Result<PluginManifest, ToolError> {
    let id = sanitize_plugin_id(id)?;
    if is_official_plugin_id(&id) {
        return Err(ToolError::new(format!(
            "cannot create over official plugin `{id}`"
        )));
    }
    let dir = plugin_dir(&id)?;
    if dir.join("plugin.json").is_file() {
        return Err(ToolError::new(format!(
            "plugin `{id}` already exists; use put_file / open / delete"
        )));
    }
    let name = name.trim();
    if name.is_empty() {
        return Err(ToolError::new("name is required"));
    }
    let manifest = scaffold::default_manifest(&id, name, description.trim());
    write_scaffold(&dir, &manifest)?;
    Ok(manifest)
}

pub fn delete_plugin(id: &str) -> Result<(), ToolError> {
    let id = sanitize_plugin_id(id)?;
    if is_official_plugin_id(&id) {
        return Err(ToolError::new(format!(
            "cannot uninstall official plugin `{id}`"
        )));
    }
    let dir = plugin_dir(&id)?;
    if !dir.is_dir() {
        return Err(ToolError::new(format!("plugin `{id}` not found")));
    }
    fs::remove_dir_all(&dir).map_err(|e| ToolError::new(e.to_string()))?;
    Ok(())
}

pub fn put_plugin_file(id: &str, relative: &str, contents: &str) -> Result<PathBuf, ToolError> {
    let dir = plugin_dir(id)?;
    if !dir.join("plugin.json").is_file() {
        return Err(ToolError::new(format!(
            "plugin `{id}` not found; call create first"
        )));
    }
    let rel = normalize_rel(relative)?;
    if contents.len() > MAX_PLUGIN_FILE_BYTES {
        return Err(ToolError::new(format!(
            "file too large ({} bytes; max {MAX_PLUGIN_FILE_BYTES})",
            contents.len()
        )));
    }
    let dest = dir.join(&rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
    }
    fs::write(&dest, contents).map_err(|e| ToolError::new(e.to_string()))?;
    Ok(dest)
}

pub fn read_plugin_file(id: &str, relative: &str) -> Result<String, ToolError> {
    let dir = plugin_dir(id)?;
    if !dir.join("plugin.json").is_file() {
        return Err(ToolError::new(format!(
            "plugin `{id}` not found; call list first"
        )));
    }
    let rel = normalize_rel(relative)?;
    let dest = dir.join(&rel);
    fs::read_to_string(&dest).map_err(|e| ToolError::new(format!("read {}: {e}", dest.display())))
}

const MAX_LISTED_PLUGIN_FILES: usize = 200;

/// Relative paths inside the plugin folder. Skips generated `ui/.anya` and `node_modules`.
pub fn list_plugin_files(id: &str) -> Result<Vec<String>, ToolError> {
    let dir = plugin_dir(id)?;
    if !dir.join("plugin.json").is_file() {
        return Err(ToolError::new(format!("plugin `{id}` not found")));
    }
    let mut out = Vec::new();
    collect_rel_files(&dir, &dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_rel_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), ToolError> {
    if out.len() >= MAX_LISTED_PLUGIN_FILES {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| ToolError::new(e.to_string()))? {
        if out.len() >= MAX_LISTED_PLUGIN_FILES {
            break;
        }
        let entry = entry.map_err(|e| ToolError::new(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".anya" || name_str == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_rel_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

pub(crate) fn normalize_rel(relative: &str) -> Result<String, ToolError> {
    let normalized = relative.trim().replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || normalized.contains("..") {
        return Err(ToolError::new(
            "path must be relative inside the plugin folder",
        ));
    }
    let ext = Path::new(&normalized)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let file_name = Path::new(&normalized)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name == "cargo.toml" || file_name == "cargo.lock" {
        return Err(ToolError::new(
            "plugins cannot include Cargo projects; JS/TS only",
        ));
    }
    if !ALLOWED_EXT.contains(&ext.as_str()) {
        return Err(ToolError::new(format!(
            "disallowed extension `.{ext}` (html/css/js/ts/json/images/fonts/md only)"
        )));
    }
    Ok(normalized)
}

pub fn tool_prefix(plugin_id: &str) -> String {
    format!("plugin_{plugin_id}__")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_escape() {
        assert!(sanitize_plugin_id("../x").is_err());
        assert!(sanitize_plugin_id("Photo Editor").is_err());
        assert!(sanitize_plugin_id("photo-editor").is_ok());
        assert!(normalize_rel("../secret.txt").is_err());
        assert!(normalize_rel("ui/app.exe").is_err());
        assert!(normalize_rel("ui/index.html").is_ok());
        assert!(normalize_rel("host/main.ts").is_ok());
        assert!(normalize_rel("Cargo.toml").is_err());
    }

    #[test]
    fn parses_extended_manifest() {
        let raw = r#"{
            "id": "demo",
            "name": "Demo",
            "ui": { "entry": "ui/activate.js" },
            "host": { "entry": "host/main.ts" },
            "contributes": { "sidebar": true, "agent": { "tools": true, "prompt": "hi", "skills": ["skills/windows.md"] } },
            "permissions": ["storage", "ui.workbench", "agent.tools"]
        }"#;
        let m: PluginManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.ui.entry, "ui/activate.js");
        assert_eq!(m.host.entry.as_deref(), Some("host/main.ts"));
        assert!(m.contributes.sidebar);
        assert!(m.contributes.agent.tools);
        assert_eq!(
            m.contributes.agent.skills,
            vec!["skills/windows.md".to_string()]
        );
        assert_eq!(resolved_role(&m), "ui");
        assert!(needs_workbench_ui(&m));
    }

    #[test]
    fn infers_agent_role_without_chrome() {
        let raw = r#"{
            "id": "desk",
            "name": "Desk",
            "role": "agent",
            "host": { "entry": "host/main.ts" },
            "contributes": { "agent": { "tools": true } },
            "permissions": ["agent.tools", "computer"]
        }"#;
        let m: PluginManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(resolved_role(&m), "agent");
        assert!(!needs_workbench_ui(&m));
        assert!(!contributes_chrome(&m.contributes));
    }
}
