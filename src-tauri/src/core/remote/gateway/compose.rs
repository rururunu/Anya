use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;

/// Resolves session compose state, waiting briefly for desktop Pinia sync when needed.
pub(super) async fn resolve_session_compose(
    app: &AppHandle,
    session_id: &str,
) -> crate::core::remote::compose::SessionCompose {
    let _ = app.emit("remote-compose-needed", json!({ "sessionId": session_id }));
    tokio::time::sleep(Duration::from_millis(80)).await;
    let mut compose = crate::core::remote::compose::get(session_id);
    if compose.chat_model.trim().is_empty() {
        for _ in 0..16 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            compose = crate::core::remote::compose::get(session_id);
            if !compose.chat_model.trim().is_empty() {
                break;
            }
        }
    }
    if compose.chat_model.trim().is_empty() || compose.reasoning_effort.is_none() {
        if let Some(state) = app.try_state::<crate::services::settings_store::SettingsState>() {
            if let Ok(settings) = state.settings.lock() {
                if compose.chat_model.trim().is_empty() {
                    compose.chat_model = settings.chat_model.clone();
                    compose.chat_model_provider = settings.chat_model_provider.clone();
                    compose.chat_mode = settings.chat_mode;
                    compose.tool_approval_mode = settings.tool_approval_mode;
                }
                if compose.reasoning_effort.is_none() {
                    compose.reasoning_effort = Some(
                        serde_json::to_value(settings.reasoning_effort)
                            .ok()
                            .and_then(|value| value.as_str().map(str::to_string))
                            .unwrap_or_else(|| "disabled".into()),
                    );
                }
            }
        }
        if !compose.chat_model.trim().is_empty() {
            crate::core::remote::compose::set(session_id, compose.clone());
        }
    }
    compose
}

/// Parses a companion chat mode string into the desktop enum.
pub(super) fn parse_chat_mode(raw: Option<&str>) -> Option<crate::models::settings::ChatMode> {
    match raw? {
        "ask" => Some(crate::models::settings::ChatMode::Ask),
        "agent" => Some(crate::models::settings::ChatMode::Agent),
        "plan" => Some(crate::models::settings::ChatMode::Plan),
        "image" => Some(crate::models::settings::ChatMode::Image),
        _ => None,
    }
}

/// Parses a companion tool-approval mode string into the desktop enum.
pub(super) fn parse_approval_mode(
    raw: Option<&str>,
) -> Option<crate::models::settings::ToolApprovalMode> {
    match raw? {
        "ask" => Some(crate::models::settings::ToolApprovalMode::Ask),
        "auto" => Some(crate::models::settings::ToolApprovalMode::Auto),
        "alwaysAllow" => Some(crate::models::settings::ToolApprovalMode::AlwaysAllow),
        _ => None,
    }
}

/// Applies reasoning-effort changes sent from the companion to persisted settings.
pub(super) fn apply_remote_reasoning_effort(app: &AppHandle, effort: &str) {
    let trimmed = effort.trim();
    if trimmed.is_empty() {
        return;
    }
    let Ok(parsed) = serde_json::from_value::<crate::models::settings::ReasoningEffort>(json!(
        trimmed.to_ascii_lowercase()
    )) else {
        return;
    };
    let Ok(mut settings) = crate::services::settings_store::get_settings(app) else {
        return;
    };
    if settings.reasoning_effort == parsed {
        return;
    }
    settings.reasoning_effort = parsed;
    let _ = crate::services::settings_store::set_settings(app, settings);
}
