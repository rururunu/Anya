use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::core::remote;
use crate::models::chat::{InteractionResolvedEvent, RespondAskUserRequest};

#[tauri::command]
pub fn respond_ask_user(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RespondAskUserRequest,
) -> Result<(), String> {
    let ok = state
        .core
        .chat()
        .ask_store()
        .complete(&request.request_id, request.answer);
    if ok {
        remote::push_interaction_resolved(&request.request_id, "ask_user");
        crate::commands::window::dismiss_tracked_interaction_notifications(
            &app,
            Some(&request.request_id),
            None,
        );
        let _ = app.emit(
            "interaction-resolved",
            InteractionResolvedEvent {
                request_id: request.request_id,
                kind: "ask_user".to_string(),
            },
        );
        Ok(())
    } else {
        Err("ask request not found or already completed".into())
    }
}
