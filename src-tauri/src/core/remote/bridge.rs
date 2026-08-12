//! Live session status fan-out for Companion clients.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::sync::broadcast;

use super::protocol::ServerMessage;
use crate::app_state::AppState;
use crate::core::event::BusEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum RemoteRunState {
    Idle,
    Streaming,
    WaitingApproval,
    WaitingAskUser,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionDto {
    pub id: String,
    pub title: String,
    pub updated_at_epoch_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    pub run_state: RemoteRunState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteWorkspaceDto {
    pub id: String,
    pub name: String,
    pub pinned: bool,
}

struct StatusStore {
    states: HashMap<String, RemoteRunState>,
}

static STATUS: OnceLock<Mutex<StatusStore>> = OnceLock::new();
static OUTBOUND: OnceLock<broadcast::Sender<String>> = OnceLock::new();

fn status_store() -> &'static Mutex<StatusStore> {
    STATUS.get_or_init(|| Mutex::new(StatusStore {
        states: HashMap::new(),
    }))
}

pub fn outbound_sender() -> broadcast::Sender<String> {
    OUTBOUND
        .get_or_init(|| {
            let (tx, _) = broadcast::channel(256);
            tx
        })
        .clone()
}

pub fn subscribe_outbound() -> broadcast::Receiver<String> {
    outbound_sender().subscribe()
}

pub fn broadcast_server_message(message: &ServerMessage) {
    if let Ok(text) = serde_json::to_string(message) {
        let _ = outbound_sender().send(text);
    }
}

/// Notify companion clients that an ask / approval / permission was resolved
/// (including when the desktop UI answered it).
pub fn push_interaction_resolved(request_id: &str, kind: &str) {
    broadcast_server_message(&ServerMessage::Event {
        name: "interaction.resolved".into(),
        data: json!({
            "requestId": request_id,
            "kind": kind,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    });
}

pub fn set_run_state(session_id: &str, state: RemoteRunState) {
    if let Ok(mut guard) = status_store().lock() {
        if state == RemoteRunState::Idle {
            guard.states.remove(session_id);
        } else {
            guard.states.insert(session_id.to_string(), state);
        }
    }
    let run_state = serde_json::to_value(state).unwrap_or(Value::String("Idle".into()));
    broadcast_server_message(&ServerMessage::Event {
        name: "session.status".into(),
        data: json!({
            "sessionId": session_id,
            "runState": run_state,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    });
}

pub fn run_state_for(session_id: &str) -> RemoteRunState {
    status_store()
        .lock()
        .ok()
        .and_then(|g| g.states.get(session_id).copied())
        .unwrap_or(RemoteRunState::Idle)
}

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
            set_run_state(session_id, RemoteRunState::Idle);
            broadcast_server_message(&ServerMessage::Event {
                name: "chat.finished".into(),
                data: json!({
                    "sessionId": session_id,
                    "messageId": message_id,
                    "content": content,
                    "reasoning": reasoning,
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
        BusEvent::PathPermissionRequest {
            session_id,
            request_id,
            path,
            tool_name,
            ..
        } => {
            set_run_state(session_id, RemoteRunState::WaitingAskUser);
            broadcast_server_message(&ServerMessage::Event {
                name: "ask-user".into(),
                data: json!({
                    "sessionId": session_id,
                    "requestId": request_id,
                    "title": format!("路径权限 · {tool_name}"),
                    "preview": path,
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
            set_run_state(session_id, RemoteRunState::Streaming);
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

pub fn build_session_snapshot(app: &AppHandle) -> Value {
    let Some(state) = app.try_state::<AppState>() else {
        return json!({ "sessions": [], "workspaces": [] });
    };
    let workspaces = state.core.workspaces().list();
    let workspace_name = |id: &str| {
        workspaces
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.name.clone())
    };
    let sessions: Vec<RemoteSessionDto> = state
        .core
        .chat()
        .list_sessions()
        .into_iter()
        .map(|session| {
            let run_state = run_state_for(&session.session_id);
            RemoteSessionDto {
                id: session.session_id.clone(),
                title: session.preview.clone(),
                updated_at_epoch_ms: session.updated_at,
                workspace_id: session.workspace_id.clone(),
                workspace_name: session
                    .workspace_id
                    .as_deref()
                    .and_then(workspace_name),
                run_state,
            }
        })
        .collect();
    let workspace_dtos: Vec<RemoteWorkspaceDto> = workspaces
        .into_iter()
        .map(|w| RemoteWorkspaceDto {
            id: w.id,
            name: w.name,
            pinned: w.pinned,
        })
        .collect();
    json!({ "sessions": sessions, "workspaces": workspace_dtos })
}
