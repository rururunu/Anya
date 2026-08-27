use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::tools::agent::run_subagent_sync;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::skills_dir;
use crate::core::tools::registry::ToolRegistry;

mod docx_assets;

fn enabled_builtin_skills() -> &'static Mutex<HashSet<String>> {
    static ENABLED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ENABLED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Apply Settings → Skills opt-in list for built-in skills (default: none).
pub fn configure_enabled_builtin_skills(names: &[String]) {
    let mut set = HashSet::new();
    for name in names {
        if let Some(canon) = canonical_builtin_name(name) {
            set.insert(canon.to_string());
        } else {
            let cleaned = name.trim();
            if !cleaned.is_empty() {
                set.insert(cleaned.to_string());
            }
        }
    }
    if let Ok(mut lock) = enabled_builtin_skills().lock() {
        *lock = set;
    }
}

/// Canonical built-in skill id, or `None` if `name` is not a known built-in / alias.
pub fn canonical_builtin_name(name: &str) -> Option<&'static str> {
    match name.trim() {
        "explore" | "explore_codebase" => Some("explore"),
        "research" | "research_topic" => Some("research"),
        "review" | "review_code" => Some("review"),
        "security_review" | "review_security" => Some("security_review"),
        "generate_word" | "generate_docx" | "word" => Some("generate_word"),
        "docx" | "docx_skill" | "word_docx" => Some("docx"),
        "pandoc" | "convert_document" | "md2docx" => Some("pandoc"),
        _ => None,
    }
}

/// Whether a skill name may be used by the agent.
/// User-installed skills are always allowed; built-ins require Settings opt-in.
pub fn is_skill_enabled(name: &str) -> bool {
    match canonical_builtin_name(name) {
        Some(canon) => enabled_builtin_skills()
            .lock()
            .map(|set| set.contains(canon))
            .unwrap_or(false),
        None => true,
    }
}

pub fn require_skill_enabled(name: &str) -> Result<(), ToolError> {
    if is_skill_enabled(name) {
        return Ok(());
    }
    let label = canonical_builtin_name(name).unwrap_or(name);
    Err(ToolError::new(format!(
        "built-in skill `{label}` is disabled; enable it in Settings → Skills → Built-in"
    )))
}

pub(crate) fn materialize_docx_for_workspace(workspace_root: &Path) -> Result<PathBuf, ToolError> {
    docx_assets::materialize_docx_skill(workspace_root)
}

