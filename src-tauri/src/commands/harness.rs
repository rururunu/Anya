use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::core::checkpoint::{shared_checkpoint_store, Checkpoint, CheckpointStore};
use crate::core::event::PlanModeSource;
use crate::core::tools::plan_mode::shared_plan_mode_store;
use crate::core::tools::tool_approval::shared_tool_approval_store;
use crate::models::chat::InteractionResolvedEvent;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondToolApprovalRequest {
    pub request_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPlanModeRequest {
    pub session_id: String,
    pub active: bool,
    /// How the gate was toggled. Defaults to manual (mode picker / explicit IPC).
    #[serde(default)]
    pub source: Option<PlanModeSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPlanModeRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCheckpointsRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewindSessionRequest {
    pub session_id: String,
    pub turn: usize,
    /// `code` | `conversation` | `both`
    pub restore: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewindSessionResponse {
    pub restored_files: usize,
    pub truncated_messages: bool,
}

#[tauri::command]
pub fn respond_tool_approval(
    app: AppHandle,
    request: RespondToolApprovalRequest,
) -> Result<(), String> {
    let ok = shared_tool_approval_store().complete(&request.request_id, &request.decision);
    if ok {
        let _ = app.emit(
            "interaction-resolved",
            InteractionResolvedEvent {
                request_id: request.request_id,
                kind: "tool_approval".to_string(),
            },
        );
        Ok(())
    } else {
        Err("tool approval request not found or already completed".into())
    }
}

#[tauri::command]
pub fn set_plan_mode(
    state: State<'_, AppState>,
    request: SetPlanModeRequest,
) -> Result<(), String> {
    shared_plan_mode_store().set_active(&request.session_id, request.active);
    let source = request.source.unwrap_or(PlanModeSource::Manual);
    state
        .core
        .chat()
        .emit_plan_mode_changed(&request.session_id, request.active, source);
    Ok(())
}

#[tauri::command]
pub fn get_plan_mode(request: GetPlanModeRequest) -> Result<bool, String> {
    Ok(shared_plan_mode_store().is_active(&request.session_id))
}

#[tauri::command]
pub fn list_checkpoints(request: ListCheckpointsRequest) -> Result<Vec<Checkpoint>, String> {
    shared_checkpoint_store()
        .list(&request.session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rewind_session(
    state: State<'_, AppState>,
    request: RewindSessionRequest,
) -> Result<RewindSessionResponse, String> {
    let restore = request.restore.as_str();
    if !matches!(restore, "code" | "conversation" | "both") {
        return Err("restore must be code, conversation, or both".into());
    }

    let mut restored_files = 0usize;
    let mut truncated_messages = false;
    let checkpoint = shared_checkpoint_store()
        .list(&request.session_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|checkpoint| checkpoint.turn == request.turn)
        .ok_or_else(|| format!("checkpoint turn {} not found", request.turn))?;

    if restore == "code" || restore == "both" {
        let session_root = state
            .core
            .chat()
            .conversation()
            .workspace_for_session(&request.session_id)
            .map(std::path::PathBuf::from);
        let root = session_root
            .or_else(|| {
                checkpoint
                    .workspace_root
                    .clone()
                    .map(std::path::PathBuf::from)
            })
            .or_else(|| {
                state
                    .core
                    .workspaces()
                    .current()
                    .map(|workspace| workspace.root)
            })
            .filter(|path| !path.as_os_str().is_empty());
        restored_files = restore_checkpoint_code(
            shared_checkpoint_store(),
            &request.session_id,
            &checkpoint,
            root.as_deref(),
        )?;
    }

    if restore == "conversation" || restore == "both" {
        let Some(user_message_id) = &checkpoint.user_message_id else {
            return Err("checkpoint has no user_message_id for conversation rewind".into());
        };
        state
            .core
            .chat()
            .conversation()
            .truncate_from_message(&request.session_id, user_message_id)
            .await
            .map_err(|e| e.to_string())?;
        truncated_messages = true;
        // Drop later checkpoints for this session after rewind turn
        let _ = shared_checkpoint_store().drop_from_turn(&request.session_id, request.turn);
    }

    Ok(RewindSessionResponse {
        restored_files,
        truncated_messages,
    })
}

fn restore_checkpoint_code(
    store: &CheckpointStore,
    session_id: &str,
    checkpoint: &Checkpoint,
    workspace_root: Option<&std::path::Path>,
) -> Result<usize, String> {
    if checkpoint.files.is_empty() {
        return Ok(0);
    }
    let root = workspace_root.ok_or_else(|| "no workspace selected for code rewind".to_string())?;
    store
        .restore_code(session_id, checkpoint.turn, root)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::restore_checkpoint_code;
    use crate::core::checkpoint::{Checkpoint, CheckpointStore, FileSnap};

    fn checkpoint(files: Vec<FileSnap>) -> Checkpoint {
        Checkpoint {
            turn: 1,
            time: 0,
            prompt: "question".into(),
            files,
            user_message_id: Some("user-1".into()),
            workspace_root: None,
        }
    }

    #[test]
    fn conversation_only_checkpoint_does_not_require_workspace() {
        let root = std::env::temp_dir().join(format!("anya-rewind-{}", uuid::Uuid::new_v4()));
        let store = CheckpointStore::new(root);
        assert_eq!(
            restore_checkpoint_code(&store, "session", &checkpoint(vec![]), None),
            Ok(0)
        );
    }

    #[test]
    fn file_checkpoint_still_requires_workspace() {
        let root = std::env::temp_dir().join(format!("anya-rewind-{}", uuid::Uuid::new_v4()));
        let store = CheckpointStore::new(root);
        let error = restore_checkpoint_code(
            &store,
            "session",
            &checkpoint(vec![FileSnap {
                path: "file.txt".into(),
                content: Some("old".into()),
            }]),
            None,
        )
        .expect_err("file restore should require a workspace");
        assert!(error.contains("no workspace selected"));
    }
}
