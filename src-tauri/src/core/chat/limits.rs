//! Shared text/token limits for prompt assembly and tool loops.

use regex::Regex;

use crate::core::runtime::ChatMessage;

/// Characters → estimated tokens (coarse).
pub const CHARS_PER_TOKEN: usize = 4;

pub const TOOL_OUTPUT_MAX_CHARS: usize = 12_000;
pub const CLIPBOARD_MAX_CHARS: usize = 8_000;
pub const ACTIVE_WINDOW_MAX_CHARS: usize = 2_000;
pub const ACTIVE_FILE_MAX_CHARS: usize = 2_000;
pub const IDE_SELECTION_MAX_CHARS: usize = 8_000;
pub const GIT_STATUS_MAX_CHARS: usize = 4_000;
pub const LAST_SHELL_EXECUTION_MAX_CHARS: usize = 4_000;
pub const SELECTED_FILES_MAX_CHARS: usize = 4_000;
pub const RULES_MAX_CHARS: usize = 8_000;
pub const MEMORIES_MAX_CHARS: usize = 8_000;
pub const CONTEXT_BLOCKS_TOTAL_MAX_CHARS: usize = 16_000;

/// Max agent tool-loop iterations per turn. `0` = unlimited.
///
/// Prefer consecutive-failure circuit breaker; a hard step cap often aborts
/// real multi-file work mid-task. Context pressure is handled Codex-style:
/// auto-compact near the window and continue (no hard token stop).
pub const DEFAULT_MAX_STEPS: u32 = 0;
/// Consecutive failed tool *steps* before injecting a change-strategy challenge.
/// A step is one model tool batch; parallel children in that batch count as one.
pub const CONSECUTIVE_TOOL_FAILURE_CHALLENGE: u32 = 3;
/// Consecutive failed tool *steps* before a hard stop. Keep this well above the
/// challenge threshold so exploratory web/booking work can recover.
pub const MAX_CONSECUTIVE_TOOL_FAILURES: u32 = 8;
/// Mid-turn auto-compact window when large context is off (keep in sync with `compact::DEFAULT_CONTEXT_WINDOW`).
pub const DEFAULT_MAX_TURN_TOKENS: usize = 64_000;
/// Mid-turn auto-compact window when large context is on (keep in sync with `compact::LARGE_CONTEXT_WINDOW`).
pub const LARGE_MAX_TURN_TOKENS: usize = 1_000_000;

/// Context window used as the mid-turn auto-compact basis (not a hard stop).
/// Prefer [`crate::core::chat::model_context::effective_context_window`] with a model id.
#[allow(dead_code)]
pub fn max_turn_tokens_for(large_context_enabled: bool) -> usize {
    if large_context_enabled {
        LARGE_MAX_TURN_TOKENS
    } else {
        DEFAULT_MAX_TURN_TOKENS
    }
}

pub const LLM_COMPACT_TIMEOUT_SECS: u64 = 8;
pub const FOLD_PAYLOAD_MSG_MAX_CHARS: usize = 800;
pub const FOLD_PAYLOAD_TOTAL_MAX_CHARS: usize = 24_000;

/// Cap MCP tools registered from a single server.
pub const MCP_MAX_TOOLS_PER_SERVER: usize = 64;
/// Cap total MCP dynamic tools across all servers for one registry refresh.
pub const MCP_MAX_TOTAL_TOOLS: usize = 128;
/// Soft cap on serialized inputSchema / description size per MCP tool.
pub const MCP_MAX_TOOL_SCHEMA_CHARS: usize = 8_000;

/// Caps applied when persisting tool/timeline JSON into SQLite so one message
/// row cannot grow without bound from large tool results or diffs.
pub const STORED_TOOL_RESULT_MAX_CHARS: usize = 12_000;
pub const STORED_PREVIEW_TEXT_MAX_CHARS: usize = 6_000;
pub const STORED_TIMELINE_ITEM_MAX_CHARS: usize = 32_000;
pub const STORED_TOOL_CALL_ARGS_MAX_CHARS: usize = 8_000;

pub fn estimate_tokens(text: &str) -> usize {
    if !text.contains("data:image/") {
        let chars = text.chars().count();
        return (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });
    }

    // Strip out base64 image contents to avoid huge token estimation
    let re = match Regex::new(r"data:image/[^)]+") {
        Ok(re) => re,
        Err(_) => {
            let chars = text.chars().count();
            return (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });
        }
    };

    let cleaned = re.replace_all(text, "image_placeholder");
    let chars = cleaned.chars().count();
    let base_tokens = (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });

    let image_count = text.matches("data:image/").count();
    base_tokens + (image_count * 1000)
}

/// Estimate the tokens represented by one persisted chat message.
///
/// Reasoning and tool payloads are included because they contribute to the
/// conversation even when rendered outside the main message body.
pub fn estimate_message_tokens(message: &ChatMessage) -> usize {
    let mut total = estimate_tokens(&message.content)
        + estimate_tokens(message.reasoning.as_deref().unwrap_or(""));

    if let Some(activities) = &message.tool_activities {
        for activity in activities {
            total += estimate_tokens(&activity.tool_name);
            total += estimate_tokens(&activity.title);
            total += estimate_tokens(activity.detail.as_deref().unwrap_or(""));
            total += estimate_tokens(activity.result.as_deref().unwrap_or(""));
            if let Some(arguments) = &activity.arguments {
                total += estimate_tokens(&arguments.to_string());
            }
        }
    }

    if let Some(calls) = &message.tool_calls {
        for call in calls {
            total += estimate_tokens(&call.name);
            total += estimate_tokens(&call.arguments);
        }
    }

    total + 4
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

/// Keep head and tail so models still see start + end of large tool output.
pub fn truncate_tool_output(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars < 64 {
        return truncate_chars(text, max_chars);
    }
    let omitted = count - max_chars;
    let head_budget = max_chars * 2 / 3;
    let tail_budget = max_chars - head_budget;
    let head: String = text.chars().take(head_budget).collect();
    let tail: String = text
        .chars()
        .skip(count.saturating_sub(tail_budget))
        .collect();
    format!("{head}\n…[truncated {omitted} chars]…\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{MessageStatus, Role};

    #[test]
    fn tool_output_keeps_head_and_tail() {
        let text = "A".repeat(100) + &"B".repeat(100);
        let out = truncate_tool_output(&text, 80);
        assert!(out.contains("truncated"));
        assert!(out.starts_with('A'));
        assert!(out.ends_with('B'));
        assert!(out.chars().count() < text.chars().count());
    }

    #[test]
    fn message_estimate_includes_content_reasoning_and_overhead() {
        let message = ChatMessage {
            id: "message-1".into(),
            session_id: "session-1".into(),
            role: Role::Assistant,
            content: "12345678".into(),
            reasoning: Some("1234".into()),
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        };

        assert_eq!(estimate_message_tokens(&message), 7);
    }
}