/// Build the docx skill prompt (playbook + materialized OOXML helper scripts).
pub(crate) fn build_docx_prompt(task: &str, workspace_root: &Path) -> Result<String, ToolError> {
    let skill_dir = materialize_docx_for_workspace(workspace_root)?;
    let scripts = skill_dir.join("scripts");
    let body = resolve_skill_body("docx")?;
    Ok(format!(
        "{body}\n\n## Runtime assets\n\
         Docx helper scripts materialized to:\n\
         `{dir}`\n\n\
         Set `SCRIPTS` = `{scripts}` in all shell examples (replace `{{SCRIPTS}}` placeholders).\n\
         Example: `python \"{scripts}/merge_runs.py\" file.docx`\n\n\
         ## Task\n{task}",
        dir = skill_dir.display(),
        scripts = scripts.display(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    /// `"builtin"` or `"user"`.
    pub source: String,
    pub title: String,
    pub description: String,
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    /// Origin of the install: `smithery` | `file` | `builtin`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

const EXPLORE_SKILL: &str = include_str!("../../../../prompts/skills/explore.md");
const RESEARCH_SKILL: &str = include_str!("../../../../prompts/skills/research.md");
const REVIEW_SKILL: &str = include_str!("../../../../prompts/skills/review.md");
const SECURITY_REVIEW_SKILL: &str = include_str!("../../../../prompts/skills/security_review.md");
const GENERATE_WORD_SKILL: &str = include_str!("../../../../prompts/skills/generate_word.md");
const DOCX_SKILL: &str = include_str!("../../../../prompts/skills/docx.md");
const PANDOC_SKILL: &str = include_str!("../../../../prompts/skills/pandoc.md");

pub fn register_all(registry: &mut ToolRegistry) {
    registry.register(Arc::new(LoadSkillTool));
    registry.register(Arc::new(RunSkillTool));
    registry.register(Arc::new(ListSkillsTool));
    registry.register(Arc::new(InstallSkillTool));
    registry.register(Arc::new(UninstallSkillTool));
    registry.register(Arc::new(ExploreCodebaseTool));
    registry.register(Arc::new(ResearchTopicTool));
    registry.register(Arc::new(ReviewCodeTool));
    registry.register(Arc::new(ReviewSecurityTool));
    registry.register(Arc::new(GenerateWordTool));
    registry.register(Arc::new(DocxSkillTool));
    registry.register(Arc::new(PandocSkillTool));
}

pub fn is_skill_tool(name: &str) -> bool {
    matches!(
        name,
        "load_skill"
            | "run_skill"
            | "list_skills"
            | "install_skill"
            | "uninstall_skill"
            | "explore_codebase"
            | "research_topic"
            | "review_code"
            | "review_security"
            | "generate_word"
            | "docx"
            | "pandoc"
    )
}

fn builtin_skill_body(name: &str) -> Option<&'static str> {
    match name {
        "explore" | "explore_codebase" => Some(EXPLORE_SKILL),
        "research" | "research_topic" => Some(RESEARCH_SKILL),
        "review" | "review_code" => Some(REVIEW_SKILL),
        "security_review" | "review_security" => Some(SECURITY_REVIEW_SKILL),
        "generate_word" | "generate_docx" | "word" => Some(GENERATE_WORD_SKILL),
        "docx" | "docx_skill" | "word_docx" => Some(DOCX_SKILL),
        "pandoc" | "convert_document" | "md2docx" => Some(PANDOC_SKILL),
        _ => None,
    }
}

fn sanitize_skill_name(name: &str) -> Result<String, ToolError> {
    let cleaned = name
        .trim()
        .trim_matches(|c| c == '/' || c == '\\')
        .replace('\\', "/");
    if cleaned.is_empty()
        || cleaned.contains("..")
        || cleaned.contains('/')
        || cleaned
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
    {
        return Err(ToolError::new(
            "skill name must be a simple identifier (letters, digits, _ - .)",
        ));
    }
    Ok(cleaned)
}

fn user_skill_dir(name: &str) -> Result<PathBuf, ToolError> {
    let name = sanitize_skill_name(name)?;
    Ok(skills_dir().join(name))
}

fn read_user_skill(name: &str) -> Result<Option<String>, ToolError> {
    let dir = user_skill_dir(name)?;
    let candidates = [
        dir.join("SKILL.md"),
        dir.join("skill.md"),
        dir.join("README.md"),
        dir.with_extension("md"),
    ];
    for path in candidates {
        if path.is_file() {
            return Ok(Some(fs::read_to_string(path)?));
        }
    }
    if dir.is_file() {
        return Ok(Some(fs::read_to_string(dir)?));
    }
    Ok(None)
}

pub(crate) fn resolve_skill_body(name: &str) -> Result<String, ToolError> {
    if let Some(body) = builtin_skill_body(name) {
        return Ok(body.to_string());
    }
    if let Some(body) = read_user_skill(name)? {
        return Ok(body);
    }
    Err(ToolError::new(format!("unknown skill: {name}")))
}

fn list_builtin_names() -> Vec<&'static str> {
    vec![
        "explore",
        "research",
        "review",
        "security_review",
        "generate_word",
        "docx",
        "pandoc",
    ]
}

/// Parse a short title + first paragraph from skill markdown (or YAML frontmatter).
fn parse_skill_blurb(body: &str) -> (String, String) {
    let mut rest = body;
    let mut fm_title = String::new();
    let mut fm_desc = String::new();
    if let Some(after) = body.strip_prefix("---") {
        if let Some(end) = after.find("\n---") {
            let front = &after[..end];
            for line in front.lines() {
                let line = line.trim();
                if let Some(v) = line.strip_prefix("name:") {
                    fm_title = v.trim().trim_matches('"').trim_matches('\'').to_string();
                } else if let Some(v) = line.strip_prefix("description:") {
                    fm_desc = v.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
            rest = after[end + 4..].trim_start();
        }
    }

    let mut title = fm_title;
    let mut description = fm_desc;
    for line in rest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if title.is_empty() {
            if let Some(heading) = trimmed.strip_prefix('#') {
                title = heading.trim_start_matches('#').trim().to_string();
                continue;
            }
            title = trimmed.to_string();
            continue;
        }
        if trimmed.starts_with('#') {
            break;
        }
        if description.is_empty() {
            description = trimmed.to_string();
        }
        break;
    }
    (title, description)
}

fn skill_info_from_body(name: &str, source: &str, body: &str, path: Option<String>) -> SkillInfo {
    let (mut title, description) = parse_skill_blurb(body);
    if title.is_empty() {
        title = name.to_string();
    }
    let mut info = SkillInfo {
        name: name.to_string(),
        source: source.to_string(),
        title,
        description,
        path: path.clone(),
        icon_url: None,
        qualified_name: None,
        registry_id: None,
        namespace: None,
        slug: None,
        homepage: None,
        git_url: None,
        verified: None,
        categories: None,
        origin: None,
    };
    if let Some(dir) = path.as_ref().map(PathBuf::from) {
        merge_skill_meta(&mut info, &dir);
    } else if source == "user" {
        if let Ok(dir) = user_skill_dir(name) {
            merge_skill_meta(&mut info, &dir);
        }
    }
    info
}

fn skill_meta_path(dir: &Path) -> PathBuf {
    dir.join("meta.json")
}

fn merge_skill_meta(info: &mut SkillInfo, dir: &Path) {
    let path = skill_meta_path(dir);
    let Ok(raw) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    if let Some(v) = value.get("displayName").and_then(|v| v.as_str()) {
        if !v.trim().is_empty() {
            info.title = v.trim().to_string();
        }
    }
    if let Some(v) = value.get("description").and_then(|v| v.as_str()) {
        if !v.trim().is_empty() {
            info.description = v.trim().to_string();
        }
    }
    info.icon_url = value
        .get("iconUrl")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or(info.icon_url.clone());
    info.qualified_name = value
        .get("qualifiedName")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.registry_id = value
        .get("registryId")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.namespace = value
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.slug = value
        .get("slug")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.homepage = value
        .get("homepage")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.git_url = value
        .get("gitUrl")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    info.verified = value.get("verified").and_then(|v| v.as_bool());
    info.categories = value.get("categories").and_then(|v| {
        v.as_array().map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
    });
    info.origin = value
        .get("source")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or(info.origin.clone());
}

pub fn write_skill_meta(name: &str, meta: &serde_json::Value) -> Result<(), ToolError> {
    let dir = user_skill_dir(name)?;
    if !dir.is_dir() {
        return Err(ToolError::new(format!("skill `{name}` is not installed")));
    }
    let path = skill_meta_path(&dir);
    let raw = serde_json::to_string_pretty(meta)
        .map_err(|error| ToolError::new(format!("invalid skill meta: {error}")))?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn list_skill_infos() -> Result<Vec<SkillInfo>, ToolError> {
    let mut out = Vec::new();
    for name in list_builtin_names() {
        let body = builtin_skill_body(name).unwrap_or("");
        out.push(skill_info_from_body(name, "builtin", body, None));
    }
    for name in list_user_skill_names()? {
        let body = read_user_skill(&name)?.unwrap_or_default();
        let path = user_skill_dir(&name)?.to_string_lossy().to_string();
        out.push(skill_info_from_body(&name, "user", &body, Some(path)));
    }
    Ok(out)
}

pub fn install_skill_at(source: &Path, name: Option<&str>) -> Result<SkillInfo, ToolError> {
    if !source.exists() {
        return Err(ToolError::new(format!(
            "skill source not found: {}",
            source.display()
        )));
    }

    let name = if let Some(explicit) = name.map(str::trim).filter(|s| !s.is_empty()) {
        sanitize_skill_name(explicit)?
    } else if source.is_file() {
        sanitize_skill_name(
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("skill"),
        )?
    } else {
        sanitize_skill_name(
            source
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("skill"),
        )?
    };

    if list_builtin_names().iter().any(|b| *b == name) {
        return Err(ToolError::new(format!(
            "cannot manage built-in skill `{name}`"
        )));
    }

    let dest = skills_dir().join(&name);
    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(&dest)?;
        } else {
            fs::remove_file(&dest)?;
        }
    }
    fs::create_dir_all(skills_dir())?;

    if source.is_file() {
        fs::create_dir_all(&dest)?;
        fs::copy(source, dest.join("SKILL.md"))?;
    } else {
        copy_dir_recursive(source, &dest)?;
        if !dest.join("SKILL.md").is_file()
            && !dest.join("skill.md").is_file()
            && !dest.join("README.md").is_file()
        {
            let _ = fs::remove_dir_all(&dest);
            return Err(ToolError::new(
                "skill directory must contain SKILL.md, skill.md, or README.md",
            ));
        }
    }

    let body = read_user_skill(&name)?.unwrap_or_default();
    Ok(skill_info_from_body(
        &name,
        "user",
        &body,
        Some(dest.to_string_lossy().to_string()),
    ))
}

/// Install a skill from markdown content (e.g. downloaded from Smithery).
pub fn install_skill_from_markdown(
    name: &str,
    content: &str,
    meta: Option<&serde_json::Value>,
) -> Result<SkillInfo, ToolError> {
    let name = sanitize_skill_name(name)?;
    if list_builtin_names().iter().any(|b| *b == name) {
        return Err(ToolError::new(format!(
            "cannot manage built-in skill `{name}`"
        )));
    }
    let body = content.trim();
    if body.is_empty() {
        return Err(ToolError::new("skill markdown is empty"));
    }

    let dest = skills_dir().join(&name);
    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(&dest)?;
        } else {
            fs::remove_file(&dest)?;
        }
    }
    fs::create_dir_all(&dest)?;
    fs::write(dest.join("SKILL.md"), body)?;
    if let Some(meta) = meta {
        let raw = serde_json::to_string_pretty(meta)
            .map_err(|error| ToolError::new(format!("invalid skill meta: {error}")))?;
        fs::write(skill_meta_path(&dest), raw)?;
    }

    Ok(skill_info_from_body(
        &name,
        "user",
        body,
        Some(dest.to_string_lossy().to_string()),
    ))
}

