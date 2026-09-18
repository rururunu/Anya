//! Persist / score / format learned playbooks.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::tools::error::ToolError;
use crate::core::tools::memory::user_data_dir;

const MAX_STEPS: usize = 40;
const MAX_CATALOG: usize = 12;
const MAX_LOOKUP: usize = 5;
const QUARANTINE_BELOW: f32 = 0.35;
const FAIL_PENALTY: f32 = 0.18;
const SUCCESS_BOOST: f32 = 0.08;

static DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookStatus {
    Active,
    Quarantined,
}

impl Default for PlaybookStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookStep {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precondition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: String,
    pub intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_last_ok: Option<String>,
    pub steps: Vec<PlaybookStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_assert: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub status: PlaybookStatus,
    #[serde(default)]
    pub success_count: u32,
    #[serde(default)]
    pub fail_count: u32,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

fn default_confidence() -> f32 {
    0.7
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
pub fn set_dir_override(path: Option<PathBuf>) {
    if let Ok(mut g) = DIR_OVERRIDE.lock() {
        *g = path;
    }
}

fn playbooks_dir() -> PathBuf {
    if let Ok(g) = DIR_OVERRIDE.lock() {
        if let Some(p) = g.as_ref() {
            return p.clone();
        }
    }
    user_data_dir().join("computer-use").join("playbooks")
}

fn ensure_dir(path: &Path) -> Result<(), ToolError> {
    fs::create_dir_all(path).map_err(|e| ToolError::new(format!("playbook dir: {e}")))
}

fn path_for(id: &str) -> PathBuf {
    playbooks_dir().join(format!("{id}.json"))
}

fn load_all() -> Result<Vec<Playbook>, ToolError> {
    let dir = playbooks_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| ToolError::new(format!("playbook list: {e}")))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(pb) = serde_json::from_str::<Playbook>(&raw) {
            out.push(pb);
        }
    }
    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    Ok(out)
}

fn save_one(pb: &Playbook) -> Result<(), ToolError> {
    let dir = playbooks_dir();
    ensure_dir(&dir)?;
    let path = path_for(&pb.id);
    let raw = serde_json::to_string_pretty(pb)
        .map_err(|e| ToolError::new(format!("playbook serialize: {e}")))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &raw).map_err(|e| ToolError::new(format!("playbook write: {e}")))?;
    fs::rename(&tmp, &path).map_err(|e| ToolError::new(format!("playbook rename: {e}")))
}

fn load_one(id: &str) -> Result<Playbook, ToolError> {
    let path = path_for(id);
    let raw = fs::read_to_string(&path)
        .map_err(|_| ToolError::new(format!("playbook not found: {id}")))?;
    serde_json::from_str(&raw).map_err(|e| ToolError::new(format!("playbook parse: {e}")))
}

pub(super) fn score(pb: &Playbook, query: &str, app: Option<&str>, exe: Option<&str>) -> f32 {
    let q = query.to_lowercase();
    let mut s = 0.0_f32;
    let intent = pb.intent.to_lowercase();
    if intent.contains(&q) || q.contains(&intent) {
        s += 3.0;
    }
    for token in q.split_whitespace().filter(|t| t.len() > 1) {
        if intent.contains(token) {
            s += 1.0;
        }
        if pb
            .app
            .as_ref()
            .map(|a| a.to_lowercase().contains(token))
            .unwrap_or(false)
        {
            s += 0.5;
        }
    }
    if let Some(app) = app {
        let app_l = app.to_lowercase();
        if pb
            .app
            .as_ref()
            .map(|a| a.to_lowercase().contains(&app_l) || app_l.contains(&a.to_lowercase()))
            .unwrap_or(false)
        {
            s += 2.0;
        }
    }
    if let Some(exe) = exe {
        let exe_l = exe.to_lowercase();
        if pb
            .exe
            .as_ref()
            .map(|e| e.to_lowercase().contains(&exe_l) || exe_l.contains(&e.to_lowercase()))
            .unwrap_or(false)
        {
            s += 2.5;
        }
    }
    s * pb.confidence.max(0.1)
}

pub(super) fn parse_steps(value: &Value) -> Result<Vec<PlaybookStep>, ToolError> {
    let Some(arr) = value.as_array() else {
        return Err(ToolError::new("steps must be an array"));
    };
    if arr.is_empty() {
        return Err(ToolError::new("steps must not be empty"));
    }
    if arr.len() > MAX_STEPS {
        return Err(ToolError::new(format!("steps max {MAX_STEPS}")));
    }
    let mut steps = Vec::with_capacity(arr.len());
    for item in arr {
        if let Ok(step) = serde_json::from_value::<PlaybookStep>(item.clone()) {
            if step.kind.trim().is_empty() {
                return Err(ToolError::new("each step needs kind"));
            }
            steps.push(step);
            continue;
        }
        let kind = item
            .get("kind")
            .or_else(|| item.get("op"))
            .and_then(|v| v.as_str())
            .unwrap_or("note")
            .to_string();
        steps.push(PlaybookStep {
            kind,
            action: str_field(item, "action"),
            name: str_field(item, "name"),
            role: str_field(item, "role"),
            keys: str_field(item, "keys").or_else(|| str_field(item, "key")),
            text: str_field(item, "text").or_else(|| str_field(item, "text_input")),
            window: str_field(item, "window"),
            note: str_field(item, "note"),
            precondition: str_field(item, "precondition"),
        });
    }
    Ok(steps)
}

