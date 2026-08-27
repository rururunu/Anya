use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::core::agent::AgentDebugEvent;
use crate::core::ai::deepseek;
use crate::core::chat::session_origin::RequestOrigin;
use crate::core::chat::SendPreferences;
use crate::models::chat::{
    ChatCancelRequest, ChatHistoryRequest, ChatHistoryResponse, ChatModelInfo, ChatSendOverrides,
    ChatSendRequest, ChatSendResponse, ContextUsageRequest, ContextUsageResponse,
    ListChatSessionsResponse,
};
use crate::services::gemini_oauth;
use crate::services::settings_store::{apply_chat_request_settings, get_settings};

#[tauri::command]
pub async fn chat(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChatSendRequest,
) -> Result<ChatSendResponse, String> {
    let settings = get_settings(&app)?;
    let preferences = SendPreferences::from(&settings);
    // `reqwest::blocking::Client` owns a Tokio runtime. Creating/dropping it on a
    // tokio worker panics — keep configure off the async path.
    let settings_for_cfg = settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        apply_chat_request_settings(&settings_for_cfg);
    })
    .await
    .map_err(|error| format!("configure runtimes failed: {error}"))?;

    let overrides = ChatSendOverrides::from_request(&request);
    let result = state
        .core
        .chat()
        .send(
            request.session_id,
            request.message,
            preferences,
            request.workspace_id,
            request.quick_ask,
            overrides,
            RequestOrigin::Desktop,
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(ChatSendResponse {
        session_id: result.session_id,
        user_message_id: result.user_message_id,
        assistant_message_id: result.assistant_message_id,
        agent_run_id: result.agent_run_id,
    })
}

