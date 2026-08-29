use serde_json::json;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;

use crate::app_state::AppState;

use super::chat_send::handle_chat_send;
use super::compose::{apply_remote_reasoning_effort, parse_approval_mode, parse_chat_mode, resolve_session_compose};
use super::outbound::Outbound;
use super::payloads::{
    list_mcp_payload, list_remote_models_payload, list_skills_payload, session_history,
};
use crate::core::remote::protocol::{ClientMessage, ServerMessage};
use super::send::send_msg;
use super::workspace_files::{
    begin_file_download, list_workspace_files_payload, read_workspace_file_payload,
    workspace_snapshot_payload,
};

pub(super) async fn handle_binary_chunk(payload: Vec<u8>, out: Outbound) {
    const HEADER_LEN: usize = 36 + 36 + 8;
    if payload.len() < HEADER_LEN {
        tracing::debug!(len = payload.len(), "binary chunk frame too short");
        return;
    }
    let request_id = match std::str::from_utf8(&payload[..36]) {
        Ok(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let upload_id = match std::str::from_utf8(&payload[36..72]) {
        Ok(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let offset = u64::from_be_bytes(payload[72..80].try_into().unwrap_or([0u8; 8]));
    let data = payload[HEADER_LEN..].to_vec();

    let (request_id, upload_id) = (request_id, upload_id);
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::core::remote::upload::chunk_bytes(&upload_id, offset, &data)
    })
    .await;

    match result {
        Ok(Ok(chunk_result)) => {
            if out
                .send(&ServerMessage::rpc_ok(request_id, chunk_result))
                .await
                .is_err()
            {
                tracing::debug!("failed to ack binary chunk");
            }
        }
        Ok(Err(message)) => {
            let _ = out
                .send(&ServerMessage::rpc_err(request_id, &message))
                .await;
        }
        Err(error) => {
            let _ = out
                .send(&ServerMessage::rpc_err(
                    request_id,
                    &format!("chunk task failed: {error}"),
                ))
                .await;
        }
    }
}

pub(super) async fn handle_text(app: &AppHandle, ws: &Outbound, text: &str) -> Result<(), String> {
    let parsed: ClientMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(error) => {
            tracing::debug!(error = %error, "drop undecodable remote frame");
            return Ok(());
        }
    };

    match parsed {
        ClientMessage::Hello { .. } => {
            Ok(())
        }
        ClientMessage::Pong { .. } => Ok(()),
        ClientMessage::SessionList { request_id } => {
            let sessions = crate::core::remote::bridge::build_session_snapshot(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, sessions)).await
        }
        ClientMessage::SessionHistory {
            request_id,
            session_id,
        } => {
            let data = session_history(app, &session_id);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, data)).await
        }
        ClientMessage::SessionDelete {
            request_id,
            session_id,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            let bound = state
                .core
                .chat()
                .conversation()
                .workspace_for_session(&session_id);
            let workspace_root = bound.and_then(|id| {
                state
                    .core
                    .workspaces()
                    .list()
                    .into_iter()
                    .find(|w| w.id == id)
                    .map(|w| w.root)
            });
            crate::core::remote::upload::cleanup_session_uploads(app, &session_id, workspace_root.as_deref());
            state.core.chat().conversation().delete_session(&session_id);
            let _ = app.emit("history-updated", json!({ "sessionId": session_id }));
            let snapshot = crate::core::remote::bridge::build_session_snapshot(app);
            crate::core::remote::bridge::broadcast_server_message(&ServerMessage::Event {
                name: "session.snapshot".into(),
                data: snapshot.as_object().cloned().unwrap_or_default(),
            });
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true, "sessionId": session_id })),
            )
            .await
        }
        ClientMessage::WorkspaceSnapshot {
            request_id,
            session_id,
        } => {
            let snapshot = workspace_snapshot_payload(app, session_id.as_deref());
            send_msg(ws, &ServerMessage::rpc_ok(request_id, snapshot)).await
        }
        ClientMessage::WorkspaceReadFile {
            request_id,
            path,
            max_bytes,
            session_id,
            workspace_id,
            mode,
            offset,
            length,
        } => {
            let result = read_workspace_file_payload(
                app,
                session_id.as_deref(),
                workspace_id.as_deref(),
                &path,
                max_bytes,
                mode.as_deref().unwrap_or("text"),
                offset,
                length,
            )
            .await;
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::WorkspaceFiles {
            request_id,
            session_id,
            workspace_id,
        } => {
            let payload =
                list_workspace_files_payload(app, session_id.as_deref(), workspace_id.as_deref())
                    .await;
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::SkillsList { request_id } => {
            let payload = list_skills_payload(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::McpList { request_id } => {
            let payload = list_mcp_payload(app);
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::FileUploadBegin {
            request_id,
            session_id,
            workspace_id,
            file_name,
            size,
            mime: _,
        } => {
            let result = crate::core::remote::upload::begin(
                app,
                session_id.as_deref(),
                workspace_id.as_deref(),
                &file_name,
                size,
            );
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::FileUploadChunk {
            request_id,
            upload_id,
            offset,
            data_base64,
        } => match crate::core::remote::upload::chunk(&upload_id, offset, &data_base64) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileUploadFinish {
            request_id,
            upload_id,
        } => match crate::core::remote::upload::finish(&upload_id) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileUploadAbort {
            request_id,
            upload_id,
        } => match crate::core::remote::upload::abort(&upload_id) {
            Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
            Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
        },
        ClientMessage::FileDownloadBegin {
            request_id,
            path,
            session_id,
            workspace_id,
        } => {
            let result =
                begin_file_download(app, session_id.as_deref(), workspace_id.as_deref(), &path);
            match result {
                Ok(payload) => send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await,
                Err(message) => send_msg(ws, &ServerMessage::rpc_err(request_id, &message)).await,
            }
        }
        ClientMessage::ChatSend {
            request_id,
            session_id,
            message,
            workspace_id,
            chat_mode,
            tool_approval_mode,
            chat_model,
            chat_model_provider,
        } => {
            handle_chat_send(
                app,
                ws,
                request_id,
                session_id,
                message,
                workspace_id,
                chat_mode,
                tool_approval_mode,
                chat_model,
                chat_model_provider,
            )
            .await
        }
        ClientMessage::SessionComposeGet {
            request_id,
            session_id,
        } => {
            let compose = resolve_session_compose(&app, &session_id).await;
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({ "sessionId": session_id, "compose": compose }),
                ),
            )
            .await
        }
        ClientMessage::SessionComposeSet {
            request_id,
            session_id,
            chat_mode,
            tool_approval_mode,
            chat_model,
            chat_model_provider,
            chat_model_label,
            reasoning_effort,
        } => {
            if let Some(effort) = reasoning_effort.as_deref() {
                apply_remote_reasoning_effort(&app, effort);
            }
            let patch = crate::core::remote::compose::SessionComposePatch {
                chat_mode: parse_chat_mode(chat_mode.as_deref()),
                tool_approval_mode: parse_approval_mode(tool_approval_mode.as_deref()),
                chat_model,
                chat_model_provider,
                chat_model_label,
                reasoning_effort,
            };
            let compose = crate::core::remote::compose::patch(&session_id, &patch);
            let _ = app.emit(
                "remote-compose-changed",
                json!({
                    "sessionId": session_id,
                    "compose": compose,
                    "source": "companion",
                }),
            );
            crate::core::remote::bridge::broadcast_server_message(&ServerMessage::Event {
                name: "session.compose".into(),
                data: crate::core::remote::compose::event_payload(&session_id, &compose),
            });
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({ "sessionId": session_id, "compose": compose }),
                ),
            )
            .await
        }
        ClientMessage::ContextUsage {
            request_id,
            session_id,
            draft_message,
            model_id,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            match state
                .core
                .chat()
                .context_usage(&app, session_id, draft_message, None, model_id)
            {
                Ok(usage) => send_msg(ws, &ServerMessage::rpc_ok(request_id, json!(usage))).await,
                Err(error) => {
                    send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await
                }
            }
        }
        ClientMessage::ModelsList { request_id } => {
            let payload = list_remote_models_payload(app).await;
            send_msg(ws, &ServerMessage::rpc_ok(request_id, payload)).await
        }
        ClientMessage::PlanApprove {
            request_id,
            session_id,
        } => {
            let _ = app.emit("remote-plan-approve", json!({ "sessionId": session_id }));
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true, "sessionId": session_id })),
            )
            .await
        }
        ClientMessage::ChatCancel {
            request_id,
            message_id,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            match state.core.chat().cancel(&message_id) {
                Ok(()) => {
                    send_msg(
                        ws,
                        &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
                    )
                    .await
                }
                Err(error) => {
                    send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await
                }
            }
        }
        ClientMessage::ApprovalRespond {
            request_id,
            approval_request_id,
            decision,
        } => {
            let tool_session = crate::core::tools::tool_approval::shared_tool_approval_store()
                .complete(&approval_request_id, &decision);
            let path_decision = match decision.as_str() {
                "allow_session" => "allow_always",
                other => other,
            };
            let path_session = if tool_session.is_none() {
                let Some(state) = app.try_state::<AppState>() else {
                    return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready"))
                        .await;
                };
                state
                    .core
                    .chat()
                    .path_permission_store()
                    .complete(&approval_request_id, path_decision)
            } else {
                None
            };
            if let Some((kind, session_id)) = tool_session
                .map(|sid| ("tool_approval", sid))
                .or_else(|| path_session.map(|sid| ("path_permission", sid)))
            {
                crate::core::remote::bridge::push_interaction_resolved(
                    &approval_request_id,
                    kind,
                    Some(&session_id),
                );
                crate::core::remote::bridge::resume_run_state_after_interaction(app, &session_id);
                crate::commands::window::dismiss_tracked_interaction_notifications(
                    app,
                    Some(&approval_request_id),
                    None,
                );
                let _ = app.emit(
                    "interaction-resolved",
                    json!({
                        "requestId": approval_request_id,
                        "kind": kind,
                    }),
                );
                send_msg(
                    ws,
                    &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
                )
                .await
            } else {
                send_msg(
                    ws,
                    &ServerMessage::rpc_err(
                        request_id,
                        "approval request not found or already completed",
                    ),
                )
                .await
            }
        }
        ClientMessage::AskRespond {
            request_id,
            ask_request_id,
            answer,
        } => {
            let Some(state) = app.try_state::<AppState>() else {
                return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
            };
            let Some(session_id) = state
                .core
                .chat()
                .ask_store()
                .complete(&ask_request_id, answer)
            else {
                return send_msg(
                    ws,
                    &ServerMessage::rpc_err(
                        request_id,
                        "ask request not found or already completed",
                    ),
                )
                .await;
            };
            crate::core::remote::bridge::push_interaction_resolved(
                &ask_request_id,
                "ask_user",
                Some(&session_id),
            );
            crate::core::remote::bridge::resume_run_state_after_interaction(app, &session_id);
            crate::commands::window::dismiss_tracked_interaction_notifications(
                app,
                Some(&ask_request_id),
                None,
            );
            let _ = app.emit(
                "interaction-resolved",
                json!({
                    "requestId": ask_request_id,
                    "kind": "ask_user",
                }),
            );
            send_msg(
                ws,
                &ServerMessage::rpc_ok(request_id, json!({ "ok": true })),
            )
            .await
        }
    }
}