fn str_field(item: &Value, key: &str) -> Option<String> {
    item.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn format_playbook(pb: &Playbook) -> String {
    let mut lines = vec![
        format!("# Playbook `{}`", pb.id),
        format!("intent: {}", pb.intent),
    ];
    if let Some(app) = &pb.app {
        lines.push(format!("app: {app}"));
    }
    if let Some(exe) = &pb.exe {
        lines.push(format!("exe: {exe}"));
    }
    if let Some(v) = &pb.version_last_ok {
        lines.push(format!("version_last_ok: {v}"));
    }
    lines.push(format!(
        "confidence: {:.2} status: {:?} ok/fail: {}/{}",
        pb.confidence, pb.status, pb.success_count, pb.fail_count
    ));
    if let Some(a) = &pb.success_assert {
        lines.push(format!("success_assert: {a}"));
    }
    lines.push("steps:".into());
    for (i, step) in pb.steps.iter().enumerate() {
        let mut parts = vec![format!("{}. {}", i + 1, step.kind)];
        if let Some(a) = &step.action {
            parts.push(format!("action={a}"));
        }
        if let Some(n) = &step.name {
            parts.push(format!("name={n}"));
        }
        if let Some(r) = &step.role {
            parts.push(format!("role={r}"));
        }
        if let Some(k) = &step.keys {
            parts.push(format!("keys={k}"));
        }
        if let Some(t) = &step.text {
            parts.push(format!("text={t}"));
        }
        if let Some(w) = &step.window {
            parts.push(format!("window={w}"));
        }
        if let Some(p) = &step.precondition {
            parts.push(format!("pre={p}"));
        }
        if let Some(n) = &step.note {
            parts.push(format!("note={n}"));
        }
        lines.push(parts.join(" "));
    }
    lines.push(
        "Run step-by-step with see/assert gates. If verified fails, playbook fail then explore and save a patch."
            .into(),
    );
    lines.join("\n")
}

pub fn prompt_catalog() -> Option<String> {
    let Ok(all) = load_all() else {
        return None;
    };
    let active: Vec<_> = all
        .into_iter()
        .filter(|p| matches!(p.status, PlaybookStatus::Active) && p.confidence >= QUARANTINE_BELOW)
        .take(MAX_CATALOG)
        .collect();
    if active.is_empty() {
        return None;
    }
    let mut body = String::from(
        "## Learned playbooks (local)\nUse `playbook` lookup before exploring unfamiliar apps. After a novel success, `playbook` save. On step failure, `playbook` fail.\n",
    );
    for pb in active {
        let app = pb.app.as_deref().or(pb.exe.as_deref()).unwrap_or("-");
        body.push_str(&format!(
            "- `{}` [{app}] {} (conf {:.2})\n",
            pb.id, pb.intent, pb.confidence
        ));
    }
    Some(body)
}

pub fn dispatch(args: &Value) -> Result<String, ToolError> {
    let op = args
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("list")
        .to_lowercase();
    match op.as_str() {
        "list" => {
            let all = load_all()?;
            if all.is_empty() {
                return Ok(
                    "No learned playbooks yet. After a successful novel workflow, call playbook op=save."
                        .into(),
                );
            }
            let lines: Vec<String> = all
                .iter()
                .take(40)
                .map(|p| {
                    format!(
                        "- {} | {:?} | conf={:.2} | {} | {}",
                        p.id,
                        p.status,
                        p.confidence,
                        p.app.as_deref().or(p.exe.as_deref()).unwrap_or("-"),
                        p.intent
                    )
                })
                .collect();
            Ok(format!("Playbooks ({}):\n{}", all.len(), lines.join("\n")))
        }
        "lookup" | "get" => {
            if let Some(id) = args.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                return Ok(format_playbook(&load_one(id)?));
            }
            let query = args
                .get("query")
                .or_else(|| args.get("intent"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if query.is_empty() {
                return Err(ToolError::new("lookup needs query/intent or id"));
            }
            let app = args.get("app").and_then(|v| v.as_str());
            let exe = args.get("exe").and_then(|v| v.as_str());
            let mut ranked: Vec<(f32, Playbook)> = load_all()?
                .into_iter()
                .filter(|p| !matches!(p.status, PlaybookStatus::Quarantined))
                .map(|p| (score(&p, query, app, exe), p))
                .filter(|(s, _)| *s > 0.0)
                .collect();
            ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            if ranked.is_empty() {
                return Ok(format!(
                    "No playbook match for {query:?}. Explore with see→act→verify, then playbook save."
                ));
            }
            let mut out = format!("Matched {} playbook(s):\n\n", ranked.len().min(MAX_LOOKUP));
            for (s, pb) in ranked.into_iter().take(MAX_LOOKUP) {
                out.push_str(&format!("### score={s:.2}\n{}\n\n", format_playbook(&pb)));
            }
            Ok(out)
        }
        "save" => {
            let intent = args
                .get("intent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if intent.is_empty() {
                return Err(ToolError::new("save needs intent"));
            }
            let steps = parse_steps(
                args.get("steps")
                    .ok_or_else(|| ToolError::new("save needs steps[]"))?,
            )?;
            let now = now_secs();
            let replace_id = args
                .get("id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty());
            let mut pb = if let Some(id) = replace_id {
                let mut existing = load_one(id)?;
                existing.intent = intent.to_string();
                existing.steps = steps;
                existing.updated_at = now;
                existing.status = PlaybookStatus::Active;
                existing.confidence = (existing.confidence + SUCCESS_BOOST).min(0.99);
                existing.success_count = existing.success_count.saturating_add(1);
                existing
            } else {
                Playbook {
                    id: Uuid::new_v4().to_string(),
                    intent: intent.to_string(),
                    app: None,
                    exe: None,
                    version_last_ok: None,
                    steps,
                    success_assert: None,
                    confidence: default_confidence(),
                    status: PlaybookStatus::Active,
                    success_count: 1,
                    fail_count: 0,
                    created_at: now,
                    updated_at: now,
                }
            };
            if let Some(app) = args.get("app").and_then(|v| v.as_str()) {
                pb.app = Some(app.to_string());
            }
            if let Some(exe) = args.get("exe").and_then(|v| v.as_str()) {
                pb.exe = Some(exe.to_string());
            }
            if let Some(v) = args.get("version").and_then(|v| v.as_str()) {
                pb.version_last_ok = Some(v.to_string());
            }
            if let Some(a) = args.get("success_assert").and_then(|v| v.as_str()) {
                pb.success_assert = Some(a.to_string());
            }
            save_one(&pb)?;
            Ok(format!(
                "Saved playbook `{}` intent={:?} steps={} conf={:.2}",
                pb.id,
                pb.intent,
                pb.steps.len(),
                pb.confidence
            ))
        }
        "fail" | "report_fail" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::new("fail needs id"))?;
            let mut pb = load_one(id)?;
            pb.fail_count = pb.fail_count.saturating_add(1);
            pb.confidence = (pb.confidence - FAIL_PENALTY).max(0.05);
            pb.updated_at = now_secs();
            if pb.confidence < QUARANTINE_BELOW {
                pb.status = PlaybookStatus::Quarantined;
            }
            let step_hint = args
                .get("step")
                .and_then(|v| v.as_u64())
                .map(|n| format!(" failed_step≈{n}"))
                .unwrap_or_default();
            let reason = args
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            save_one(&pb)?;
            Ok(format!(
                "Recorded fail for `{}` conf={:.2} status={:?}{step_hint}{}. Explore, then playbook save with id={} to patch.",
                pb.id,
                pb.confidence,
                pb.status,
                if reason.is_empty() {
                    String::new()
                } else {
                    format!(" reason={reason}")
                },
                pb.id
            ))
        }
        "success" | "confirm" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::new("success needs id"))?;
            let mut pb = load_one(id)?;
            pb.success_count = pb.success_count.saturating_add(1);
            pb.confidence = (pb.confidence + SUCCESS_BOOST).min(0.99);
            pb.status = PlaybookStatus::Active;
            pb.updated_at = now_secs();
            if let Some(v) = args.get("version").and_then(|v| v.as_str()) {
                pb.version_last_ok = Some(v.to_string());
            }
            save_one(&pb)?;
            Ok(format!(
                "Confirmed playbook `{}` conf={:.2} ok/fail={}/{}",
                pb.id, pb.confidence, pb.success_count, pb.fail_count
            ))
        }
        "delete" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::new("delete needs id"))?;
            let path = path_for(id);
            if !path.exists() {
                return Err(ToolError::new(format!("playbook not found: {id}")));
            }
            fs::remove_file(&path).map_err(|e| ToolError::new(format!("playbook delete: {e}")))?;
            Ok(format!("Deleted playbook `{id}`"))
        }
        other => Err(ToolError::new(format!(
            "playbook unknown op `{other}` (list|lookup|save|fail|success|delete)"
        ))),
    }
}

/// All learned playbooks for the Computer Use settings UI.
pub fn list_for_ui() -> Result<Vec<Playbook>, ToolError> {
    load_all()
}

/// One playbook by id for the settings detail pane.
pub fn get_for_ui(id: &str) -> Result<Playbook, ToolError> {
    load_one(id)
}

/// Delete a learned playbook from disk (settings UI).
pub fn delete_for_ui(id: &str) -> Result<(), ToolError> {
    let path = path_for(id);
    if !path.exists() {
        return Err(ToolError::new(format!("playbook not found: {id}")));
    }
    fs::remove_file(&path).map_err(|e| ToolError::new(format!("playbook delete: {e}")))
}
