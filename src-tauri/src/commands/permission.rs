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
    let ok = state
        .core
        .chat()
        .path_permission_store()
        .complete(&request.request_id, &request.decision);
    if ok {
        remote::push_interaction_resolved(&request.request_id, "path_permission");
        let _ = app.emit(
            "interaction-resolved",
            InteractionResolvedEvent {
                request_id: request.request_id,
                kind: "path_permission".to_string(),
            },
        );
        Ok(())
    } else {
        Err("path permission request not found or already completed".into())
    }
}
