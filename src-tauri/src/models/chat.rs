use serde::{Deserialize, Serialize};

pub use crate::core::runtime::ChatMessage;
use crate::models::settings::{ChatMode, ToolApprovalMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub quick_ask: bool,
    /// Per-conversation model override. Each conversation remembers its own
    /// model/mode/approval choice; a missing value falls back to global settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<ChatMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_approval_mode: Option<ToolApprovalMode>,
    /// When true, do not auto-enter plan mode for this send (e.g. approve & execute).
    #[serde(default)]
    pub skip_auto_plan: bool,
    /// True when this send is the "approve plan & execute" continuation: the
    /// message drives the turn prompt but is never persisted to chat history.
    #[serde(default)]
    pub resume_plan: bool,
}

/// Optional per-send settings that override global settings for one conversation.
#[derive(Debug, Clone, Default)]
pub struct ChatSendOverrides {
    pub model_id: Option<String>,
    pub model_provider: Option<String>,
    pub chat_mode: Option<ChatMode>,
    pub tool_approval_mode: Option<ToolApprovalMode>,
    pub skip_auto_plan: bool,
    pub resume_plan: bool,
}

impl ChatSendOverrides {
    pub fn from_request(request: &ChatSendRequest) -> Self {
        Self {
            model_id: request.model_id.clone(),
            model_provider: request.model_provider.clone(),
            chat_mode: request.chat_mode,
            tool_approval_mode: request.tool_approval_mode,
            skip_auto_plan: request.skip_auto_plan,
            resume_plan: request.resume_plan,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendResponse {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatCancelRequest {
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStartedEvent {
    pub session_id: String,
    pub user_message: ChatMessage,
    pub assistant_message: ChatMessage,
    #[serde(default)]
    pub resume_plan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatDeltaEvent {
    pub session_id: String,
    pub message_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatReasoningEvent {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStatusEvent {
    pub session_id: String,
    pub message_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserContentEvent {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatFinishedEvent {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionTitleUpdatedEvent {
    pub session_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatErrorEvent {
    pub session_id: String,
    pub message_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatContextNoticeEvent {
    pub session_id: String,
    pub kind: String,
    pub message: String,
    pub usage_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folded_messages: Option<usize>,
    #[serde(default)]
    pub estimated_tokens: usize,
    #[serde(default)]
    pub context_window_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsageRequest {
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<crate::core::runtime::RequestContext>,
    /// Active chat model — used to cap the context window (1M toggle ≠ every model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsageResponse {
    pub usage_ratio: f32,
    pub estimated_tokens: usize,
    pub context_window_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryResponse {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionSummary {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub preview: String,
    pub message_count: usize,
    pub turn_count: usize,
    pub estimated_tokens: usize,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChatSessionsResponse {
    pub sessions: Vec<ChatSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelThinkingVariant {
    /// Catalog model id used in API requests.
    pub id: String,
    /// Short tier label (e.g. `"High"`, `"Low"`, `"Agent"`).
    pub label: String,
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModelInfo {
    pub id: String,
    pub owned_by: String,
    /// Stable provider key used by the UI for icons (e.g. `"deepseek"`).
    pub provider: String,
    /// Human-readable label for pickers (e.g. `"Gemini 3.1 Pro (High)"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Alternate thinking tiers for the same model family (High / Low / Agent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_variants: Option<Vec<ModelThinkingVariant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserOption {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserQuestion {
    pub header: String,
    pub question: String,
    pub options: Vec<AskUserOption>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserEvent {
    pub session_id: String,
    pub request_id: String,
    pub questions: Vec<AskUserQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondAskUserRequest {
    pub request_id: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPermissionEvent {
    pub session_id: String,
    pub request_id: String,
    pub path: String,
    pub operation: String,
    pub tool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondPathPermissionRequest {
    pub request_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionResolvedEvent {
    pub request_id: String,
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_request_defaults_resume_plan_to_false() {
        let request: ChatSendRequest =
            serde_json::from_str(r#"{"message":"hello"}"#).expect("request parses");
        assert!(!request.resume_plan);
        assert!(!request.skip_auto_plan);
    }

    #[test]
    fn from_request_maps_resume_plan_and_skip_auto_plan() {
        let request: ChatSendRequest =
            serde_json::from_str(r#"{"message":"go","skipAutoPlan":true,"resumePlan":true}"#)
                .expect("request parses");
        let overrides = ChatSendOverrides::from_request(&request);
        assert!(overrides.resume_plan);
        assert!(overrides.skip_auto_plan);
    }

    #[test]
    fn chat_started_event_serializes_resume_plan_camel_case() {
        let message = ChatMessage {
            id: "m1".into(),
            session_id: "s1".into(),
            role: crate::core::runtime::Role::User,
            content: "go".into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: crate::core::runtime::MessageStatus::Done,
            timestamp: 0,
            estimated_tokens: None,
        };
        let event = ChatStartedEvent {
            session_id: "s1".into(),
            user_message: message.clone(),
            assistant_message: message,
            resume_plan: true,
        };
        let value = serde_json::to_value(&event).expect("event serializes");
        assert_eq!(value["resumePlan"], true);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActivityEvent {
    pub session_id: String,
    pub message_id: String,
    pub activity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_activity_id: Option<String>,
    pub tool_name: String,
    pub title: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<crate::core::tools::preview::ToolPreview>,
    #[serde(default)]
    pub success: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListUpdatedEvent {
    pub session_id: String,
    pub tasks: Vec<crate::core::tools::context::TaskItem>,
}