pub fn uninstall_user_skill(name: &str) -> Result<(), ToolError> {
    let name = sanitize_skill_name(name)?;
    if list_builtin_names().iter().any(|b| *b == name) {
        return Err(ToolError::new(format!(
            "cannot uninstall built-in skill `{name}`"
        )));
    }
    let dest = skills_dir().join(&name);
    let md = skills_dir().join(format!("{name}.md"));
    if dest.is_dir() {
        fs::remove_dir_all(&dest)?;
        Ok(())
    } else if md.is_file() {
        fs::remove_file(&md)?;
        Ok(())
    } else {
        Err(ToolError::new(format!("skill not installed: {name}")))
    }
}

pub fn ensure_skills_directory() -> Result<PathBuf, ToolError> {
    let dir = skills_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn list_user_skill_names() -> Result<Vec<String>, ToolError> {
    let root = skills_dir();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if path.join("SKILL.md").is_file()
                || path.join("skill.md").is_file()
                || path.join("README.md").is_file()
            {
                names.push(file_name);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            names.push(
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(file_name),
            );
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ToolError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

struct LoadSkillTool;

impl Tool for LoadSkillTool {
    fn name(&self) -> &str {
        "load_skill"
    }
    fn description(&self) -> &str {
        "Load a skill playbook body without executing it. Resolves built-in skills and user-installed skills under the skills directory."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let name = args["name"].as_str().unwrap_or("");
        require_skill_enabled(name)?;
        resolve_skill_body(name)
    }
}

struct RunSkillTool;

impl Tool for RunSkillTool {
    fn name(&self) -> &str {
        "run_skill"
    }
    fn description(&self) -> &str {
        "Run a skill by injecting its playbook and executing as subagent. With read_only=true the subagent is restricted to read-only tools (research, exploration, review)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "task": { "type": "string" },
                "read_only": { "type": "boolean", "default": false, "description": "Restrict the skill subagent to read-only tools" }
            },
            "required": ["name", "task"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let name = args["name"].as_str().unwrap_or("");
        require_skill_enabled(name)?;
        let task = args["task"].as_str().unwrap_or("");
        let prompt = if matches!(name, "docx" | "docx_skill" | "word_docx") {
            build_docx_prompt(task, &ctx.workspace_root)?
        } else {
            let body = resolve_skill_body(name)?;
            format!("{body}\n\n## Task\n{task}")
        };
        run_subagent_sync(ctx, &prompt, args["read_only"].as_bool().unwrap_or(false))
    }
}

struct ListSkillsTool;

impl Tool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }
    fn description(&self) -> &str {
        "List built-in and user-installed skills."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let infos = list_skill_infos()?
            .into_iter()
            .filter(|info| info.source != "builtin" || is_skill_enabled(&info.name))
            .collect::<Vec<_>>();
        if infos.is_empty() {
            return Ok("(no skills)".into());
        }
        let lines: Vec<String> = infos
            .into_iter()
            .map(|info| {
                let blurb = if info.description.is_empty() {
                    String::new()
                } else {
                    format!("\t{}", info.description)
                };
                format!("{}\t{}{}", info.source, info.name, blurb)
            })
            .collect();
        Ok(lines.join("\n"))
    }
}

pub struct InstallSkillTool;

impl Tool for InstallSkillTool {
    fn name(&self) -> &str {
        "install_skill"
    }
    fn description(&self) -> &str {
        "Install a skill package into the user skills directory. `path` may be a directory containing SKILL.md or a single .md file. Optional `name` overrides the destination folder name."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Source skill file or directory" },
                "name": { "type": "string", "description": "Optional install name" }
            },
            "required": ["path"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let raw_path = args["path"].as_str().unwrap_or("").trim();
        if raw_path.is_empty() {
            return Err(ToolError::new("path is required"));
        }
        let source = {
            let candidate = PathBuf::from(raw_path);
            if candidate.is_absolute() {
                candidate
            } else {
                ctx.workspace_root.join(candidate)
            }
        };
        let name = args["name"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let info = install_skill_at(&source, name)?;
        Ok(format!(
            "installed skill `{}` -> {}",
            info.name,
            info.path.unwrap_or_default()
        ))
    }
}

struct UninstallSkillTool;

impl Tool for UninstallSkillTool {
    fn name(&self) -> &str {
        "uninstall_skill"
    }
    fn description(&self) -> &str {
        "Remove a user-installed skill by name. Built-in skills cannot be removed."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let name = args["name"].as_str().unwrap_or("");
        uninstall_user_skill(name)?;
        Ok(format!("uninstalled skill `{name}`"))
    }
}

macro_rules! shortcut_skill {
    ($struct:ident, $tool_name:literal, $skill:literal, $desc:literal) => {
        struct $struct;
        impl Tool for $struct {
            fn name(&self) -> &str {
                $tool_name
            }
            fn description(&self) -> &str {
                $desc
            }
            fn parameters_schema(&self) -> Value {
                json!({
                    "type": "object",
                    "properties": { "task": { "type": "string" } },
                    "required": ["task"]
                })
            }
            fn read_only(&self) -> bool {
                true
            }
            fn available(&self) -> bool {
                is_skill_enabled($skill)
            }
            fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
                require_skill_enabled($skill)?;
                let task = args["task"].as_str().unwrap_or("");
                let body = resolve_skill_body($skill)?;
                let prompt = format!("{body}\n\n## Task\n{task}");
                run_subagent_sync(ctx, &prompt, true)
            }
        }
    };
}

shortcut_skill!(
    ExploreCodebaseTool,
    "explore_codebase",
    "explore",
    "Explore the codebase with the explore skill."
);
shortcut_skill!(
    ResearchTopicTool,
    "research_topic",
    "research",
    "Research a topic with the research skill."
);
shortcut_skill!(
    ReviewCodeTool,
    "review_code",
    "review",
    "Review code with the review skill."
);
shortcut_skill!(
    ReviewSecurityTool,
    "review_security",
    "security_review",
    "Security review with the security_review skill."
);

/// Write-capable shortcut: Word generation needs `write_file` / `run_shell`.
struct GenerateWordTool;

impl Tool for GenerateWordTool {
    fn name(&self) -> &str {
        "generate_word"
    }
    fn description(&self) -> &str {
        "Generate a .docx Word document with the generate_word skill (python-docx)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "task": { "type": "string" } },
            "required": ["task"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn available(&self) -> bool {
        is_skill_enabled("generate_word")
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        require_skill_enabled("generate_word")?;
        let task = args["task"].as_str().unwrap_or("");
        let body = resolve_skill_body("generate_word")?;
        let prompt = format!("{body}\n\n## Task\n{task}");
        run_subagent_sync(ctx, &prompt, false)
    }
}

/// Anthropic-style docx skill: docx-js creation + OOXML edit/validate helpers.
struct DocxSkillTool;

impl Tool for DocxSkillTool {
    fn name(&self) -> &str {
        "docx"
    }
    fn description(&self) -> &str {
        "Create or edit Word .docx with the docx skill (docx-js, OOXML unzip/edit, tracked changes, comments, validate). Materializes helper scripts to .anya/docx."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Docx task: create, edit, redline, comment, extract, validate"
                }
            },
            "required": ["task"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn available(&self) -> bool {
        is_skill_enabled("docx")
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        require_skill_enabled("docx")?;
        let task = args["task"].as_str().unwrap_or("");
        let prompt = build_docx_prompt(task, &ctx.workspace_root)?;
        run_subagent_sync(ctx, &prompt, false)
    }
}

/// Pandoc format conversion (md↔docx/pdf/html).
struct PandocSkillTool;

impl Tool for PandocSkillTool {
    fn name(&self) -> &str {
        "pandoc"
    }
    fn description(&self) -> &str {
        "Convert documents with pandoc (Markdown, DOCX, PDF, HTML). Use for md→docx/pdf and docx→md extraction. For OOXML surgery use docx."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Conversion task: input/output paths, formats, reference-doc, toc"
                }
            },
            "required": ["task"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn available(&self) -> bool {
        is_skill_enabled("pandoc")
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        require_skill_enabled("pandoc")?;
        let task = args["task"].as_str().unwrap_or("");
        let body = resolve_skill_body("pandoc")?;
        let prompt = format!("{body}\n\n## Task\n{task}");
        run_subagent_sync(ctx, &prompt, false)
    }
}
