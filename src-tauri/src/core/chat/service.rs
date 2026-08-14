use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::agent::{AgentDebugEvent, AgentRuntime, AgentSpawnInput};
use crate::core::ai::provider::AIProvider;
use crate::core::chat::compact;
use crate::core::chat::conversation_manager::{create_message, ConversationManager};
use crate::core::chat::error::ChatError;
use crate::core::chat::preferences::SendPreferences;
use crate::core::chat::prompt::{PromptBuildInput, PromptBuilder, PromptPreferences};
use crate::core::context::ContextResolver;
use crate::core::chat::session_origin::{shared_session_origin_store, RequestOrigin};
use crate::core::event::{BusEvent, EventBus, PlanModeSource};
use crate::core::runtime::{ChatMessage, MessageStatus, Role, DEFAULT_SESSION_ID};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem};
use crate::core::workspace::{Workspace, WorkspaceManager};
use crate::models::chat::ChatSendOverrides;
use crate::models::settings::ChatMode;
use crate::runtime::ToolManager;
use tauri::Emitter;

pub struct ChatSendResult {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub agent_run_id: Option<String>,
}

pub struct ChatService {
    provider: Arc<dyn AIProvider>,
    event_bus: Arc<dyn EventBus>,
    conversation: Arc<ConversationManager>,
    workspace_manager: Arc<WorkspaceManager>,
    context_resolver: ContextResolver,
    agent_runtime: AgentRuntime,
    tools: Arc<ToolManager>,
    ask_store: Arc<AskStore>,
    path_permission_store: Arc<PathPermissionStore>,
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    app_handle: Option<tauri::AppHandle>,
}

