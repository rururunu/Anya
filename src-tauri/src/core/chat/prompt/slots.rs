use super::ImageModePolicy;
use crate::core::chat::limits::{
    truncate_chars, ACTIVE_FILE_MAX_CHARS, ACTIVE_WINDOW_MAX_CHARS, CLIPBOARD_MAX_CHARS,
    CONTEXT_BLOCKS_TOTAL_MAX_CHARS, GIT_STATUS_MAX_CHARS, IDE_SELECTION_MAX_CHARS,
    LAST_SHELL_EXECUTION_MAX_CHARS, MEMORIES_MAX_CHARS, RULES_MAX_CHARS, SELECTED_FILES_MAX_CHARS,
};
use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role};

use crate::core::chat::prompts::{
    COMPANION_ORIGIN_PROMPT, IMAGE_MODE_PROMPT, MINIMAL_CODING_PROMPT,
    MULTI_MODEL_COLLABORATION_PROMPT, PLAN_MODE_PROMPT, SYSTEM_PROMPT,
};

/// Slot [4]: optional strategies in fixed relative order. Disabled strategies
/// omit their message entirely, but never insert ahead of slots [0]–[3].
pub(super) fn inject_optional_policy_suffix(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    collaboration_models: &[String],
    minimal_coding: bool,
    plan_mode: bool,
    companion_origin: bool,
    image_mode: Option<&ImageModePolicy>,
) {
    if companion_origin {
        inject_system_block(
            messages,
            session_id,
            "companion-origin",
            Some(COMPANION_ORIGIN_PROMPT),
        );
    }
    if !collaboration_models.is_empty() {
        let list =
            crate::core::ai::model_ref::format_collaboration_prompt_ids(collaboration_models);
        let content = MULTI_MODEL_COLLABORATION_PROMPT.replace("{{MODELS}}", &list);
        inject_system_block(messages, session_id, "collaboration-models", Some(&content));
    }
    if minimal_coding {
        inject_system_block(
            messages,
            session_id,
            "minimal-coding",
            Some(MINIMAL_CODING_PROMPT),
        );
    }
    if plan_mode {
        inject_system_block(messages, session_id, "plan-mode", Some(PLAN_MODE_PROMPT));
    }
    if let Some(image) = image_mode {
        let style_block = if image.style_prompt.trim().is_empty() {
            String::new()
        } else {
            format!(
                "   Mandatory visual style. Put these rendering instructions at the START of the generate_image prompt, before the subject. Follow them strictly so the result is unmistakably this medium — do not fall back to a generic or photoreal look unless the style is photography:\n   {}\n",
                image.style_prompt.trim()
            )
        };
        let reference_block = if image.has_reference {
            "   A reference image is already attached. The runtime will call the Images edits API (image-to-image) with that reference. Write `prompt` as an edit or restyle instruction; do not skip generate_image.\n".to_string()
        } else {
            String::new()
        };
        let content = IMAGE_MODE_PROMPT
            .replace("{{SIZE}}", &image.size)
            .replace("{{QUALITY}}", &image.quality)
            .replace("{{N}}", &image.n.to_string())
            .replace("{{STYLE_BLOCK}}", &style_block)
            .replace("{{REFERENCE_BLOCK}}", &reference_block);
        inject_system_block(messages, session_id, "image-mode", Some(&content));
    }
}

