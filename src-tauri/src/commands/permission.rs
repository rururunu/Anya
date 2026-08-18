use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::core::remote;
use crate::models::chat::{InteractionResolvedEvent, RespondPathPermissionRequest};

#[tauri::command]
pub fn respond_path_permission(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RespondPathPermissionRequest,
) -> Result<(), String> {
    let Some(session_id) = state
        .core
        .chat()
        .path_permission_store()
        .complete(&request.request_id, &request.decision)
    else {
        return Err("path permission request not found or already completed".into());
    };
    remote::push_interaction_resolved(&request.request_id, "path_permission", Some(&session_id));
    remote::resume_run_state_after_interaction(&app, &session_id);
    crate::commands::window::dismiss_tracked_interaction_notifications(
        &app,
        Some(&request.request_id),
        None,
    );
    let _ = app.emit(
        "interaction-resolved",
        InteractionResolvedEvent {
            request_id: request.request_id,
            kind: "path_permission".to_string(),
        },
    );
    Ok(())
}
