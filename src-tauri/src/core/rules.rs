use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::tools::error::ToolError;
use crate::core::tools::memory::{shared_memory_store, MemoryDecision, MemoryRuleEngine};

const MAX_RULE_FILES: usize = 8;
const MAX_RULE_DEPTH: usize = 3;
const MAX_RULE_CHARS: usize = 24_000;

/// Prefer lowercase `agent.md` (user-facing), then keep `AGENTS.md` compatibility.
const PROJECT_RULE_CANDIDATES: &[&str] = &["agent.md", "Agent.md", "AGENTS.md", "agents.md"];

pub struct TaskRules {
    pub recalled_memories: Option<String>,
    pub project_rules: Option<String>,
    /// Explicit `#skill:` / `#mcp:` selections from the user message.
    pub preferred_resources: Option<String>,
    pub memory_decision: MemoryDecision,
}

pub struct RuleEngine;

impl RuleEngine {
    pub fn prepare_task(
        user_message: &str,
        workspace_root: Option<&Path>,
        is_new_session: bool,
    ) -> TaskRules {
        TaskRules {
            recalled_memories: should_recall_memory(user_message, is_new_session)
                .then(|| shared_memory_store().recall_block(user_message))
                .flatten(),
            // Always attempt project rules when a workspace is active.
            // Missing files are ignored (returns None).
            project_rules: workspace_root.and_then(load_project_rules),
            preferred_resources: format_preferred_resources(user_message),
            memory_decision: MemoryRuleEngine::evaluate(user_message, false),
        }
    }

    pub fn authorize_tool(tool_name: &str, args: &Value) -> Result<(), ToolError> {
        if tool_name == "run_shell" {
            let command = args.get("command").and_then(Value::as_str).unwrap_or("");
            reject_dangerous_shell(command)?;
        }
        if tool_name == "save_memory" {
            let title = args.get("title").and_then(Value::as_str).unwrap_or("");
            let content = args.get("content").and_then(Value::as_str).unwrap_or("");
            match MemoryRuleEngine::evaluate(&format!("{title}\n{content}"), true) {
                MemoryDecision::Accept => {}
                MemoryDecision::Infer => {
                    return Err(ToolError::new(
                        "rule denied tool: memory requires inference",
                    ));
                }
                MemoryDecision::RejectSensitive => {
                    return Err(ToolError::new("rule denied tool: sensitive memory content"));
                }
                MemoryDecision::RejectTransient => {
                    return Err(ToolError::new("rule denied tool: invalid memory content"));
                }
            }
        }
        Ok(())
    }
}

fn should_recall_memory(message: &str, is_new_session: bool) -> bool {
    let normalized = message.trim().to_lowercase();
    if normalized.is_empty() || is_low_information_message(&normalized) {
        return false;
    }
    if is_new_session {
        return true;
    }
    [
        "remember",
        "previous",
        "last time",
        "before",
        "as usual",
        "my preference",
        "you know",
        "still use",
        "same as",
        "还记得",
        "之前",
        "上次",
        "以前",
        "按我的",
        "我的偏好",
        "我的习惯",
        "照旧",
        "一贯",
        "还是用",
    ]
    .iter()
    .any(|cue| normalized.contains(cue))
}

fn is_low_information_message(message: &str) -> bool {
    let compact = message.trim_matches(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation());
    if compact.chars().count() > 12 {
        return false;
    }
    [
        "hi",
        "hello",
        "hey",
        "ok",
        "okay",
        "thanks",
        "thank you",
        "continue",
        "yes",
        "no",
        "你好",
        "您好",
        "嗨",
        "好的",
        "好",
        "谢谢",
        "继续",
        "是",
        "否",
        "可以",
    ]
    .contains(&compact)
}

fn load_project_rules(root: &Path) -> Option<String> {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let entry = PROJECT_RULE_CANDIDATES
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())?;
    let mut visited = HashSet::new();
    let mut output = String::new();
    load_rule_file(&root, &entry, 0, &mut visited, &mut output);
    (!output.trim().is_empty()).then(|| {
        format!(
            "<project-rules source=\"{}\">\nRules loaded from the active workspace. Follow them for this task.\n{}\n</project-rules>",
            entry
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("agent.md"),
            output.trim()
        )
    })
}

