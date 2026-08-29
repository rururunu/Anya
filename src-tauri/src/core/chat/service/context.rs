use crate::core::chat::compact;
use crate::core::chat::error::ChatError;
use crate::core::chat::session_origin::shared_session_origin_store;
use crate::core::runtime::{ChatMessage, Role};

use super::ChatService;

impl ChatService {
    /// Estimates context-window usage for the current session and optional draft.
    pub fn context_usage(
        &self,
        app: &tauri::AppHandle,
        session_id: Option<String>,
        draft_message: Option<String>,
        context: Option<crate::core::runtime::RequestContext>,
        model_id: Option<String>,
    ) -> Result<crate::models::chat::ContextUsageResponse, ChatError> {
        use crate::core::chat::model_context::effective_context_window;
        use crate::services::settings_store::get_settings;

        let history = match session_id.as_deref() {
            Some(id) => self.conversation.messages(id),
            None => Vec::new(),
        };
        let current_workspace = self.workspace_manager.current();
        let known_workspaces = self.workspace_manager.list();
        let mut ctx = self.context_resolver.resolve_request(
            context.unwrap_or_else(|| self.context_resolver.resolve()),
            current_workspace.as_ref(),
            &known_workspaces,
        );
        crate::core::context::provider::environment_provider::collect(&mut ctx);

        let settings = get_settings(app).ok();
        let large_context = settings
            .as_ref()
            .map(|settings| settings.large_context_enabled)
            .unwrap_or(true);
        let model = model_id
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(str::to_string)
            .or_else(|| {
                settings
                    .as_ref()
                    .map(|settings| settings.chat_model.clone())
            })
            .unwrap_or_default();
        let context_window = effective_context_window(large_context, &model);
        let extras = self.context_usage_extras(
            session_id.as_deref(),
            &history,
            &ctx,
            draft_message.as_deref(),
            &settings,
        );
        let measure = compact::measure_context_usage_with(
            &history,
            &ctx,
            draft_message.as_deref(),
            context_window,
            extras,
        );

        Ok(crate::models::chat::ContextUsageResponse {
            usage_ratio: measure.usage_ratio,
            estimated_tokens: measure.estimated_tokens,
            context_window_tokens: context_window,
            system_prompt_tokens: measure.system_prompt_tokens,
            environment_tokens: measure.environment_tokens,
            tools_tokens: measure.tools_tokens,
            rules_tokens: measure.rules_tokens,
            memories_tokens: measure.memories_tokens,
            skills_tokens: measure.skills_tokens,
            mcp_tokens: measure.mcp_tokens,
            subagent_tokens: measure.subagent_tokens,
            summarized_tokens: measure.summarized_tokens,
            message_tokens: measure.message_tokens,
        })
    }