#[tauri::command]
pub fn chat_cancel(state: State<'_, AppState>, request: ChatCancelRequest) -> Result<(), String> {
    state
        .core
        .chat()
        .cancel(&request.message_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn agent_debug_snapshot(state: State<'_, AppState>) -> Result<Vec<AgentDebugEvent>, String> {
    Ok(state.core.chat().agent_debug_snapshot())
}

#[tauri::command]
pub async fn chat_history(
    state: State<'_, AppState>,
    request: ChatHistoryRequest,
) -> Result<ChatHistoryResponse, String> {
    let session_id = request
        .session_id
        .unwrap_or_else(|| crate::core::runtime::DEFAULT_SESSION_ID.to_string());

    let messages = state
        .core
        .chat()
        .history(&session_id)
        .map_err(|error| error.to_string())?;
    let last_cache_usage = crate::core::chat::db::load_session_cache_usage(
        &state.core.chat().conversation().db_pool(),
        &session_id,
    )
    .await
    .unwrap_or(None);
    let message_cache_usages = crate::core::chat::db::load_message_cache_usages(
        &state.core.chat().conversation().db_pool(),
        &session_id,
    )
    .await
    .unwrap_or_default();
    let message_completed_at = crate::core::chat::db::load_message_completed_at(
        &state.core.chat().conversation().db_pool(),
        &session_id,
    )
    .await
    .unwrap_or_default();

    Ok(ChatHistoryResponse {
        session_id,
        messages,
        last_cache_usage,
        message_cache_usages,
        message_completed_at,
    })
}

#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<ListChatSessionsResponse, String> {
    let sessions = state.core.chat().list_sessions();
    Ok(ListChatSessionsResponse { sessions })
}

#[tauri::command]
pub fn list_archived_chat_sessions(
    state: State<'_, AppState>,
) -> Result<ListChatSessionsResponse, String> {
    let sessions = state.core.chat().list_archived_sessions();
    Ok(ListChatSessionsResponse { sessions })
}

#[tauri::command]
pub async fn set_chat_session_archived(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    archived: bool,
) -> Result<(), String> {
    if session_id.trim().is_empty() {
        return Err("Session id is required".into());
    }
    let workspace_id = state
        .core
        .chat()
        .conversation()
        .workspace_for_session(&session_id);
    state
        .core
        .chat()
        .set_session_archived(&session_id, archived);
    if !archived {
        if let Some(workspace_id) = workspace_id {
            if state
                .core
                .workspaces()
                .list_archived()
                .iter()
                .any(|workspace| workspace.id == workspace_id)
            {
                let manager = state.core.workspaces();
                manager.set_archived(&workspace_id, false).await?;
                app.emit("workspaces-changed", manager.current())
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_chat_session_workspace(
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
) -> Result<(), String> {
    if session_id.trim().is_empty() {
        return Err("Session id is required".into());
    }
    if workspace_id.trim().is_empty() {
        return Err("Workspace id is required".into());
    }
    let exists = state
        .core
        .workspaces()
        .list()
        .iter()
        .any(|workspace| workspace.id == workspace_id);
    if !exists {
        return Err("Workspace not found".into());
    }
    state
        .core
        .chat()
        .conversation()
        .rebind_workspace(&session_id, &workspace_id)
        .await
}

#[tauri::command]
pub async fn list_chat_models(app: AppHandle) -> Result<Vec<ChatModelInfo>, String> {
    let settings = get_settings(&app)?;
    let mut all_models: Vec<ChatModelInfo> = Vec::new();

    if !settings.deepseek_api_key.trim().is_empty() {
        match deepseek::list_models(&settings.deepseek_api_key).await {
            Ok(models) => all_models.extend(models),
            Err(e) => {
                // Partial failure — log but don't abort if custom provider has models.
                eprintln!("DeepSeek list_models error: {e}");
            }
        }
    }

    if settings.gemini_oauth.is_logged_in() {
        match gemini_oauth::list_models(&app).await {
            Ok(models) => all_models.extend(models),
            Err(error) => {
                eprintln!("Gemini fetchAvailableModels error: {error}");
            }
        }
    }

    for custom in &settings.custom_providers {
        let base = custom.base_url.trim();
        let key = custom.api_key.trim();
        if base.is_empty() {
            continue;
        }

        let is_disabled =
            |id: &str| crate::core::ai::registry::provider_model_is_disabled(custom, id);

        let mut remote_ok = false;
        if !key.is_empty() {
            let models_url = deepseek::normalize_models_url(base);
            match deepseek::list_openai_compatible_models(
                &models_url,
                key,
                &custom.id,
                Some(&custom.name),
            )
            .await
            {
                Ok(models) if !models.is_empty() => {
                    all_models.extend(models.into_iter().filter(|m| !is_disabled(&m.id)));
                    remote_ok = true;
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("custom provider {} list_models error: {error}", custom.name);
                }
            }
        }

        // Keep manually configured models when remote listing is unavailable.
        if !remote_ok && !custom.models.trim().is_empty() {
            let custom_models: Vec<ChatModelInfo> = custom
                .models
                .split([',', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty() && !is_disabled(s))
                .map(|id| ChatModelInfo {
                    id: id.to_string(),
                    owned_by: custom.name.clone(),
                    provider: custom.id.clone(),
                    display_name: None,
                    thinking_variants: None,
                    reasoning: None,
                })
                .collect();
            all_models.extend(custom_models);
        }
    }

    if all_models.is_empty() && !settings.deepseek_api_key.trim().is_empty() {
        // Re-run DeepSeek to surface its error properly.
        return deepseek::list_models(&settings.deepseek_api_key)
            .await
            .map_err(|e| e.to_string());
    }

    Ok(all_models)
}

#[tauri::command]
pub async fn list_custom_provider_models(
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    let base = base_url.trim();
    let key = api_key.trim();
    if base.is_empty() {
        return Err("Base URL is required".into());
    }
    if key.is_empty() {
        return Err("API Key is required".into());
    }
    let models_url = deepseek::normalize_models_url(base);
    let models = deepseek::list_openai_compatible_models(&models_url, key, "custom", None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(models.into_iter().map(|model| model.id).collect())
}

#[tauri::command]
pub fn delete_chat_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
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
    crate::core::remote::cleanup_session_uploads(&app, &session_id, workspace_root.as_deref());
    state.core.chat().conversation().delete_session(&session_id);
    Ok(())
}

#[tauri::command]
pub fn branch_chat_session(
    state: State<'_, AppState>,
    session_id: String,
    message_id: Option<String>,
) -> Result<crate::models::chat::ChatSessionSummary, String> {
    if session_id.trim().is_empty() {
        return Err("Session id is required".into());
    }
    let until = message_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    state.core.chat().branch_session(&session_id, until)
}

#[tauri::command]
pub fn get_context_usage(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ContextUsageRequest,
) -> Result<ContextUsageResponse, String> {
    state
        .core
        .chat()
        .context_usage(
            &app,
            request.session_id,
            request.draft_message,
            request.context,
            request.model_id,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_environment_context(state: State<'_, AppState>) -> crate::core::runtime::RequestContext {
    crate::core::context::store::wait_for_completed_capture();
    let context = environment_context_from_source(state.core.chat());
    tracing::debug!(
        active_window = ?context.active_window,
        active_file = ?context.active_file,
        workspace = ?context.workspace,
        git = ?context.git_status,
        ide = ?context.ide_context.as_ref().map(|ide| ide.ide.as_str()),
        "get_environment_context IPC RequestContext before serialization"
    );
    context
}

trait EnvironmentContextSource {
    fn resolved_environment_context(&self) -> crate::core::runtime::RequestContext;
}

impl EnvironmentContextSource for crate::core::chat::ChatService {
    fn resolved_environment_context(&self) -> crate::core::runtime::RequestContext {
        self.environment_context()
    }
}

fn environment_context_from_source(
    source: &impl EnvironmentContextSource,
) -> crate::core::runtime::RequestContext {
    source.resolved_environment_context()
}

#[tauri::command]
pub fn clear_all_chat_sessions(state: State<'_, AppState>) -> Result<(), String> {
    state.core.chat().conversation().clear_all_sessions();
    Ok(())
}

#[cfg(test)]
mod environment_context_tests {
    use super::*;
    use crate::core::runtime::request::WorkspaceContext;
    use crate::core::runtime::RequestContext;

    struct ServiceContextStub(RequestContext);

    impl EnvironmentContextSource for ServiceContextStub {
        fn resolved_environment_context(&self) -> RequestContext {
            self.0.clone()
        }
    }

    #[test]
    fn command_and_chat_service_use_equivalent_workspace_resolution() {
        let service_context = RequestContext {
            active_file: Some(r"C:\code\Anya\src\main.rs".to_string()),
            workspace: Some(WorkspaceContext {
                name: "Anya".to_string(),
                root: r"C:\code\Anya".to_string(),
            }),
            git_status: Some("## main".to_string()),
            ..RequestContext::default()
        };
        let source = ServiceContextStub(service_context.clone());

        let command_context = environment_context_from_source(&source);

        assert_eq!(command_context.workspace, service_context.workspace);
        assert_eq!(command_context.active_file, service_context.active_file);
        assert_eq!(command_context.git_status, service_context.git_status);
    }

    #[test]
    fn environment_context_ipc_is_allowed_by_chat_permission() {
        let permission = include_str!("../../permissions/chat.toml");
        assert!(permission.contains("\"get_environment_context\""));
    }
}