/// Parse `#skill:name` / `#mcp:id` tokens into a system hint block.
fn format_preferred_resources(user_message: &str) -> Option<String> {
    let mut skills = Vec::new();
    let mut mcps = Vec::new();
    let mut rest = user_message;
    while let Some(hash) = rest.find('#') {
        rest = &rest[hash + 1..];
        let (kind, after_kind) = if let Some(rest_skill) = rest.strip_prefix("skill:") {
            ("skill", rest_skill)
        } else if let Some(rest_mcp) = rest.strip_prefix("mcp:") {
            ("mcp", rest_mcp)
        } else {
            continue;
        };
        let id: String = after_kind
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
            .collect();
        if id.is_empty() {
            continue;
        }
        let id_len = id.len();
        match kind {
            "skill" => {
                if !skills.iter().any(|existing: &String| existing == &id) {
                    skills.push(id);
                }
            }
            "mcp" => {
                if !mcps.iter().any(|existing: &String| existing == &id) {
                    mcps.push(id);
                }
            }
            _ => {}
        }
        rest = &after_kind[id_len.min(after_kind.len())..];
    }
    if skills.is_empty() && mcps.is_empty() {
        return None;
    }
    let mut lines = vec![
        "<preferred-resources>".to_string(),
        "The user explicitly selected these resources for this task. Prefer them over alternatives.".to_string(),
    ];
    for name in &skills {
        lines.push(format!(
            "- Skill `{name}`: load with `load_skill` / `run_skill`, or call the dedicated skill tool when available."
        ));
    }
    for id in &mcps {
        lines.push(format!(
            "- MCP server `{id}`: prefer tools whose names start with `mcp__{id}__`."
        ));
    }
    lines.push("</preferred-resources>".to_string());
    Some(lines.join("\n"))
}

fn load_rule_file(
    root: &Path,
    path: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    output: &mut String,
) {
    if depth > MAX_RULE_DEPTH
        || visited.len() >= MAX_RULE_FILES
        || output.chars().count() >= MAX_RULE_CHARS
    {
        return;
    }
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !resolved.starts_with(root) || !visited.insert(resolved.clone()) {
        return;
    }
    let Ok(content) = fs::read_to_string(&resolved) else {
        return;
    };
    let relative = resolved.strip_prefix(root).unwrap_or(&resolved);
    output.push_str(&format!("\n## {}\n", relative.display()));
    output.extend(
        content
            .chars()
            .take(MAX_RULE_CHARS.saturating_sub(output.chars().count())),
    );

    let Some(parent) = resolved.parent() else {
        return;
    };
    for line in content.lines() {
        let Some(reference) = line.trim().strip_prefix('@') else {
            continue;
        };
        let reference = reference.trim();
        if reference.is_empty() || Path::new(reference).is_absolute() || reference.contains("..") {
            continue;
        }
        load_rule_file(root, &parent.join(reference), depth + 1, visited, output);
    }
}

fn reject_dangerous_shell(command: &str) -> Result<(), ToolError> {
    crate::core::tools::sandbox::reject_dangerous_shell(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn denies_destructive_shell_commands() {
        let error = RuleEngine::authorize_tool(
            "run_shell",
            &json!({ "command": "git reset --hard HEAD~1" }),
        )
        .unwrap_err();
        assert!(error.message.contains("rule denied"));
    }

    #[test]
    fn allows_normal_tool_calls() {
        RuleEngine::authorize_tool("read_file", &json!({ "path": "README.md" })).unwrap();
        RuleEngine::authorize_tool("run_shell", &json!({ "command": "cargo test" })).unwrap();
    }

    #[test]
    fn recalls_for_new_tasks_and_cross_chat_cues() {
        assert!(should_recall_memory("Fix the build failure", true));
        assert!(should_recall_memory(
            "Use my previous formatting preference",
            false
        ));
        assert!(should_recall_memory("还是按我的习惯处理", false));
    }

    #[test]
    fn skips_recall_for_low_information_and_regular_followups() {
        assert!(!should_recall_memory("hello", true));
        assert!(!should_recall_memory("继续", false));
        assert!(!should_recall_memory("Run the tests now", false));
    }

    #[test]
    fn loads_agents_file_and_safe_references() {
        let root = std::env::temp_dir().join(format!("peek-rules-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("AGENTS.md"),
            "Use project conventions.\n@RTK.md\n@../outside.md",
        )
        .unwrap();
        fs::write(root.join("RTK.md"), "Use rtk for compact output.").unwrap();

        let rules = load_project_rules(&root).unwrap();

        assert!(rules.contains("Use project conventions."));
        assert!(rules.contains("Use rtk for compact output."));
        assert!(!rules.contains("## ../outside.md"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefers_agent_md_over_agents_md() {
        let root = std::env::temp_dir().join(format!("peek-rules-agent-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("agent.md"), "From agent.md").unwrap();
        fs::write(root.join("AGENTS.md"), "From AGENTS.md").unwrap();
        let rules = load_project_rules(&root).unwrap();
        assert!(rules.contains("From agent.md"));
        assert!(!rules.contains("From AGENTS.md"));
        assert!(rules.contains("source=\"agent.md\""));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn formats_preferred_skill_and_mcp_tokens() {
        let block =
            format_preferred_resources("Use #skill:generate_word and #mcp:filesystem please")
                .unwrap();
        assert!(block.contains("generate_word"));
        assert!(block.contains("mcp__filesystem__"));
        assert!(block.contains("<preferred-resources>"));
    }

    #[test]
    fn ignores_messages_without_hash_resources() {
        assert!(format_preferred_resources("just a normal question").is_none());
    }
}
