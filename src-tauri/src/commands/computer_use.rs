//! Computer Use HUD commands.

use tauri::State;

use crate::app_state::AppState;

/// Stop the operation shown in the banner and cancel its owning assistant turn.
#[tauri::command]
pub fn dismiss_computer_use_hud(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(message_id) = crate::core::plugins::stop_computer_turn() {
        let _ = state.core.chat().cancel(&message_id);
    }
    Ok(())
}