impl ChatService {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        event_bus: Arc<dyn EventBus>,
        context_resolver: ContextResolver,
        tools: Arc<ToolManager>,
        conversation: Arc<ConversationManager>,
        workspace_manager: Arc<WorkspaceManager>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        let agent_runtime = AgentRuntime::new(Arc::clone(&event_bus), Arc::clone(&tools));
        Self {
            provider,
            event_bus,
            conversation,
            workspace_manager,
            context_resolver,
            agent_runtime,
            tools,
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            tasks: Arc::new(Mutex::new(Vec::new())),
            app_handle: Some(app_handle),
        }
    }

    pub fn conversation(&self) -> Arc<ConversationManager> {
        Arc::clone(&self.conversation)
    }

    pub fn ask_store(&self) -> Arc<AskStore> {
        Arc::clone(&self.ask_store)
    }

    pub fn path_permission_store(&self) -> Arc<PathPermissionStore> {
        Arc::clone(&self.path_permission_store)
    }

    pub fn agent_debug_snapshot(&self) -> Vec<AgentDebugEvent> {
        self.agent_runtime.debug_snapshot()
    }

    /// Resolve the AI provider from current settings on every turn.
    /// Startup-time provider is only a fallback for tests without an AppHandle —
    /// otherwise switching Gemini ↔ DeepSeek would keep the wrong backend until restart.
    fn active_provider(&self) -> Arc<dyn AIProvider> {
        match &self.app_handle {
            Some(app) => crate::core::ai::resolve_provider(app.clone()),
            None => Arc::clone(&self.provider),
        }
    }

    /// Honor a per-conversation model override; otherwise fall back to global settings.
    fn resolve_provider(&self, overrides: &ChatSendOverrides) -> Arc<dyn AIProvider> {
        let model = overrides
            .model_id
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty());
        match (model, &self.app_handle) {
            (Some(model), Some(app)) => crate::core::ai::resolve_provider_for_selection(
                app.clone(),
                model.to_string(),
                overrides.model_provider.clone().unwrap_or_default(),
            ),
            _ => self.active_provider(),
        }
    }

    #[tracing::instrument(
        target = "peek.agent",
        name = "chat.send",
        skip(
            self,
            session_id,
            content,
            preferences,
            workspace_id,
            quick_ask,
            overrides,
            origin
        ),
        fields(
            session_id = %session_id.as_deref().unwrap_or(DEFAULT_SESSION_ID),
            content_len = content.len(),
            content_preview = %content.chars().take(120).collect::<String>(),
        )
    )]
    pub async fn send(
        &self,
        session_id: Option<String>,
        content: String,
        preferences: SendPreferences,
        workspace_id: Option<String>,
        quick_ask: bool,
        overrides: ChatSendOverrides,
        origin: RequestOrigin,
    ) -> Result<ChatSendResult, ChatError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }

        let session_id = session_id.unwrap_or_else(|| DEFAULT_SESSION_ID.to_string());
        shared_session_origin_store().mark(&session_id, origin);

        // Mid-turn soft inject: queue into the active agent loop (tool boundary).
        if let Some(assistant_message_id) =
            self.agent_runtime.active_assistant_for_session(&session_id)
        {
            return self.soft_inject(&session_id, content, &assistant_message_id);
        }

        let known_workspaces = self.workspace_manager.list();
        // Prefer the request's workspace, then the session binding. Never rely
        // solely on IDE/window inference for an already-bound conversation —
        // approve/queue/resume paths often omit workspace_id and would otherwise
        // silently bind the foreground IDE (e.g. this app's own repo).
        let explicit_workspace = workspace_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty());
        let already_bound = self.conversation.workspace_for_session(&session_id);
        let inbox_root = self.app_handle.as_ref().and_then(|app| {
            crate::core::remote::inbox_root_if_exists(app, &session_id)
        });
        // Phone FAB / 随文 omits workspace_id. Never inherit the desktop's
        // currently selected workspace — that fallback is only for workbench
        // turns that forgot to pass one. Workspace-folder "+" still sends an
        // explicit id; later turns of a bound session use `already_bound`.
        let quick_ask = quick_ask
            || (matches!(origin, RequestOrigin::Companion)
                && explicit_workspace.is_none()
                && already_bound.is_none());
        let using_inbox = explicit_workspace.is_none()
            && already_bound.is_none()
            && inbox_root.is_some();
        let workspace = if using_inbox || quick_ask {
            None
        } else {
            self.resolve_send_workspace(&session_id, workspace_id.as_deref(), &known_workspaces)
        };
        if let Some(workspace) = workspace.as_ref() {
            self.workspace_manager
                .touch(&workspace.id)
                .await
                .map_err(ChatError::Internal)?;
        }

        let agent_run_id = self.agent_runtime.create_run(content.clone());
        let mut context = self
            .agent_runtime
            .collect_context(&agent_run_id, || {
                let mut context = self
                    .context_resolver
                    .resolve_environment(workspace.as_ref(), &known_workspaces);
                crate::core::context::provider::environment_provider::collect(&mut context);
                context
            })
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        // An explicitly selected / session-bound workspace owns the turn. IDE
        // context is still useful for files and selection, but must not switch
        // the active project root.
        if using_inbox {
            if let Some(root) = inbox_root.as_ref() {
                context.set_workspace("Uploads".to_string(), root);
            }
        } else if let Some(workspace) = workspace.as_ref() {
            context.set_workspace(workspace.name.clone(), &workspace.root);
        }
        if quick_ask {
            context.workspace = None;
        }
        if !using_inbox {
            self.remember_ide_workspace(&context).await;
        }
        let known_workspaces = self.workspace_manager.list();
        let is_new_session = self.conversation.messages(&session_id).is_empty();
        if !using_inbox && is_new_session && !quick_ask {
            if let Some(resolved) = context.workspace.as_ref() {
                let workspace_id = known_workspaces
                    .iter()
                    .find(|workspace| workspace.root == PathBuf::from(&resolved.root))
                    .map(|workspace| workspace.id.clone())
                    .unwrap_or_else(|| resolved.root.clone());
                self.conversation.bind_workspace(&session_id, &workspace_id);
            }
        } else if !using_inbox && !quick_ask {
            // Keep the session sticky even when later turns omit workspace_id.
            if let Some(workspace) = workspace.as_ref() {
                self.conversation
                    .bind_workspace(&session_id, &workspace.id);
            }
        }
        let user_message = create_message(&session_id, Role::User, content, MessageStatus::Done);
        let assistant_message = create_message(
            &session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Pending,
        );

        // Always persist the user turn (including plan approve) so Desktop /
        // Companion history and the session inbox show the approval message.
        self.conversation.append(&session_id, user_message.clone());
        self.conversation
            .append(&session_id, assistant_message.clone());

        self.event_bus.emit(BusEvent::ChatStarted {
            session_id: session_id.clone(),
            user_message: user_message.clone(),
            assistant_message: assistant_message.clone(),
            resume_plan: overrides.resume_plan,
        });

        // Memory recall may use `reqwest::blocking` — must not run on a tokio worker.
        let recall_text = super::selection::visible_user_text(&user_message.content).to_string();
        let workspace_root = context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root));
        let task_rules_result = tauri::async_runtime::spawn_blocking(move || {
            crate::core::rules::RuleEngine::prepare_task(
                &recall_text,
                workspace_root.as_deref(),
                is_new_session,
            )
        })
        .await;
        let task_rules = match task_rules_result {
            Ok(task_rules) => task_rules,
            Err(error) => {
                self.agent_runtime
                    .fail_run(&agent_run_id, "task preparation failed");
                return Err(ChatError::Provider(error.to_string()));
            }
        };
        let _memory_decision = task_rules.memory_decision;

        let mut history = self.conversation.messages(&session_id);
        if overrides.resume_plan {
            // Empty assistant is the stream target, not prompt history: trim it
            // so the (already persisted) approval is the final user turn.
            // Otherwise the empty assistant lands between turns and providers
            // reject the request (approve & execute fails).
            history = compact::trim_empty_assistant_tail(history);
        }
        let settings = self
            .app_handle
            .as_ref()
            .and_then(|app| crate::services::settings_store::get_settings(app).ok());
        let large_context = settings
            .as_ref()
            .map(|settings| settings.large_context_enabled)
            .unwrap_or(true);
        let model = overrides
            .model_id
            .as_deref()
            .filter(|model| !model.trim().is_empty())
            .map(str::to_string)
            .or_else(|| settings.as_ref().map(|settings| settings.chat_model.clone()))
            .unwrap_or_default();
        let context_window =
            crate::core::chat::model_context::effective_context_window(large_context, &model);
        // Mid-turn auto-compact uses the same window; overshoot → compact & continue.
        let max_turn_tokens = context_window;
        let provider = self.resolve_provider(&overrides);
        let summarizer = crate::core::chat::compact::ProviderSummarizer::new(Arc::clone(&provider));
        let compact = compact::prepare_history_for_prompt(
            &history,
            &context,
            &session_id,
            context_window,
            Some(&summarizer),
        )
        .await;
        if let Some(notice) = &compact.notice {
            let language_zh = matches!(
                preferences.app_language,
                crate::models::settings::AppLanguage::ZhCn
            );
            self.event_bus.emit(BusEvent::ChatContextNotice {
                session_id: session_id.clone(),
                kind: match notice.kind {
                    compact::ContextNoticeKind::ApproachingLimit => "approaching-limit".to_string(),
                    compact::ContextNoticeKind::Compacted => "compacted".to_string(),
                },
                message: compact::notice_message(notice, language_zh),
                usage_ratio: notice.usage_ratio,
                folded_messages: notice.folded_messages,
            });
        }
        let collaboration_models = settings
            .as_ref()
            .filter(|settings| settings.multi_model_collaboration)
            .map(|settings| settings.collaboration_models.clone())
            .unwrap_or_default();
        let minimal_coding = settings
            .as_ref()
            .map(|settings| settings.minimal_coding)
            .unwrap_or(false);
        let chat_mode = overrides
            .chat_mode
            .or_else(|| settings.as_ref().map(|settings| settings.chat_mode))
            .unwrap_or_default();

        // Plan can be chosen in the mode picker, or auto-entered for complex
        // Agent turns. Approve & execute sends skip_auto_plan so writers unlock.
        // Approval UI lives at the end of the assistant reply (not a composer banner).
        let plan_store = crate::core::tools::plan_mode::shared_plan_mode_store();
        let plan_was_active = plan_store.is_active(&session_id);
        if overrides.resume_plan {
            // Approve & execute: the backend owns the plan gate. Unlock writers
            // here instead of relying on the frontend's set_plan_mode IPC
            // landing first — a resume turn must always run unlocked.
            if plan_was_active {
                plan_store.set_active(&session_id, false);
                self.emit_plan_mode_changed(&session_id, false, PlanModeSource::Manual);
            }
        } else {
            match chat_mode {
                ChatMode::Ask => {
                    if plan_was_active {
                        plan_store.set_active(&session_id, false);
                        self.emit_plan_mode_changed(&session_id, false, PlanModeSource::Manual);
                    }
                }
                ChatMode::Plan => {
                    if !plan_was_active {
                        plan_store.set_active(&session_id, true);
                        self.emit_plan_mode_changed(&session_id, true, PlanModeSource::Manual);
                    }
                }
                ChatMode::Agent => {
                    if plan_was_active {
                        // Leaving a sticky plan gate for this Agent send.
                        plan_store.set_active(&session_id, false);
                        self.emit_plan_mode_changed(&session_id, false, PlanModeSource::Manual);
                    }
                    // Always evaluate auto-plan for Agent turns (including after
                    // clearing a leftover gate). Otherwise complex Agent asks
                    // never show the approval card / countdown.
                    if !overrides.skip_auto_plan
                        && crate::core::tools::plan_mode::should_auto_plan(
                            &user_message.content,
                            chat_mode,
                        )
                    {
                        plan_store.set_active(&session_id, true);
                        self.emit_plan_mode_changed(&session_id, true, PlanModeSource::Auto);
                    }
                }
            }
        }
        let plan_mode = plan_store.is_active(&session_id);

        let prompt_preferences = PromptPreferences {
            app_language: preferences.app_language,
            reasoning_language: preferences.reasoning_language,
            collaboration_models,
            minimal_coding,
            plan_mode,
            companion_origin: shared_session_origin_store().is_companion(&session_id),
        };
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: &assistant_message.id,
            session_id: &session_id,
            history: &compact.messages,
            context: &context,
            project_rules: task_rules.project_rules.as_deref(),
            recalled_memories: task_rules.recalled_memories.as_deref(),
            preferred_resources: task_rules.preferred_resources.as_deref(),
            provider: Some(provider.id().to_string()),
            preferences: &prompt_preferences,
        });

        let turn = history
            .iter()
            .filter(|message| matches!(message.role, Role::User))
            .count();
        crate::core::checkpoint::shared_checkpoint_store().begin_turn(
            &session_id,
            turn,
            &user_message.content,
            Some(user_message.id.clone()),
            context
                .workspace
                .as_ref()
                .map(|workspace| std::path::Path::new(&workspace.root)),
        );

        let tools = if chat_mode == ChatMode::Ask {
            Arc::new(self.tools.ask_mode())
        } else {
            // Agent and Plan share the full registry; PlanModeStore gates writers.
            Arc::clone(&self.tools)
        };

        // Per-conversation approval mode: register (or clear) the override for
        // this session so tool approvals honor each conversation's choice.
        crate::core::tools::tool_approval::shared_tool_approval_store()
            .set_session_mode(&session_id, overrides.tool_approval_mode);

        let spawn_result = self.agent_runtime.spawn(AgentSpawnInput {
            run_id: agent_run_id.clone(),
            provider,
            tools,
            conversation: Arc::clone(&self.conversation),
            ask_store: Arc::clone(&self.ask_store),
            path_permission_store: Arc::clone(&self.path_permission_store),
            tasks: Arc::clone(&self.tasks),
            app_handle: self.app_handle.clone(),
            request,
            assistant_message_id: assistant_message.id.clone(),
            session_id: session_id.clone(),
            max_turn_tokens,
            model,
        });
        if let Err(error) = spawn_result {
            self.agent_runtime
                .fail_run(&agent_run_id, "agent runtime failed to start");
            return Err(ChatError::Internal(error.to_string()));
        }

        Ok(ChatSendResult {
            session_id,
            user_message_id: user_message.id,
            assistant_message_id: assistant_message.id,
            agent_run_id: Some(agent_run_id),
        })
    }

    fn resolve_send_workspace(
        &self,
        session_id: &str,
        workspace_id: Option<&str>,
        known_workspaces: &[Workspace],
    ) -> Option<Workspace> {
        let lookup = |id: &str| -> Option<Workspace> {
            known_workspaces
                .iter()
                .find(|workspace| workspace.id == id)
                .cloned()
                .or_else(|| {
                    let root = PathBuf::from(id);
                    known_workspaces
                        .iter()
                        .find(|workspace| workspace.root == root)
                        .cloned()
                })
        };

        if let Some(id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) {
            if let Some(workspace) = lookup(id) {
                return Some(workspace);
            }
        }

        if let Some(bound) = self.conversation.workspace_for_session(session_id) {
            if let Some(workspace) = lookup(&bound) {
                return Some(workspace);
            }
        }

        // Last resort for *workbench* turns that omitted workspace_id before
        // the session was bound. Companion unbound sends are treated as
        // quick-ask in `send()` and must not reach this fallback.
        self.workspace_manager.current()
    }

    async fn remember_ide_workspace(&self, context: &crate::core::runtime::RequestContext) {
        let root = context
            .ide_context
            .as_ref()
            .and_then(|ide| ide.workspace.clone())
            .or_else(|| {
                context
                    .workspace
                    .as_ref()
                    .map(|workspace| PathBuf::from(&workspace.root))
            });
        let ide = context
            .ide_context
            .as_ref()
            .map(|ide| ide.ide.as_str())
            .unwrap_or("ide");
        let Some(root) = root else {
            return;
        };

        match self.workspace_manager.remember_from_ide(root, ide).await {
            Ok((_, false)) => {}
            Ok((workspace, true)) => {
                if let Some(app) = &self.app_handle {
                    if let Err(error) =
                        app.emit("workspaces-changed", self.workspace_manager.current())
                    {
                        tracing::warn!(
                            provider = "ide",
                            workspace = %workspace.root.display(),
                            error = %error,
                            "failed to emit IDE workspace update"
                        );
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    provider = "ide",
                    ide = %ide,
                    error = %error,
                    "failed to remember IDE workspace"
                );
            }
        }
    }

    fn soft_inject(
        &self,
        session_id: &str,
        content: String,
        assistant_message_id: &str,
    ) -> Result<ChatSendResult, ChatError> {
        // Marker persists soft-inject identity across history reload (UI folds these
        // into the preceding assistant turn instead of an unanswered user bubble).
        const SOFT_INJECT_MARKER: &str = "<!--peek:soft-inject-->\n";
        let stored = format!("{SOFT_INJECT_MARKER}{content}");
        let user_message = create_message(session_id, Role::User, stored, MessageStatus::Done);
        self.conversation.append(session_id, user_message.clone());
        // Agent queue gets plain text (no HTML marker).
        self.agent_runtime.soft_inject(session_id, content)?;

        // Do not emit ChatStarted: that would re-project the assistant bubble and can
        // wipe in-flight streamed content. Frontend already staged the user message.
        Ok(ChatSendResult {
            session_id: session_id.to_string(),
            user_message_id: user_message.id,
            assistant_message_id: assistant_message_id.to_string(),
            agent_run_id: self
                .agent_runtime
                .run_for_message(assistant_message_id)
                .map(|run| run.id),
        })
    }

    pub fn cancel(&self, message_id: &str) -> Result<(), ChatError> {
        self.agent_runtime.cancel(&self.conversation, message_id)
    }

    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        self.conversation.history(session_id)
    }

    pub fn list_sessions(&self) -> Vec<crate::models::chat::ChatSessionSummary> {
        self.conversation.list_sessions()
    }

    pub fn context_usage(
        &self,
        app: &tauri::AppHandle,
        session_id: Option<String>,
        draft_message: Option<String>,
        context: Option<crate::core::runtime::RequestContext>,
        model_id: Option<String>,
    ) -> Result<crate::models::chat::ContextUsageResponse, ChatError> {
        use crate::core::chat::model_context::effective_context_window;
        use crate::core::chat::compact::measure_context_usage;
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
            .or_else(|| settings.map(|settings| settings.chat_model))
            .unwrap_or_default();
        let context_window = effective_context_window(large_context, &model);
        let measure =
            measure_context_usage(&history, &ctx, draft_message.as_deref(), context_window);

        Ok(crate::models::chat::ContextUsageResponse {
            usage_ratio: measure.usage_ratio,
            estimated_tokens: measure.estimated_tokens,
            context_window_tokens: context_window,
        })
    }

    pub fn environment_context(&self) -> crate::core::runtime::RequestContext {
        self.resolve_environment_context(true)
    }

    /// Overlay (Alt+Alt) capture must not inherit the workbench's selected
    /// workspace. Binding happens only when the user picks a workspace in the
    /// overlay composer (or when IDE/window signals provide one).
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

    pub fn emit_plan_mode_changed(
        &self,
        session_id: &str,
        active: bool,
        source: PlanModeSource,
    ) {
        self.event_bus.emit(BusEvent::PlanModeChanged {
            session_id: session_id.to_string(),
            active,
            source,
        });
    }
}