    fn context_usage_extras(
        &self,
        session_id: Option<&str>,
        history: &[ChatMessage],
        ctx: &crate::core::runtime::RequestContext,
        draft_message: Option<&str>,
        settings: &Option<crate::models::settings::AppSettings>,
    ) -> compact::ContextUsageExtras {
        use crate::core::chat::limits::{MEMORIES_MAX_CHARS, RULES_MAX_CHARS};
        use crate::core::chat::prompts::{
            COMPANION_ORIGIN_PROMPT, IMAGE_MODE_PROMPT, MINIMAL_CODING_PROMPT,
            MULTI_MODEL_COLLABORATION_PROMPT, PLAN_MODE_PROMPT,
        };
        use crate::core::rules::RuleEngine;
        use crate::core::tools::plan_mode::shared_plan_mode_store;

        let workspace_root = ctx
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root));
        let last_user = history
            .iter()
            .rev()
            .find(|message| message.role == Role::User)
            .map(|message| message.content.as_str())
            .unwrap_or("");
        let recall_text = draft_message
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(last_user);
        let is_new_session = history
            .iter()
            .filter(|message| matches!(message.role, Role::User))
            .count()
            <= 1;
        let task_rules =
            RuleEngine::prepare_task(recall_text, workspace_root.as_deref(), is_new_session);

        let session_id = session_id.unwrap_or("");
        let plan_mode = !session_id.is_empty() && shared_plan_mode_store().is_active(session_id);
        let companion_origin =
            !session_id.is_empty() && shared_session_origin_store().is_companion(session_id);
        let collaboration_models = settings
            .as_ref()
            .map(|settings| settings.collaboration_models.as_slice())
            .unwrap_or(&[]);
        let minimal_coding = settings
            .as_ref()
            .map(|settings| settings.minimal_coding)
            .unwrap_or(false);

        let mut policy_suffix_tokens = 0;
        if companion_origin {
            policy_suffix_tokens += compact::estimate_tokens(COMPANION_ORIGIN_PROMPT);
        }
        if !collaboration_models.is_empty() {
            let list =
                crate::core::ai::model_ref::format_collaboration_prompt_ids(collaboration_models);
            let content = MULTI_MODEL_COLLABORATION_PROMPT.replace("{{MODELS}}", &list);
            policy_suffix_tokens += compact::estimate_tokens(&content);
        }
        if minimal_coding {
            policy_suffix_tokens += compact::estimate_tokens(MINIMAL_CODING_PROMPT);
        }
        if plan_mode {
            policy_suffix_tokens += compact::estimate_tokens(PLAN_MODE_PROMPT);
        }
        if crate::core::tools::image_mode::is_image_mode(session_id) {
            policy_suffix_tokens += compact::estimate_tokens(IMAGE_MODE_PROMPT);
        }

        let (tool_definition_tokens, mut skills_tokens, mut mcp_tokens, subagent_tokens) =
            estimate_tool_schema_groups(self.tools.registry().as_ref());
        let preferred = compact::estimate_prompt_block(
            task_rules.preferred_resources.as_deref(),
            RULES_MAX_CHARS,
        );
        if preferred > 0 {
            let block = task_rules.preferred_resources.as_deref().unwrap_or("");
            if block.contains("skill:") {
                skills_tokens += preferred;
            } else {
                mcp_tokens += preferred;
            }
        }

        compact::ContextUsageExtras {
            rules_tokens: compact::estimate_prompt_block(
                task_rules.project_rules.as_deref(),
                RULES_MAX_CHARS,
            ),
            memories_tokens: compact::estimate_prompt_block(
                task_rules.recalled_memories.as_deref(),
                MEMORIES_MAX_CHARS,
            ),
            skills_tokens,
            mcp_tokens,
            subagent_tokens,
            tool_definition_tokens,
            policy_suffix_tokens,
        }
    }

    /// Returns environment context including the workbench's selected workspace.
    pub fn environment_context(&self) -> crate::core::runtime::RequestContext {
        self.resolve_environment_context(true)
    }

    /// Returns environment context for the overlay without inheriting the workbench workspace.
    pub fn environment_context_for_overlay(&self) -> crate::core::runtime::RequestContext {
        self.resolve_environment_context(false)
    }

    fn resolve_environment_context(
        &self,
        include_current_workspace: bool,
    ) -> crate::core::runtime::RequestContext {
        let current_workspace = if include_current_workspace {
            self.workspace_manager.current()
        } else {
            None
        };
        let known_workspaces = self.workspace_manager.list();
        let captured = self.context_resolver.resolve();
        tracing::debug!(
            active_window = ?captured.active_window.as_deref(),
            active_file = ?captured.active_file.as_deref(),
            workspace = ?captured.workspace.as_ref().map(|workspace| workspace.root.as_str()),
            selected_files = captured.selected_files.len(),
            ide = ?captured.ide_context.as_ref().map(|ide| ide.ide.as_str()),
            include_current_workspace,
            "ChatService::resolve_environment_context input captured context"
        );
        let mut context = self.context_resolver.resolve_request(
            captured,
            current_workspace.as_ref(),
            &known_workspaces,
        );
        crate::core::context::provider::environment_provider::collect(&mut context);
        tracing::debug!(
            active_window = ?context.active_window.as_deref(),
            active_file = ?context.active_file.as_deref(),
            workspace = ?context.workspace.as_ref().map(|workspace| workspace.root.as_str()),
            has_git_status = context.git_status.is_some(),
            has_shell_execution = context.last_shell_execution.is_some(),
            ide = ?context.ide_context.as_ref().map(|ide| ide.ide.as_str()),
            "ChatService::resolve_environment_context final resolved context"
        );
        context
    }
}

fn estimate_tool_schema_groups(
    registry: &crate::core::tools::ToolRegistry,
) -> (usize, usize, usize, usize) {
    let mut definitions = 0;
    let mut skills = 0;
    let mut mcp = 0;
    let mut subagents = 0;
    for name in registry.names() {
        let Some(tool) = registry.get(&name) else {
            continue;
        };
        if !tool.available() {
            continue;
        }
        let tokens = compact::estimate_tokens(&tool.schema().to_string());
        if name.starts_with("mcp__") {
            mcp += tokens;
        } else if crate::core::tools::agent::is_subagent_tool(&name) {
            subagents += tokens;
        } else if crate::core::tools::skills::is_skill_tool(&name) {
            skills += tokens;
        } else {
            definitions += tokens;
        }
    }
    (definitions, skills, mcp, subagents)
}
