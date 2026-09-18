//! Prompt assembly with fixed slots for KV-cache prefix stability.

mod language;
mod slots;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, RequestContext, Role};
use crate::models::settings::{AppLanguage, ReasoningLanguage};

use language::inject_language_blocks;
use slots::{
    format_volatile_context, inject_context, inject_memories, inject_optional_policy_suffix,
    inject_system_block, split_current_user, system_message,
};

/// Prompt 组装偏好 — 来自设置，不进入稳定 system。
#[derive(Debug, Clone, Default)]
pub struct PromptPreferences {
    pub app_language: AppLanguage,
    pub reasoning_language: ReasoningLanguage,
    pub collaboration_models: Vec<String>,
    /// Inject the optional minimal-coding ladder when enabled in Settings.
    pub minimal_coding: bool,
    /// Inject plan-mode instructions while writer tools are gated.
    pub plan_mode: bool,
    /// Nudge Agent to call `request_plan_mode` on complex work (does not gate writers).
    pub suggest_plan_request: bool,
    /// True when this turn was sent from the paired phone (Companion app).
    pub companion_origin: bool,
    /// Inject image-mode instructions and pin generate_image arguments.
    pub image_mode: Option<ImageModePolicy>,
}

#[derive(Debug, Clone, Default)]
pub struct ImageModePolicy {
    pub size: String,
    pub quality: String,
    pub n: u8,
    pub style_prompt: String,
    pub has_reference: bool,
}

impl From<&crate::core::tools::image_mode::ImageModeOptions> for ImageModePolicy {
    fn from(options: &crate::core::tools::image_mode::ImageModeOptions) -> Self {
        Self {
            size: options.size.clone(),
            quality: options.quality.clone(),
            n: options.n,
            style_prompt: options.style_prompt.clone(),
            has_reference: !options.reference_images.is_empty(),
        }
    }
}

pub struct PromptBuildInput<'a> {
    pub request_id: &'a str,
    pub session_id: &'a str,
    pub history: &'a [ChatMessage],
    pub context: &'a RequestContext,
    pub project_rules: Option<&'a str>,
    pub recalled_memories: Option<&'a str>,
    pub preferred_resources: Option<&'a str>,
    pub provider: Option<String>,
    pub preferences: &'a PromptPreferences,
}

/// AI Runtime Prompt 组装 — 固定槽位以保护 KV cache 前缀稳定性：
///
/// ```text
/// [0] SYSTEM_PROMPT
/// [1] stable workspace / IDE identity
/// [2] project rules (agent.md / AGENTS.md)
/// [3] optional policy suffix (collab / minimal-coding / plan hint / …)
/// [4] plugin prompt suffix (deterministic order)
/// [5..] history + recalled memories + current user
///        └─ preferred #skill/#mcp chips + live context appended to user text
/// ```
///
/// Optional strategy toggles only populate the policy suffix slot; they never
/// insert ahead of context/rules, so enabling/disabling them cannot
/// shift the stable prefix. Per-turn chips and volatile capture fields hang off
/// the current user message so they do not invalidate the cached prefix.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(input: PromptBuildInput<'_>) -> ChatRequest {
        let PromptBuildInput {
            request_id,
            session_id,
            history,
            context,
            project_rules,
            recalled_memories,
            preferred_resources,
            provider,
            preferences,
        } = input;
        let mut messages = Vec::with_capacity(history.len() + 7);

        // Always inject the stable agent preamble. A compaction summary in
        // history is an extra system note, not a replacement for SYSTEM_PROMPT.
        if !history
            .iter()
            .any(|message| message.id == format!("system-{session_id}"))
        {
            messages.push(system_message(session_id));
        }

        // [1]–[2] Core context slots (order locked).
        inject_context(&mut messages, session_id, context);
        inject_system_block(&mut messages, session_id, "rules", project_rules);

        // [3] Optional policy suffix — toggles only hang here.
        inject_optional_policy_suffix(
            &mut messages,
            session_id,
            &preferences.collaboration_models,
            preferences.minimal_coding,
            preferences.plan_mode,
            preferences.suggest_plan_request,
            preferences.companion_origin,
            preferences.image_mode.as_ref(),
        );
        let plugin_prompts = if cfg!(test) {
            None
        } else {
            crate::core::plugins::shared_runtime().plugin_prompt_suffix()
        };
        inject_system_block(
            &mut messages,
            session_id,
            "plugin-prompts",
            plugin_prompts.as_deref(),
        );

        // [5..] History（排除 pending 的空 assistant）
        let (prior, current_user) = split_current_user(history);
        messages.extend(prior.into_iter().filter(ChatMessage::contributes_to_api));
        // Query-dependent retrieval must not invalidate the historical prefix.
        inject_memories(&mut messages, session_id, recalled_memories);

        // 当前用户输入（含 transient 语言块 + per-turn chips + 本轮易变语境）
        let mut user_tail = String::new();
        if let Some(resources) = preferred_resources
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            user_tail.push_str(resources);
        }
        if let Some(live) = format_volatile_context(context) {
            if !user_tail.is_empty() {
                user_tail.push_str("\n\n");
            }
            user_tail.push_str(&live);
        }
        if let Some(mut user_message) = current_user {
            user_message.content = inject_language_blocks(&user_message.content, preferences);
            if !user_tail.is_empty() {
                user_message.content = format!("{}\n\n{user_tail}", user_message.content);
            }
            messages.push(user_message);
        } else if !user_tail.is_empty() {
            messages.push(ChatMessage {
                id: format!("context-live-{session_id}"),
                session_id: session_id.to_string(),
                role: Role::User,
                content: user_tail,
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 0,
                estimated_tokens: None,
            });
        }

        ChatRequest {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            messages,
            context: context.clone(),
            provider,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        }
    }
}

/// Inject plan-mode instructions if the gate flipped mid-turn (user accepted
/// `request_plan_mode`) and the assembled prompt does not already include them.
pub fn ensure_plan_mode_prompt(messages: &mut Vec<ChatMessage>, session_id: &str) {
    if !crate::core::tools::plan_mode::shared_plan_mode_store().is_active(session_id) {
        return;
    }
    let id = format!("plan-mode-{session_id}");
    if messages.iter().any(|message| message.id == id) {
        return;
    }
    inject_system_block(
        messages,
        session_id,
        "plan-mode",
        Some(crate::core::chat::prompts::PLAN_MODE_PROMPT),
    );
}
