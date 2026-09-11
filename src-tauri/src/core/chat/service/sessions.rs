use crate::core::chat::error::ChatError;
use crate::core::event::BusEvent;
use crate::core::runtime::ChatMessage;

use super::ChatService;

impl ChatService {
    /// Cancels an in-flight assistant message by id.
    pub fn cancel(&self, message_id: &str) -> Result<(), ChatError> {
        self.agent_runtime.cancel(&self.conversation, message_id)
    }

    /// Returns the message history for a session.
    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        self.conversation.history(session_id)
    }

    /// Lists active chat sessions with summary metadata.
    pub fn list_sessions(&self) -> Vec<crate::models::chat::ChatSessionSummary> {
        self.conversation.list_sessions()
    }

    /// Lists archived chat sessions with summary metadata.
    pub fn list_archived_sessions(&self) -> Vec<crate::models::chat::ChatSessionSummary> {
        self.conversation.list_archived_sessions()
    }

    /// Archives or unarchives a single session.
    pub fn set_session_archived(&self, session_id: &str, archived: bool) {
        self.conversation.set_session_archived(session_id, archived);
    }

    /// Archives or unarchives all sessions bound to a workspace.
    pub fn set_sessions_archived_for_workspace(&self, workspace_id: &str, archived: bool) {
        self.conversation
            .set_sessions_archived_for_workspace(workspace_id, archived);
    }

    /// Branches a session at an optional message boundary.
    pub fn branch_session(
        &self,
        session_id: &str,
        until_message_id: Option<&str>,
    ) -> Result<crate::models::chat::ChatSessionSummary, String> {
        self.conversation
            .branch_session(session_id, until_message_id)
    }

    /// Renames a session and emits a title-updated event.
    pub fn set_session_title(&self, session_id: &str, title: &str) -> Result<String, String> {
        let title = self.conversation.rename_session_title(session_id, title)?;
        self.event_bus.emit(BusEvent::ChatSessionTitleUpdated {
            session_id: session_id.to_string(),
            title: title.clone(),
        });
        Ok(title)
    }

    /// Regenerates a session title from conversation context via the active provider.
    pub async fn regenerate_session_title(
        &self,
        session_id: &str,
        model_id: Option<&str>,
        model_provider: Option<&str>,
    ) -> Result<String, String> {
        let context_text = self
            .conversation
            .session_context_for_title(session_id)
            .ok_or_else(|| "当前对话没有可用于生成标题的消息".to_string())?;
        let overrides = crate::models::chat::ChatSendOverrides {
            model_id: model_id.map(str::to_string),
            model_provider: model_provider.map(str::to_string),
            ..Default::default()
        };
        let provider = self.resolve_provider(&overrides);
        let primary_user = self
            .conversation
            .primary_user_text_for_title(session_id)
            .unwrap_or_else(|| context_text.clone());
        let title = super::super::session_title::generate_session_title(
            provider,
            &context_text,
            &primary_user,
        )
        .await?;
        self.conversation.set_session_title(
            session_id,
            title.clone(),
            super::super::session_title::SessionTitleSource::Auto,
        );
        self.event_bus.emit(BusEvent::ChatSessionTitleUpdated {
            session_id: session_id.to_string(),
            title: title.clone(),
        });
        Ok(title)
    }
}
