use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::core::agent::AgentDebugEvent;
use crate::core::ai::deepseek;
use crate::core::chat::session_origin::RequestOrigin;
use crate::core::chat::SendPreferences;
use crate::core::runtime::ChatMessage;
use crate::models::chat::{
    ChatCancelRequest, ChatHistoryRequest, ChatHistoryResponse, ChatModelInfo, ChatSendOverrides,
    ChatSendRequest, ChatSendResponse, ContextUsageRequest, ContextUsageResponse,
    ListChatSessionsResponse,
};
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

/// Slice one page out of a transcript ordered by timestamp ascending.
///
/// `before_id` is resolved against the transcript first so equal timestamps
/// cannot split or duplicate a page; `before_timestamp` is only the fallback
/// for when that id is gone (rewound away, or a stale cursor from another
/// window). `has_more` reports whether older messages remain.
fn slice_history_page(
    transcript: &[ChatMessage],
    limit: Option<usize>,
    before_timestamp: Option<i64>,
    before_id: Option<&str>,
) -> (Vec<ChatMessage>, bool) {
    let total = transcript.len();
    let cursor_end = before_id
        .and_then(|id| transcript.iter().position(|message| message.id == id))
        .or_else(|| {
            before_timestamp.and_then(|timestamp| {
                transcript
                    .iter()
                    .position(|message| message.timestamp as i64 >= timestamp)
            })
        })
        .unwrap_or(total)
        .min(total);
    let window_start = match limit {
        Some(limit) if limit > 0 => cursor_end.saturating_sub(limit),
        _ => 0,
    };
    (
        transcript[window_start..cursor_end].to_vec(),
        window_start > 0,
    )
}

