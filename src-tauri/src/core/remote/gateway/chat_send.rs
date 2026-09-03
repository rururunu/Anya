use serde_json::json;
use tauri::AppHandle;
use tauri::Manager;

use crate::app_state::AppState;

use super::compose::{parse_approval_mode, parse_chat_mode};
use super::outbound::Outbound;
use super::send::send_msg;
use crate::core::remote::protocol::ServerMessage;

pub(super) async fn handle_chat_send(
    app: &AppHandle,
    ws: &Outbound,
    request_id: String,
    session_id: Option<String>,
    message: String,
    workspace_id: Option<String>,
    chat_mode: Option<String>,
    tool_approval_mode: Option<String>,
    chat_model: Option<String>,
    chat_model_provider: Option<String>,
    image_gen: Option<crate::core::remote::protocol::RemoteImageGenOptions>,
) -> Result<(), String> {
    let Some(state) = app.try_state::<AppState>() else {
        return send_msg(ws, &ServerMessage::rpc_err(request_id, "app not ready")).await;
    };
    let settings = match crate::services::settings_store::get_settings(app) {
        Ok(settings) => settings,
        Err(error) => {
            return send_msg(ws, &ServerMessage::rpc_err(request_id, error)).await;
        }
    };
    // Phone toolbar → the same ImageGenSendOptions the desktop composer builds. A custom
    // style id resolves to the desktop template (prompt + example image) here, since the
    // phone never sees the example image bytes.
    let image_gen = image_gen.map(|remote| {
        let template = remote.style_id.as_deref().and_then(|id| {
            settings
                .image_style_templates
                .iter()
                .find(|template| template.id == id)
        });
        crate::models::chat::ImageGenSendOptions {
            size: remote.size,
            quality: remote.quality,
            n: remote.n,
            style_prompt: template
                .map(|t| t.prompt.clone())
                .filter(|p| !p.trim().is_empty())
                .or(remote.style_prompt),
            example_image: template.and_then(|t| t.example_image.clone()),
        }
    });
    let preferences = crate::core::chat::SendPreferences::from(&settings);
    let session_id = match session_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(id) => Some(id),
        None => Some(format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        )),
    };
    let sid = session_id.clone().unwrap_or_default();
    if !sid.is_empty() {
        let patch = crate::core::remote::compose::SessionComposePatch {
            chat_mode: parse_chat_mode(chat_mode.as_deref()),
            tool_approval_mode: parse_approval_mode(tool_approval_mode.as_deref()),
            chat_model: chat_model.clone(),
            chat_model_provider: chat_model_provider.clone(),
            chat_model_label: None,
            reasoning_effort: None,
        };
        if patch.chat_mode.is_some()
            || patch.tool_approval_mode.is_some()
            || patch.chat_model.is_some()
            || patch.chat_model_provider.is_some()
        {
            let _ = crate::core::remote::compose::patch(&sid, &patch);
        }
    }
    let compose = if sid.is_empty() {
        crate::core::remote::compose::SessionCompose::default()
    } else {
        crate::core::remote::compose::get(&sid)
    };
    let overrides = crate::models::chat::ChatSendOverrides {
        model_id: if compose.chat_model.is_empty() {
            None
        } else {
            Some(compose.chat_model.clone())
        },
        model_provider: if compose.chat_model_provider.is_empty() {
            None
        } else {
            Some(compose.chat_model_provider.clone())
        },
        chat_mode: Some(compose.chat_mode),
        tool_approval_mode: Some(compose.tool_approval_mode),
        image_gen,
        skip_auto_plan: false,
        resume_plan: false,
    };
    let quick_ask = false;
    match state
        .core
        .chat()
        .send(
            session_id,
            message,
            preferences,
            workspace_id,
            quick_ask,
            overrides,
            crate::core::chat::session_origin::RequestOrigin::Companion,
        )
        .await
    {
        Ok(result) => {
            send_msg(
                ws,
                &ServerMessage::rpc_ok(
                    request_id,
                    json!({
                        "sessionId": result.session_id,
                        "userMessageId": result.user_message_id,
                        "assistantMessageId": result.assistant_message_id,
                        "agentRunId": result.agent_run_id,
                    }),
                ),
            )
            .await
        }
        Err(error) => send_msg(ws, &ServerMessage::rpc_err(request_id, error.to_string())).await,
    }
}
