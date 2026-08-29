use serde_json::json;
use tauri::AppHandle;
use tauri::Manager;

use crate::app_state::AppState;

use super::outbound::Outbound;
use super::send::send_msg;
use crate::core::remote::protocol::ServerMessage;

fn ask_title(questions: &[crate::core::tools::context::AskQuestion]) -> String {
    questions
        .first()
        .map(|q| {
            if q.header.trim().is_empty() {
                q.question.clone()
            } else {
                q.header.clone()
            }
        })
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "需要回答".into())
}

pub(super) fn build_interaction_snapshot(app: &AppHandle) -> serde_json::Value {
    let mut pending = Vec::new();
    if let Some(state) = app.try_state::<AppState>() {
        for item in state.core.chat().ask_store().pending_items() {
            pending.push(json!({
                "kind": "ask_user",
                "sessionId": item.session_id,
                "requestId": item.request_id,
                "title": ask_title(&item.questions),
                "questions": item.questions,
            }));
        }
        for item in state.core.chat().path_permission_store().pending_items() {
            pending.push(json!({
                "kind": "path_permission",
                "sessionId": item.session_id,
                "requestId": item.request_id,
                "toolName": item.tool_name,
                "title": format!("路径权限 · {}", item.tool_name),
                "preview": item.path,
                "operation": item.operation,
            }));
        }
    }
    for item in crate::core::tools::tool_approval::shared_tool_approval_store().pending_items() {
        pending.push(json!({
            "kind": "tool_approval",
            "sessionId": item.session_id,
            "requestId": item.request_id,
            "toolName": item.tool_name,
            "title": item.title,
        }));
    }
    for session_id in crate::core::tools::plan_mode::shared_plan_mode_store().active_session_ids() {
        if crate::core::remote::bridge::run_state_for(&session_id) == crate::core::remote::bridge::RemoteRunState::Streaming {
            continue;
        }
        if !crate::core::tools::plan_mode::shared_plan_mode_store()
            .is_awaiting_approval(&session_id)
        {
            continue;
        }
        pending.push(json!({
            "kind": "plan_approval",
            "sessionId": session_id,
            "requestId": format!("plan-{session_id}"),
            "title": "计划待批准",
        }));
    }
    json!({ "pending": pending })
}

pub(super) async fn replay_pending_interactions(app: &AppHandle, ws: &Outbound) -> Result<(), String> {
    let Some(state) = app.try_state::<crate::app_state::AppState>() else {
        return Ok(());
    };

    for pending in state.core.chat().ask_store().pending_items() {
        let title = pending
            .questions
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

        send_msg(
            ws,
            &ServerMessage::Event {
                name: "ask-user".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "title": title,
                    "questions": pending.questions,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    for pending in crate::core::tools::tool_approval::shared_tool_approval_store().pending_items() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "tool-approval".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "toolName": pending.tool_name,
                    "title": pending.title,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    for pending in state.core.chat().path_permission_store().pending_items() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "path.permission".into(),
                data: json!({
                    "sessionId": pending.session_id,
                    "requestId": pending.request_id,
                    "toolName": pending.tool_name,
                    "title": format!("路径权限 · {}", pending.tool_name),
                    "preview": pending.path,
                    "operation": pending.operation,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    for session_id in crate::core::tools::plan_mode::shared_plan_mode_store().active_session_ids() {
        send_msg(
            ws,
            &ServerMessage::Event {
                name: "session.planMode".into(),
                data: json!({
                    "sessionId": session_id,
                    "active": true,
                    "source": "manual",
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            },
        )
        .await?;
    }

    Ok(())
}