#[tauri::command]
pub async fn chat_history(
    state: State<'_, AppState>,
    request: ChatHistoryRequest,
) -> Result<ChatHistoryResponse, String> {
    let session_id = request
        .session_id
        .unwrap_or_else(|| crate::core::runtime::DEFAULT_SESSION_ID.to_string());

    // The transcript is held in memory in full and the model consumes every
    // message, so nothing is trimmed here. What grows without bound as a
    // conversation ages is the IPC payload, so the page is sliced at this
    // boundary instead.
    let transcript = state
        .core
        .chat()
        .history(&session_id)
        .map_err(|error| error.to_string())?;

    let (messages, has_more) = slice_history_page(
        &transcript,
        request.limit,
        request.before_timestamp,
        request.before_id.as_deref(),
    );
    let oldest_timestamp = messages
        .first()
        .map(|message| message.timestamp as i64);
    let oldest_id = messages.first().map(|message| message.id.clone());

    // Per-message side tables follow the same window, otherwise they would
    // restore the very payload growth this paging exists to avoid.
    let window_ids: std::collections::HashSet<String> = messages
        .iter()
        .map(|message| message.id.clone())
        .collect();

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
    .unwrap_or_default()
    .into_iter()
    .filter(|usage| window_ids.contains(&usage.message_id))
    .collect();
    let message_completed_at = crate::core::chat::db::load_message_completed_at(
        &state.core.chat().conversation().db_pool(),
        &session_id,
    )
    .await
    .unwrap_or_default()
    .into_iter()
    .filter(|(message_id, _)| window_ids.contains(message_id))
    .collect();
    let consumed_tokens = state.core.chat().conversation().consumed_tokens(&session_id);

    Ok(ChatHistoryResponse {
        session_id,
        messages,
        last_cache_usage,
        message_cache_usages,
        message_completed_at,
        consumed_tokens,
        has_more,
        oldest_timestamp,
        oldest_id,
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
    let mut deepseek_err = None;

    if !settings.deepseek_api_key.trim().is_empty() {
        let is_disabled =
            |id: &str| crate::core::ai::registry::deepseek_model_is_disabled(&settings, id);

        if !settings.deepseek_models.is_empty() {
            for entry in &settings.deepseek_models {
                if !entry.disabled {
                    all_models.push(ChatModelInfo {
                        id: entry.id.clone(),
                        owned_by: "deepseek".to_string(),
                        provider: "deepseek".to_string(),
                        display_name: None,
                        thinking_variants: None,
                        reasoning: None,
                    });
                }
            }
        } else {
            match deepseek::list_models(&settings.deepseek_api_key).await {
                Ok(models) => {
                    for m in models {
                        if !is_disabled(&m.id) {
                            all_models.push(m);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("DeepSeek list_models error: {e}");
                    deepseek_err = Some(e.to_string());
                    for id in ["deepseek-chat", "deepseek-reasoner"] {
                        if !is_disabled(id) {
                            all_models.push(ChatModelInfo {
                                id: id.to_string(),
                                owned_by: "deepseek".to_string(),
                                provider: "deepseek".to_string(),
                                display_name: None,
                                thinking_variants: None,
                                reasoning: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Provider discovery is network-bound: query every provider concurrently
    // instead of paying one round trip after another. `join_all` preserves the
    // configured order, so the picker still lists providers the same way.
    let discoveries = settings
        .custom_providers
        .iter()
        .map(discover_custom_provider_models);
    for models in futures_util::future::join_all(discoveries).await {
        all_models.extend(models);
    }

    if all_models.is_empty() && !settings.deepseek_api_key.trim().is_empty() {
        if let Some(err) = deepseek_err {
            return Err(err);
        }
    }

    Ok(all_models)
}

/// Discover one custom provider's model list: the provider's `/models` first,
/// falling back to the IDs configured by hand in Settings when it does not
/// answer (or has no key).
async fn discover_custom_provider_models(
    custom: &crate::models::settings::CustomProviderConfig,
) -> Vec<ChatModelInfo> {
    let base = custom.base_url.trim();
    if base.is_empty() {
        return Vec::new();
    }

    let is_disabled = |id: &str| crate::core::ai::registry::provider_model_is_disabled(custom, id);

    if !custom.api_key.trim().is_empty() {
        let models_url = deepseek::normalize_models_url(base);
        match deepseek::list_openai_compatible_models(
            &models_url,
            custom.api_key.trim(),
            &custom.id,
            Some(&custom.name),
        )
        .await
        {
            // The provider answered: trust its list even when every entry is
            // switched off, matching the previous sequential behaviour.
            Ok(models) if !models.is_empty() => {
                return models.into_iter().filter(|m| !is_disabled(&m.id)).collect();
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("custom provider {} list_models error: {error}", custom.name);
            }
        }
    }

    custom
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
        .collect()
}

#[tauri::command]
pub async fn list_deepseek_models(api_key: String) -> Result<Vec<String>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API Key is required".into());
    }
    let models = deepseek::list_models(key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(models.into_iter().map(|m| m.id).collect())
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
pub fn set_chat_session_title(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<String, String> {
    if session_id.trim().is_empty() {
        return Err("Session id is required".into());
    }
    state.core.chat().set_session_title(&session_id, &title)
}

#[tauri::command]
pub async fn regenerate_chat_session_title(
    state: State<'_, AppState>,
    session_id: String,
    model_id: Option<String>,
    model_provider: Option<String>,
) -> Result<String, String> {
    if session_id.trim().is_empty() {
        return Err("Session id is required".into());
    }
    state
        .core
        .chat()
        .regenerate_session_title(&session_id, model_id.as_deref(), model_provider.as_deref())
        .await
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
mod history_paging_tests {
    use super::*;
    use crate::core::runtime::{MessageStatus, Role};

    fn message(id: &str, timestamp: u64) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            role: Role::User,
            content: id.to_string(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp,
            estimated_tokens: None,
        }
    }

    fn transcript(count: usize) -> Vec<ChatMessage> {
        (0..count)
            .map(|index| message(&format!("m{index}"), index as u64))
            .collect()
    }

    fn ids(page: &[ChatMessage]) -> Vec<&str> {
        page.iter().map(|message| message.id.as_str()).collect()
    }

    #[test]
    fn no_limit_returns_the_whole_transcript() {
        let all = transcript(5);

        let (page, has_more) = slice_history_page(&all, None, None, None);

        assert_eq!(ids(&page), vec!["m0", "m1", "m2", "m3", "m4"]);
        assert!(!has_more);
    }

    #[test]
    fn limit_returns_the_newest_window() {
        let all = transcript(5);

        let (page, has_more) = slice_history_page(&all, Some(2), None, None);

        assert_eq!(ids(&page), vec!["m3", "m4"]);
        assert!(has_more);
    }

    #[test]
    fn cursor_walks_backwards_without_gaps_or_repeats() {
        let all = transcript(5);

        let (newest, more_newest) = slice_history_page(&all, Some(2), None, None);
        let (middle, more_middle) = slice_history_page(&all, Some(2), Some(3), Some("m3"));
        let (oldest, more_oldest) = slice_history_page(&all, Some(2), Some(1), Some("m1"));

        assert_eq!(ids(&newest), vec!["m3", "m4"]);
        assert_eq!(ids(&middle), vec!["m1", "m2"]);
        assert_eq!(ids(&oldest), vec!["m0"]);
        assert!(more_newest && more_middle);
        assert!(!more_oldest);
    }

    #[test]
    fn equal_timestamps_do_not_split_or_repeat_a_page() {
        // Every row shares a timestamp, so only the id anchor can page safely.
        let all: Vec<ChatMessage> = (0..4).map(|i| message(&format!("m{i}"), 7)).collect();

        let (newest, more_newest) = slice_history_page(&all, Some(2), None, None);
        let (oldest, more_oldest) = slice_history_page(&all, Some(2), Some(7), Some("m2"));

        assert_eq!(ids(&newest), vec!["m2", "m3"]);
        assert!(more_newest);
        assert_eq!(ids(&oldest), vec!["m0", "m1"]);
        assert!(!more_oldest);
    }

    #[test]
    fn unknown_cursor_id_falls_back_to_timestamp() {
        let all = transcript(5);

        let (page, has_more) = slice_history_page(&all, Some(2), Some(3), Some("rewound-away"));

        assert_eq!(ids(&page), vec!["m1", "m2"]);
        assert!(has_more);
    }
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