pub(super) fn inject_system_block(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    kind: &str,
    block: Option<&str>,
) {
    let Some(content) = block.map(str::trim).filter(|content| !content.is_empty()) else {
        return;
    };
    let capped = truncate_chars(content, RULES_MAX_CHARS);
    messages.push(ChatMessage {
        id: format!("{kind}-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: capped,
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
        estimated_tokens: None,
    });
}

pub(super) fn inject_memories(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    memories: Option<&str>,
) {
    let Some(content) = memories
        .map(str::trim)
        .filter(|content| !content.is_empty())
    else {
        return;
    };
    let capped = truncate_chars(content, MEMORIES_MAX_CHARS);
    messages.push(ChatMessage {
        id: format!("memories-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: capped,
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
        estimated_tokens: None,
    });
}

pub(super) fn system_message(session_id: &str) -> ChatMessage {
    ChatMessage {
        id: format!("system-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: SYSTEM_PROMPT.to_string(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
        estimated_tokens: None,
    }
}

pub(super) fn split_current_user(
    history: &[ChatMessage],
) -> (Vec<ChatMessage>, Option<ChatMessage>) {
    if history.is_empty() {
        return (Vec::new(), None);
    }

    if history
        .last()
        .is_some_and(|message| message.role == Role::User)
    {
        let prior = history[..history.len() - 1].to_vec();
        let current = history.last().cloned();
        return (prior, current);
    }

    (history.to_vec(), None)
}

pub(super) fn inject_context(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    context: &RequestContext,
) {
    let mut blocks = Vec::new();

    if let Some(ide) = &context.ide_context {
        let mut lines = vec![format!("IDE:\n{}", ide_display_name(&ide.ide))];
        if let Some(workspace) = &ide.workspace {
            lines.push(format!("Workspace:\n{}", workspace.display()));
        }
        if let Some(active_file) = &ide.active_file {
            lines.push(format!("Active File:\n{}", active_file.display()));
        }
        if let Some(language) = ide
            .language
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("Language:\n{language}"));
        }
        if let Some(cursor) = &ide.cursor {
            lines.push(format!(
                "Position:\nLine {}, Column {}",
                cursor.line, cursor.column
            ));
        }
        if let Some(selection) = ide
            .selection
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!(
                "Selection:\n{}",
                truncate_chars(selection, IDE_SELECTION_MAX_CHARS)
            ));
        }
        blocks.push(format!("[IDE Context]\n{}", lines.join("\n\n")));
    }
    if let Some(office) = &context.office_context {
        blocks.push(crate::core::office::context::format_office_context_block(
            office,
        ));
    }
    if let Some(workspace) = &context.workspace {
        blocks.push(format!(
            "[Current Workspace]\nName: {}\nRoot Directory: {}\nTreat this as the exact active project. All file operations must use this root via built-in workspace tools (read_file, find_files, search, shell). Do not infer another project from memory, conversation history, the application identity, the active window, or MCP filesystem allow-lists — those are not the workspace root. When asked which workspace is active, answer with this name and root.",
            workspace.name, workspace.root
        ));
    }
    if !context.selected_files.is_empty() {
        let files = truncate_chars(&context.selected_files.join("\n"), SELECTED_FILES_MAX_CHARS);
        blocks.push(format!(
            "[Selected Files]\nPaths are relative to the current workspace root.\n{files}"
        ));
    }
    if let Some(active_window) = non_empty(&context.active_window) {
        let capped = truncate_chars(&active_window, ACTIVE_WINDOW_MAX_CHARS);
        blocks.push(format!("[Active Window]\n{capped}"));
    }
    if let Some(clipboard) = non_empty(&context.clipboard) {
        let capped = truncate_chars(&clipboard, CLIPBOARD_MAX_CHARS);
        blocks.push(format!("[Clipboard]\n{capped}"));
    }
    if let Some(active_file) = non_empty(&context.active_file) {
        let capped = truncate_chars(&active_file, ACTIVE_FILE_MAX_CHARS);
        blocks.push(format!("[Active File]\n{capped}"));
    }
    if let Some(git_status) = non_empty(&context.git_status) {
        let capped = truncate_chars(&git_status, GIT_STATUS_MAX_CHARS);
        blocks.push(format!("[Git Status]\n{capped}"));
    }
    if let Some(shell) = non_empty(&context.last_shell_execution) {
        let capped = truncate_chars(&shell, LAST_SHELL_EXECUTION_MAX_CHARS);
        blocks.push(format!("[Last Agent Shell Execution]\n{capped}"));
    }

    if blocks.is_empty() {
        return;
    }

    let mut content = String::new();
    for block in blocks {
        let next = if content.is_empty() {
            block
        } else {
            format!("{content}\n\n{block}")
        };
        if next.chars().count() > CONTEXT_BLOCKS_TOTAL_MAX_CHARS {
            content = truncate_chars(&next, CONTEXT_BLOCKS_TOTAL_MAX_CHARS);
            break;
        }
        content = next;
    }

    messages.push(ChatMessage {
        // Stable id — never depends on message count / optional suffixes.
        id: format!("context-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content,
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
        estimated_tokens: None,
    });
}

fn ide_display_name(ide: &str) -> String {
    match ide.trim().to_ascii_lowercase().as_str() {
        "vscode" | "visual studio code" => "VSCode".to_string(),
        "idea" | "intellij" | "intellij idea" => "IntelliJ IDEA".to_string(),
        _ => ide.trim().to_string(),
    }
}

fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}
