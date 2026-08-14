use serde::{Deserialize, Serialize};

use super::stream::ToolCallPayload;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolActivity {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_activity_id: Option<String>,
    pub tool_name: String,
    pub title: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<crate::core::tools::preview::ToolPreview>,
    pub success: bool,
    pub status: String,
}

/// Chronological marker for one piece of assistant work: a run of reasoning
/// text, a run of regular reply text, or a tool call — in the order they
/// actually happened. Persisted alongside the message so history reloads and
/// crash recovery keep narration interleaved with the tool cards it
/// describes, instead of collapsing into "all text, then all tools".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkTimelineItem {
    Reasoning {
        id: String,
        content: String,
    },
    Content {
        id: String,
        content: String,
    },
    Tool {
        id: String,
        #[serde(rename = "toolActivityId")]
        tool_activity_id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Pending,
    Streaming,
    Done,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_timeline: Option<Vec<WorkTimelineItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_activities: Option<Vec<ToolActivity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallPayload>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub status: MessageStatus,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<usize>,
}

impl ChatMessage {
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn with_status(mut self, status: MessageStatus) -> Self {
        self.status = status;
        self
    }

    /// Keep tool-protocol rows and any message with visible text. Empty
    /// pending assistants (no `tool_calls`) are omitted so providers do not
    /// see a blank assistant turn.
    pub fn contributes_to_api(&self) -> bool {
        self.role == Role::Tool
            || self
                .tool_calls
                .as_ref()
                .is_some_and(|calls| !calls.is_empty())
            || !self.content.trim().is_empty()
    }
}

pub const DEFAULT_SESSION_ID: &str = "default";
