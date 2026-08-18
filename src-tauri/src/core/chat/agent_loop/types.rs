use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::{ChatRequest, ToolCallPayload};

/// A tool call that has started executing: its UI activity has already been
/// created/emitted, so callers only need to run it and report the outcome.
pub struct StartedTool {
    pub call_id: String,
    pub tool_name: String,
    pub activity_id: String,
    pub args: serde_json::Value,
    pub preview_detail: Option<String>,
    pub tool_preview: Option<crate::core::tools::preview::ToolPreview>,
}

/// Result of running one tool call, ready to be folded back into the request
/// as a `Role::Tool` message and inspected by the completion/failure gates.
pub struct ToolOutcome {
    pub call_id: String,
    pub tool_name: String,
    /// Serialized arguments, used to detect repeated identical calls.
    pub arguments: String,
    pub result: String,
    pub success: bool,
    pub user_denied: bool,
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn merge_tool_call(calls: &mut Vec<ToolCallPayload>, incoming: ToolCallPayload) {
    if !incoming.id.is_empty() {
        if let Some(existing) = calls.iter_mut().find(|call| call.id == incoming.id) {
            if !incoming.name.is_empty() {
                existing.name = incoming.name;
            }
            existing.arguments.push_str(&incoming.arguments);
            return;
        }
    }
    calls.push(incoming);
}

/// Prompt-side token estimate used to decide mid-turn compact.
///
/// Tool schemas are a fixed per-request overhead: they cannot be folded, and
/// counting them against the compact threshold made every agent step look
/// over-budget (especially with MCP), so list/read rounds spent seconds
/// re-compacting the same history.
pub fn estimate_request_tokens(request: &ChatRequest) -> usize {
    request.messages.iter().map(estimate_message_tokens).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role};
    use serde_json::json;

    #[test]
    fn estimate_ignores_tool_schemas() {
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            messages: vec![ChatMessage {
                id: "u1".into(),
                session_id: "s".into(),
                role: Role::User,
                content: "hello".into(),
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 1,
                estimated_tokens: None,
            }],
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from(vec![json!({
                "type": "function",
                "function": {
                    "name": "huge",
                    "description": "x".repeat(80_000),
                    "parameters": { "type": "object" }
                }
            })]),
            temperature: None,
            max_tokens: None,
        };
        let with_tools = estimate_request_tokens(&request);
        let mut without = request.clone();
        without.tools = std::sync::Arc::from([]);
        assert_eq!(with_tools, estimate_request_tokens(&without));
        assert!(with_tools < 100);
    }
}
