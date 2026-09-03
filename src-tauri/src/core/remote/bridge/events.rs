//! Fan-out of core bus events to Companion WebSocket clients.

use serde_json::json;

use super::{broadcast_server_message, run_state_for, set_run_state, RemoteRunState};
use crate::core::event::BusEvent;
use super::super::protocol::ServerMessage;

pub fn on_bus_event(event: &BusEvent) {
    match event {
        BusEvent::ChatStarted {
            session_id,
            user_message,
            assistant_message,
            ..
        } => {
            set_run_state(session_id, RemoteRunState::Streaming);
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.started".into(),
                data: json!({
                    "sessionId": session_id,
                    "userMessage": {
                        "id": user_message.id,
                        "sessionId": user_message.session_id,
                        "role": "User",
                        "content": user_message.content,
                        "status": "Complete",
                        "createdAtEpochMs": user_message.timestamp,
                    },
                    "assistantMessage": {
                        "id": assistant_message.id,
                        "sessionId": assistant_message.session_id,
                        "role": "Assistant",
                        "content": assistant_message.content,
                        "status": "Streaming",
                        "createdAtEpochMs": assistant_message.timestamp,
                    },
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ChatDelta {
            session_id,
            message_id,
            delta,
        } => {
            set_run_state(session_id, RemoteRunState::Streaming);
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.delta".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "delta": delta,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ChatFinished {
            session_id,
            message_id,
            content,
            reasoning,
            ..
        } => {
            let waiting_plan = crate::core::tools::plan_mode::shared_plan_mode_store()
                .is_awaiting_approval(session_id)
                || crate::core::tools::plan_mode::content_requests_plan_approval(content);
            if waiting_plan {
                set_run_state(session_id, RemoteRunState::WaitingApproval);
            } else {
                set_run_state(session_id, RemoteRunState::Idle);
            }
            let completed_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.finished".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "content": content,
                    "reasoning": reasoning,
                    "completedAtEpochMs": completed_at,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        // Transient turn status ("compressing context", "stream retry 2/5", "analyzing
        // images"…) — the desktop shows these as an activity line under the bubble.
        BusEvent::ChatStatus {
            session_id,
            message_id,
            kind,
        } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.status".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "kind": kind,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        // Per-message prompt-cache hit rate (DeepSeek); the desktop shows "缓存命中 N%".
        BusEvent::TokenUsage {
            session_id: Some(session_id),
            message_id: Some(message_id),
            model,
            usage,
        } if usage.cache_read_tokens.is_some() => {
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.tokenUsage".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "model": model,
                    "inputTokens": usage.input_tokens,
                    "outputTokens": usage.output_tokens,
                    "cacheReadTokens": usage.cache_read_tokens.unwrap_or(0),
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        // The user bubble was rewritten after send (attachments / selection merged in).
        BusEvent::ChatUserContent {
            session_id,
            message_id,
            content,
        } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.userContent".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "content": content,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ChatReasoning {
            session_id,
            message_id,
            content,
        } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.reasoning".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "content": content,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::TaskListUpdated { session_id, tasks } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "session.tasks".into(),
                data: json!({
                    "sessionId": session_id,
                    "tasks": tasks,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::PlanModeChanged {
            session_id,
            active,
            source,
        } => {
            if !*active && run_state_for(session_id) == RemoteRunState::WaitingApproval {
                let still_blocked = crate::core::tools::tool_approval::shared_tool_approval_store()
                    .pending_items()
                    .iter()
                    .any(|item| item.session_id == *session_id);
                if !still_blocked {
                    set_run_state(session_id, RemoteRunState::Idle);
                }
            }
            broadcast_server_message(&ServerMessage::Event {
                name: "session.planMode".into(),
                data: json!({
                    "sessionId": session_id,
                    "active": active,
                    "source": source,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ChatError {
            session_id,
            message_id,
            message,
        } => {
            set_run_state(session_id, RemoteRunState::Error);
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.error".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "message": message,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ToolApprovalRequest {
            session_id,
            request_id,
            tool_name,
            title,
            ..
        } => {
            set_run_state(session_id, RemoteRunState::WaitingApproval);
            broadcast_server_message(&ServerMessage::Event {
                name: "tool-approval".into(),
                data: json!({
                    "sessionId": session_id,
                    "requestId": request_id,
                    "toolName": tool_name,
                    "title": title,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::AskUser {
            session_id,
            request_id,
            questions,
        } => {
            set_run_state(session_id, RemoteRunState::WaitingAskUser);
            let title = questions
                .first()
                .map(|q| {
                    if q.header.trim().is_empty() {
                        q.question.clone()
                    } else {
                        q.header.clone()
                    }
                })
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "需要回答".into());
            broadcast_server_message(&ServerMessage::Event {
                name: "ask-user".into(),
                data: json!({
                    "sessionId": session_id,
                    "requestId": request_id,
                    "title": title,
                    "questions": questions,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::FileOffer {
            session_id,
            offer_id,
            path,
            absolute_path: _,
            name,
            mime,
            size,
            workspace_id,
        } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "file.offer".into(),
                data: json!({
                    "sessionId": session_id,
                    "offerId": offer_id,
                    "path": path,
                    "name": name,
                    "mime": mime,
                    "size": size,
                    "workspaceId": workspace_id,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::UrlOffer {
            session_id,
            offer_id,
            label,
            origin_url,
            public_url,
        } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "url.offer".into(),
                data: json!({
                    "sessionId": session_id,
                    "offerId": offer_id,
                    "label": label,
                    "originUrl": origin_url,
                    "publicUrl": public_url,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::PathPermissionRequest {
            session_id,
            request_id,
            path,
            operation,
            tool_name,
        } => {
            // Same interaction surface as tool approval on Companion (allow once /
            // always / deny). Dedicated event so phone does not treat it as AskUser
            // with empty questions.
            set_run_state(session_id, RemoteRunState::WaitingApproval);
            broadcast_server_message(&ServerMessage::Event {
                name: "path.permission".into(),
                data: json!({
                    "sessionId": session_id,
                    "requestId": request_id,
                    "toolName": tool_name,
                    "title": format!("路径权限 · {tool_name}"),
                    "preview": path,
                    "operation": operation,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        BusEvent::ChatSessionTitleUpdated { session_id, title } => {
            broadcast_server_message(&ServerMessage::Event {
                name: "session.title".into(),
                data: json!({
                    "sessionId": session_id,
                    "title": title,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
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
            set_run_state(session_id, RemoteRunState::Streaming);
            broadcast_server_message(&ServerMessage::Event {
                name: "tool.started".into(),
                data: json!({
                    "sessionId": session_id,
                    "subagentId": subagent_id,
                    "parentActivityId": parent_activity_id,
                    "messageId": message_id,
                    "activityId": activity_id,
                    "toolName": tool_name,
                    "title": title,
                    "kind": kind,
                    "detail": detail,
                    "arguments": arguments,
                    "preview": preview,
                    "status": "running",
                    "success": true,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
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
            // A plan-gate rejection ends the turn; do not leave Companion on
            // Streaming ("回答中") until ChatFinished arrives.
            if !*success && result.contains(crate::core::tools::plan_mode::PLAN_GATE_BLOCKED) {
                set_run_state(session_id, RemoteRunState::WaitingApproval);
            } else {
                set_run_state(session_id, RemoteRunState::Streaming);
            }
            broadcast_server_message(&ServerMessage::Event {
                name: "tool.finished".into(),
                data: json!({
                    "sessionId": session_id,
                    "subagentId": subagent_id,
                    "parentActivityId": parent_activity_id,
                    "messageId": message_id,
                    "activityId": activity_id,
                    "toolName": tool_name,
                    "title": title,
                    "kind": kind,
                    "detail": detail,
                    "arguments": arguments,
                    "preview": preview,
                    "result": result,
                    "success": success,
                    "status": if *success { "done" } else { "error" },
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            });
        }
        _ => {}
    }
}
