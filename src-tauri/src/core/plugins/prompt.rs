//! System-prompt catalog so the chat agent uses enabled plugin tools.

use super::manifest::tool_prefix;

const INTRO: &str = "Enabled agent plugins add tools to this turn. When the user asks for something they cover, call those tools. Do not say Anya cannot do it.";
const EXTRA_PROMPT_MAX_CHARS: usize = 12_000;
const MAX_PLUGIN_SKILL_FILES: usize = 8;

pub fn assemble(blocks: &[String]) -> Option<String> {
    if blocks.is_empty() {
        return None;
    }
    Some(format!("{INTRO}\n\n{}", blocks.join("\n\n")))
}

pub fn format_block(
    id: &str,
    name: &str,
    description: &str,
    local_tools: &[String],
    extra_prompt: &str,
) -> String {
    let prefix = tool_prefix(id);
    let tools = if local_tools.is_empty() {
        format!("{prefix}*")
    } else {
        local_tools
            .iter()
            .map(|name| format!("{prefix}{name}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut body = format!("### {name} (`{id}`)\nTools: {tools}");
    let desc = description.trim();
    if !desc.is_empty() {
        body.push('\n');
        body.push_str(desc);
    }
    let extra: String = extra_prompt
        .trim()
        .chars()
        .take(EXTRA_PROMPT_MAX_CHARS)
        .collect();
    if !extra.is_empty() {
        body.push('\n');
        body.push_str(&extra);
    }
    body
}

/// Concatenate the manifest prompt with skill markdown (already on disk).
pub fn extra_from_manifest(plugin_id: &str, prompt: &str, skill_rels: &[String]) -> String {
    let bodies: Vec<String> = skill_rels
        .iter()
        .take(MAX_PLUGIN_SKILL_FILES)
        .filter_map(|rel| super::manifest::read_plugin_file(plugin_id, rel).ok())
        .collect();
    join_extra(prompt, &bodies)
}

pub fn join_extra(prompt: &str, skill_bodies: &[String]) -> String {
    let mut extra = String::new();
    let prompt = prompt.trim();
    if !prompt.is_empty() {
        extra.push_str(prompt);
    }
    for body in skill_bodies {
        let t = body.trim();
        if t.is_empty() {
            continue;
        }
        if !extra.is_empty() {
            extra.push_str("\n\n");
        }
        extra.push_str(t);
    }
    extra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_agent_tools_with_intro() {
        let block = format_block(
            "computer-use",
            "Computer Use",
            "See the desktop.",
            &["screenshot".into(), "click".into()],
            "Prefer list_windows and find_control. Screenshot the foreground window when you need layout.",
        );
        let text = assemble(&[block]).unwrap();
        assert!(text.contains("Do not say Anya cannot do it"));
        assert!(text.contains("plugin_computer-use__screenshot"));
        assert!(text.contains("plugin_computer-use__click"));
        assert!(text.contains("Prefer list_windows and find_control"));
        assert!(!text.to_lowercase().contains("always screenshot first"));
    }

    #[test]
    fn joins_prompt_and_skill_bodies() {
        let extra = join_extra("SPEED first.", &["# Paint\nWin+R mspaint".into()]);
        assert!(extra.starts_with("SPEED first."));
        assert!(extra.contains("Win+R mspaint"));
    }

    #[test]
    fn keeps_long_playbook_in_block() {
        let playbook = "mspaint ".repeat(800);
        assert!(playbook.len() > 4000);
        let block = format_block(
            "computer-use",
            "Computer Use",
            "",
            &["screenshot".into()],
            &playbook,
        );
        assert!(block.contains("mspaint"));
        assert!(block.len() > 4000);
    }
}
