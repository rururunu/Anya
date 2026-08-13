use tauri::{AppHandle, Emitter};

use crate::core::event::{BusEvent, EventBus};
use crate::models::chat::{
    AskUserEvent, ChatContextNoticeEvent, ChatDeltaEvent, ChatErrorEvent, ChatFinishedEvent,
    ChatReasoningEvent, ChatSessionTitleUpdatedEvent, ChatStartedEvent, ChatStatusEvent,
    ChatUserContentEvent, PathPermissionEvent, TaskListUpdatedEvent, ToolActivityEvent,
};

pub struct TauriEventBus {
    app: AppHandle,
}

impl TauriEventBus {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventBus for TauriEventBus {
    fn emit(&self, event: BusEvent) {
        crate::core::remote::on_bus_event(&event);
        match event {
            BusEvent::AgentEvent { event } => {
                let _ = self.app.emit("agent-event", event);
            }
            BusEvent::AgentDebugEvent { event } => {
                let _ = self.app.emit("agent-debug-event", event);
            }
            BusEvent::TokenUsage { .. } => {}
            BusEvent::SubagentStarted { .. }
            | BusEvent::SubagentProgress { .. }
            | BusEvent::SubagentFinished { .. } => {}
            BusEvent::ChatStarted {
                session_id,
                user_message,
                assistant_message,
                resume_plan,
            } => {
                let _ = self.app.emit(
                    "chat-started",
                    ChatStartedEvent {
                        session_id,
                        user_message,
                        assistant_message,
                        resume_plan,
                    },
                );
            }
            BusEvent::ChatDelta {
                session_id,
                message_id,
                delta,
            } => {
                let _ = self.app.emit(
                    "chat-delta",
                    ChatDeltaEvent {
                        session_id,
                        message_id,
                        delta,
                    },
                );
            }
            BusEvent::ChatReasoning {
                session_id,
                message_id,
                content,
            } => {
                let _ = self.app.emit(
                    "chat-reasoning",
                    ChatReasoningEvent {
                        session_id,
                        message_id,
                        content,
                    },
                );
            }
            BusEvent::ChatStatus {
                session_id,
                message_id,
                kind,
            } => {
                let _ = self.app.emit(
                    "chat-status",
                    ChatStatusEvent {
                        session_id,
                        message_id,
                        kind,
                    },
                );
            }
            BusEvent::ChatUserContent {
                session_id,
                message_id,
                content,
            } => {
                let _ = self.app.emit(
                    "chat-user-content",
                    ChatUserContentEvent {
                        session_id,
                        message_id,
                        content,
                    },
                );
            }
            BusEvent::ChatFinished {
                session_id,
                message_id,
                content,
                reasoning,
                finish_reason,
            } => {
                let _ = self.app.emit(
                    "chat-finished",
                    ChatFinishedEvent {
                        session_id,
                        message_id,
                        content,
                        reasoning,
                        finish_reason,
                    },
                );
            }
            BusEvent::ChatSessionTitleUpdated { session_id, title } => {
                let _ = self.app.emit(
                    "chat-session-title-updated",
                    ChatSessionTitleUpdatedEvent { session_id, title },
                );
            }
            BusEvent::ChatError {
                session_id,
                message_id,
                message,
            } => {
                let _ = self.app.emit(
                    "chat-error",
                    ChatErrorEvent {
                        session_id,
                        message_id,
                        message,
                    },
                );
            }
            BusEvent::ChatContextNotice {
                session_id,
                kind,
                message,
                usage_ratio,
                folded_messages,
            } => {
                let _ = self.app.emit(
                    "chat-context-notice",
                    ChatContextNoticeEvent {
                        session_id,
                        kind,
                        message,
                        usage_ratio,
                        folded_messages,
                    },
                );
            }
            BusEvent::AskUser {
                session_id,
                request_id,
                questions,
            } => {
                let _ = self.app.emit(
                    "ask-user",
                    AskUserEvent {
                        session_id,
                        request_id,
                        questions: questions
                            .into_iter()
                            .map(|question| crate::models::chat::AskUserQuestion {
                                header: question.header,
                                question: question.question,
                                options: question
                                    .options
                                    .into_iter()
                                    .map(|option| crate::models::chat::AskUserOption {
                                        label: option.label,
                                        description: option.description,
                                    })
                                    .collect(),
                                multi_select: question.multi_select,
                            })
                            .collect(),
                    },
                );
            }
            BusEvent::FileOffer {
                session_id,
                offer_id,
                path,
                absolute_path,
                name,
                mime,
                size,
                workspace_id,
            } => {
                let _ = self.app.emit(
                    "file-offer",
                    serde_json::json!({
                        "sessionId": session_id,
                        "offerId": offer_id,
                        "path": path,
                        "absolutePath": absolute_path,
                        "name": name,
                        "mime": mime,
                        "size": size,
                        "workspaceId": workspace_id,
                    }),
                );
            }
            BusEvent::UrlOffer {
                session_id,
                offer_id,
                label,
                origin_url,
                public_url,
            } => {
                let _ = self.app.emit(
                    "url-offer",
                    serde_json::json!({
                        "sessionId": session_id,
                        "offerId": offer_id,
                        "label": label,
                        "originUrl": origin_url,
                        "publicUrl": public_url,
                    }),
                );
            }
            BusEvent::PathPermissionRequest {
                session_id,
                request_id,
                path,
                operation,
                tool_name,
            } => {
                let _ = self.app.emit(
                    "path-permission",
                    PathPermissionEvent {
                        session_id,
                        request_id,
                        path,
                        operation,
                        tool_name,
                    },
                );
            }
            BusEvent::ToolApprovalRequest {
                session_id,
                request_id,
                tool_name,
                title,
                arguments,
                preview,
            } => {
                let _ = self.app.emit(
                    "tool-approval",
                    serde_json::json!({
                        "sessionId": session_id,
                        "requestId": request_id,
                        "toolName": tool_name,
                        "title": title,
                        "arguments": arguments,
                        "preview": preview,
                    }),
                );
            }
            BusEvent::PlanModeChanged {
                session_id,
                active,
                source,
            } => {
                let _ = self.app.emit(
                    "plan-mode-changed",
                    serde_json::json!({
                        "sessionId": session_id,
                        "active": active,
                        "source": source,
                    }),
                );
            }
            BusEvent::ToolStarted {
                session_id,
                subagent_id,
                parent_activity_id,
                message_id,
                activity_id,
                tool_name,
                title,
                kind,
                detail,
                arguments,
                preview,
            } => {
                let _ = self.app.emit(
                    "tool-started",
                    ToolActivityEvent {
                        session_id,
                        message_id,
                        activity_id,
                        subagent_id,
                        parent_activity_id,
                        tool_name,
                        title,
                        kind,
                        detail,
                        arguments,
                        result: None,
                        preview,
                        success: true,
                        status: "running".into(),
                    },
                );
            }
            BusEvent::ToolFinished {
                session_id,
                subagent_id,
                parent_activity_id,
                message_id,
                activity_id,
                tool_name,
                title,
                kind,
                detail,
                arguments,
                preview,
                result,
                success,
            } => {
                let _ = self.app.emit(
                    "tool-finished",
                    ToolActivityEvent {
                        session_id,
                        message_id,
                        activity_id,
                        subagent_id,
                        parent_activity_id,
                        tool_name,
                        title,
                        kind,
                        detail,
                        arguments,
                        result: Some(result),
                        preview,
                        success,
                        status: if success { "done" } else { "error" }.into(),
                    },
                );
            }
            BusEvent::TaskListUpdated { session_id, tasks } => {
                let _ = self.app.emit(
                    "task-list-updated",
                    TaskListUpdatedEvent { session_id, tasks },
                );
            }
            BusEvent::SlashCommand {
                session_id,
                command,
                args,
            } => {
                let _ = self.app.emit(
                    "slash-command",
                    serde_json::json!({
                        "sessionId": session_id,
                        "command": command,
                        "args": args,
                    }),
                );
            }
        }
    }
}
