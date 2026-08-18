use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::core::ai::provider::AIProvider;
use crate::core::chat::limits::{
    self, estimate_message_tokens, truncate_chars, FOLD_PAYLOAD_MSG_MAX_CHARS,
    FOLD_PAYLOAD_TOTAL_MAX_CHARS, LLM_COMPACT_TIMEOUT_SECS,
};
use crate::core::chat::prompts::SYSTEM_PROMPT;
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, RequestContext, Role, StreamEvent,
};

/// 标准上下文窗口（token 粗算基准）。
pub const DEFAULT_CONTEXT_WINDOW: usize = crate::core::chat::limits::DEFAULT_MAX_TURN_TOKENS;
/// 1M 大上下文窗口（设置开启时使用）。
pub const LARGE_CONTEXT_WINDOW: usize = crate::core::chat::limits::LARGE_MAX_TURN_TOKENS;

/// Resolve the active context window from the large-context toggle.
/// Prefer [`crate::core::chat::model_context::effective_context_window`] when a
/// model id is known — the 1M ceiling must not exceed the model's native limit.
#[allow(dead_code)]
pub fn context_window_tokens(large_context_enabled: bool) -> usize {
    if large_context_enabled {
        LARGE_CONTEXT_WINDOW
    } else {
        DEFAULT_CONTEXT_WINDOW
    }
}
/// 达到该比例时尝试压缩较早消息。
pub const COMPACT_TRIGGER_RATIO: f32 = 0.8;
/// 保留最近 N 轮完整对话（user + assistant 各算一条）。
pub const KEEP_TAIL_TURNS: usize = 3;
/// 折叠摘要中每条消息的最大字符数（机械路径）。
pub const SNIPPET_MAX_CHARS: usize = 160;
/// 折叠摘要总字符上限。
pub const SUMMARY_MAX_CHARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextNoticeKind {
    Compacted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextNotice {
    pub kind: ContextNoticeKind,
    pub usage_ratio: f32,
    pub folded_messages: Option<usize>,
    pub estimated_tokens: usize,
    pub context_window_tokens: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompactResult {
    pub messages: Vec<ChatMessage>,
    pub notice: Option<ContextNotice>,
}

impl CompactResult {
    pub fn summary_message(&self) -> Option<&ChatMessage> {
        self.messages
            .iter()
            .find(|message| is_compaction_summary(message))
    }

    pub fn first_kept_id(&self) -> Option<&str> {
        self.messages
            .iter()
            .find(|message| !is_compaction_summary(message))
            .map(|message| message.id.as_str())
    }
}

#[async_trait]
pub trait ConversationSummarizer: Send + Sync {
    async fn summarize(&self, folded_payload: &str) -> Result<String, String>;
}

pub struct ProviderSummarizer {
    provider: Arc<dyn AIProvider>,
}

impl ProviderSummarizer {
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl ConversationSummarizer for ProviderSummarizer {
    async fn summarize(&self, folded_payload: &str) -> Result<String, String> {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(16);
        let request = ChatRequest {
            request_id: format!("compact-{}", now_millis()),
            session_id: "compact".to_string(),
            messages: vec![
                ChatMessage {
                    id: "compact-system".into(),
                    session_id: "compact".into(),
                    role: Role::System,
                    content: "You compress earlier conversation turns into a compact factual summary for a coding agent. Preserve decisions, file paths, errors, and unfinished work. Reply with plain text only. Stay under 4000 characters.".into(),
                    reasoning: None,
                    work_timeline: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    status: MessageStatus::Done,
                    timestamp: 0,
                    estimated_tokens: None,
                },
                ChatMessage {
                    id: "compact-user".into(),
                    session_id: "compact".into(),
                    role: Role::User,
                    content: format!(
                        "Summarize the earlier conversation below:\n\n{folded_payload}"
                    ),
                    reasoning: None,
                    work_timeline: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    status: MessageStatus::Done,
                    timestamp: 0,
                    estimated_tokens: None,
                },
            ],
            context: RequestContext::default(),
            provider: Some(self.provider.id().to_string()),
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: Some(0.2),
            max_tokens: Some(1200),
        };

        let provider = Arc::clone(&self.provider);
        let provider_task =
            tauri::async_runtime::spawn(async move { provider.stream(request, tx).await });

        let mut content = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(delta) => content.push_str(&delta),
                StreamEvent::TurnComplete {
                    content: turn_content,
                    ..
                } => {
                    if !turn_content.is_empty() {
                        content = turn_content;
                    }
                }
                StreamEvent::Error(message) => return Err(message),
                StreamEvent::Usage(_) => {}
                StreamEvent::Finish => break,
                _ => {}
            }
        }

        provider_task
            .await
            .map_err(|error| format!("summarizer task failed: {error}"))?
            .map_err(|error| error.to_string())?;

        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            return Err("empty summary".into());
        }
        Ok(truncate_chars(&trimmed, SUMMARY_MAX_CHARS))
    }
}

fn reconstruct_prompt(
    prior: Vec<ChatMessage>,
    current_user: Option<ChatMessage>,
    pending_tail: Vec<ChatMessage>,
) -> Vec<ChatMessage> {
    let mut messages = prior;
    if let Some(user) = current_user {
        messages.push(user);
    }
    messages.extend(pending_tail);
    messages
}

/// 为 Prompt 准备历史：超阈值时优先 LLM 摘要，失败则机械折叠。
///
/// Already-compacted turns are sliced from the last `<compaction-summary>` so
/// folded history is not sent (or counted) again.
pub async fn prepare_history_for_prompt(
    history: &[ChatMessage],
    context: &RequestContext,
    session_id: &str,
    context_window: usize,
    summarizer: Option<&dyn ConversationSummarizer>,
) -> CompactResult {
    let effective = history_from_last_summary(history);
    let (prior, current_user, pending_tail) = split_for_compact(effective);

    let estimated = estimate_history_tokens(&prior, context, current_user.as_ref());
    let usage_ratio = estimated as f32 / context_window.max(1) as f32;

    if usage_ratio >= COMPACT_TRIGGER_RATIO {
        if let Some(compacted) = compact_prior(&prior, session_id, summarizer).await {
            let mut messages = compacted.messages;
            if let Some(user) = current_user {
                messages.push(user);
            }
            messages.extend(pending_tail);
            let post = measure_context_usage(&messages, context, None, context_window);
            return CompactResult {
                messages,
                notice: Some(ContextNotice {
                    kind: ContextNoticeKind::Compacted,
                    usage_ratio: post.usage_ratio,
                    folded_messages: Some(compacted.folded_count),
                    estimated_tokens: post.estimated_tokens,
                    context_window_tokens: context_window,
                }),
            };
        }
    }

    CompactResult {
        messages: reconstruct_prompt(prior, current_user, pending_tail),
        notice: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextUsageMeasure {
    pub estimated_tokens: usize,
    pub usage_ratio: f32,
}

/// True when this row is an auto-compact summary (prompt slicing only; not shown in the thread).
pub fn is_compaction_summary(message: &ChatMessage) -> bool {
    message.id.starts_with("compact-") || message.content.contains("<compaction-summary>")
}

/// Slice history from the latest compaction summary (inclusive) so folded
/// turns are not counted or re-sent. The UI still shows the original thread.
pub fn history_from_last_summary(history: &[ChatMessage]) -> &[ChatMessage] {
    match history.iter().rposition(is_compaction_summary) {
        Some(idx) => &history[idx..],
        None => history,
    }
}

/// Estimate prompt token usage for the current session plus optional unsent draft.
pub fn measure_context_usage(
    history: &[ChatMessage],
    context: &RequestContext,
    draft_message: Option<&str>,
    context_window: usize,
) -> ContextUsageMeasure {
    let history = history_from_last_summary(history);
    let mut estimated = estimate_tokens(SYSTEM_PROMPT);
    estimated += estimate_context_tokens(context);

    for message in history {
        if !message_has_estimable_tokens(message) {
            continue;
        }
        estimated += estimate_message_tokens(message);
    }

    if let Some(draft) = draft_message.map(str::trim).filter(|text| !text.is_empty()) {
        let last_user = history
            .iter()
            .rev()
            .find(|message| message.role == Role::User);
        let already_counted = last_user.is_some_and(|message| message.content.trim() == draft);
        if !already_counted {
            estimated += estimate_tokens(draft);
        }
    }

    let usage_ratio = estimated as f32 / context_window.max(1) as f32;
    ContextUsageMeasure {
        estimated_tokens: estimated,
        usage_ratio,
    }
}

pub fn estimate_history_tokens(
    prior: &[ChatMessage],
    context: &RequestContext,
    current_user: Option<&ChatMessage>,
) -> usize {
    let mut total = estimate_tokens(SYSTEM_PROMPT);
    total += estimate_context_tokens(context);

    for message in prior {
        if !message_has_estimable_tokens(message) {
            continue;
        }
        total += estimate_message_tokens(message);
    }

    if let Some(user) = current_user {
        total += estimate_message_tokens(user);
    }

    total
}

fn message_has_estimable_tokens(message: &ChatMessage) -> bool {
    !message.content.trim().is_empty()
        || message
            .reasoning
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
        || message
            .tool_activities
            .as_ref()
            .is_some_and(|activities| !activities.is_empty())
        || message
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
}

fn estimate_context_tokens(context: &RequestContext) -> usize {
    let mut total = 0;
    for value in [
        &context.active_window,
        &context.active_file,
        &context.clipboard,
        &context.git_status,
        &context.last_shell_execution,
    ] {
        if let Some(text) = value.as_ref().filter(|text| !text.trim().is_empty()) {
            total += estimate_tokens(text) + 8;
        }
    }
    if let Some(workspace) = &context.workspace {
        total += estimate_tokens(&workspace.name) + estimate_tokens(&workspace.root) + 12;
    }
    if let Some(ide) = &context.ide_context {
        total += estimate_tokens(&ide.ide) + 8;
        if let Some(path) = &ide.workspace {
            total += estimate_tokens(&path.display().to_string()) + 4;
        }
        if let Some(path) = &ide.active_file {
            total += estimate_tokens(&path.display().to_string()) + 4;
        }
        if let Some(language) = &ide.language {
            total += estimate_tokens(language) + 4;
        }
        if let Some(selection) = &ide.selection {
            total += estimate_tokens(selection) + 4;
        }
        if ide.cursor.is_some() {
            total += 8;
        }
    }
    for file in &context.selected_files {
        if !file.trim().is_empty() {
            total += estimate_tokens(file) + 4;
        }
    }
    total
}

#[derive(Debug, Clone)]
pub struct CompactPriorResult {
    pub messages: Vec<ChatMessage>,
    pub folded_count: usize,
}

pub async fn compact_prior(
    prior: &[ChatMessage],
    session_id: &str,
    summarizer: Option<&dyn ConversationSummarizer>,
) -> Option<CompactPriorResult> {
    fold_compactable(prior, session_id, summarizer, true).await
}

/// Fold older in-flight messages after the current user turn (tool results
/// from this turn). Keeps the user message and a recent tail.
pub async fn compact_after_user(
    messages: &[ChatMessage],
    user_idx: usize,
    session_id: &str,
    summarizer: Option<&dyn ConversationSummarizer>,
) -> Option<CompactPriorResult> {
    if user_idx >= messages.len() {
        return None;
    }
    let after = &messages[user_idx + 1..];
    let folded_outcome = fold_compactable(after, session_id, summarizer, false).await?;
    let mut new_messages = messages[..=user_idx].to_vec();
    new_messages.extend(folded_outcome.messages);
    Some(CompactPriorResult {
        messages: new_messages,
        folded_count: folded_outcome.folded_count,
    })
}

async fn fold_compactable(
    messages: &[ChatMessage],
    session_id: &str,
    summarizer: Option<&dyn ConversationSummarizer>,
    fold_all_when_short: bool,
) -> Option<CompactPriorResult> {
    let compactable: Vec<_> = messages
        .iter()
        .filter(|message| is_compactable(message))
        .cloned()
        .collect();

    let keep_count = KEEP_TAIL_TURNS * 2;
    // Already over the 80% trigger: a handful of huge turns can fill 1M
    // without reaching 8+ messages. Fold the whole short prior into a summary
    // and keep only the caller's current user turn.
    // In-flight tool compact keeps a full tail and will not fold a short
    // tool loop (completion / mutation gates still need those rows).
    let keep = if compactable.len() > keep_count {
        keep_count
    } else if fold_all_when_short {
        0
    } else {
        return None;
    };
    if compactable.len() <= keep {
        return None;
    }

    let split_at = compactable.len() - keep;
    let folded = &compactable[..split_at];
    let kept = &compactable[split_at..];

    let summary = summarize_folded(folded, summarizer).await;
    if summary.is_empty() {
        return None;
    }

    let mut out = Vec::with_capacity(kept.len() + 1);
    out.push(summary_message(session_id, &summary, folded.len()));
    out.extend_from_slice(kept);

    Some(CompactPriorResult {
        messages: out,
        folded_count: folded.len(),
    })
}

async fn summarize_folded(
    folded: &[ChatMessage],
    summarizer: Option<&dyn ConversationSummarizer>,
) -> String {
    let mechanical = build_mechanical_summary(folded);
    let Some(summarizer) = summarizer else {
        return mechanical;
    };

    let payload = build_fold_payload(folded);
    let timed = timeout(
        Duration::from_secs(LLM_COMPACT_TIMEOUT_SECS),
        summarizer.summarize(&payload),
    )
    .await;

    match timed {
        Ok(Ok(summary)) if !summary.trim().is_empty() => summary,
        _ => mechanical,
    }
}

fn is_compactable(message: &ChatMessage) -> bool {
    if is_compaction_summary(message) || message.status == MessageStatus::Pending {
        return false;
    }
    matches!(message.role, Role::User | Role::Assistant | Role::Tool)
        && message_has_estimable_tokens(message)
}

fn build_mechanical_summary(messages: &[ChatMessage]) -> String {
    let mut lines = Vec::new();
    let mut total_chars = 0;

    for message in messages {
        let role = match message.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
            Role::System => continue,
        };
        let snippet = truncate_chars(message.content.trim(), SNIPPET_MAX_CHARS);
        if snippet.is_empty() {
            continue;
        }
        let line = format!("- {role}: {snippet}");
        total_chars += line.chars().count();
        if total_chars > SUMMARY_MAX_CHARS {
            lines.push("...".to_string());
            break;
        }
        lines.push(line);
    }

    lines.join("\n")
}

fn build_fold_payload(messages: &[ChatMessage]) -> String {
    let mut lines = Vec::new();
    let mut total_chars = 0usize;
    for message in messages {
        let role = match message.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
            Role::System => "System",
        };
        let snippet = truncate_chars(message.content.trim(), FOLD_PAYLOAD_MSG_MAX_CHARS);
        if snippet.is_empty() {
            continue;
        }
        let line = format!("### {role}\n{snippet}");
        let next = total_chars + line.chars().count() + 2;
        if next > FOLD_PAYLOAD_TOTAL_MAX_CHARS {
            lines.push("...".to_string());
            break;
        }
        total_chars = next;
        lines.push(line);
    }
    lines.join("\n\n")
}

fn summary_message(session_id: &str, body: &str, folded_count: usize) -> ChatMessage {
    ChatMessage {
        id: format!("compact-{session_id}-{}", folded_count),
        session_id: session_id.to_string(),
        role: Role::System,
        content: format!(
            "<compaction-summary>\nEarlier conversation ({folded_count} messages folded to save context):\n{body}\n</compaction-summary>"
        ),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
        estimated_tokens: None,
    }
}

/// Drop the trailing empty assistant message — the current turn's stream
/// target — from a history snapshot. Used by resume turns (approve & execute)
/// so the synthetic approval instruction becomes the final user turn instead
/// of landing after an empty assistant, which providers reject.
pub fn trim_empty_assistant_tail(mut history: Vec<ChatMessage>) -> Vec<ChatMessage> {
    if history
        .last()
        .is_some_and(|message| message.role == Role::Assistant && message.content.trim().is_empty())
    {
        history.pop();
    }
    history
}

fn split_for_compact(
    history: &[ChatMessage],
) -> (Vec<ChatMessage>, Option<ChatMessage>, Vec<ChatMessage>) {
    if history.is_empty() {
        return (Vec::new(), None, Vec::new());
    }

    let mut end = history.len();
    let mut pending_tail = Vec::new();
    while end > 0 {
        let message = &history[end - 1];
        let is_pending_assistant = message.role == Role::Assistant
            && message.content.trim().is_empty()
            && message.status == MessageStatus::Pending;
        if !is_pending_assistant {
            break;
        }
        pending_tail.insert(0, message.clone());
        end -= 1;
    }

    let rest = &history[..end];
    if rest.is_empty() {
        return (Vec::new(), None, pending_tail);
    }

    if rest
        .last()
        .is_some_and(|message| message.role == Role::User)
    {
        let current = rest.last().cloned();
        let prior = rest[..rest.len() - 1].to_vec();
        return (prior, current, pending_tail);
    }

    (rest.to_vec(), None, pending_tail)
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn notice_message(notice: &ContextNotice, language_zh: bool) -> String {
    let folded = notice.folded_messages.unwrap_or(0);
    if language_zh {
        format!("已自动压缩较早的 {folded} 条消息以节省上下文")
    } else {
        format!("Compacted {folded} earlier messages to save context")
    }
}

// Re-export for callers that previously used compact::estimate_tokens.
pub use limits::estimate_tokens;

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSummarizer {
        text: String,
        fail: bool,
    }

    #[async_trait]
    impl ConversationSummarizer for FakeSummarizer {
        async fn summarize(&self, _folded_payload: &str) -> Result<String, String> {
            if self.fail {
                Err("boom".into())
            } else {
                Ok(self.text.clone())
            }
        }
    }

    fn user_msg(id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            session_id: "s1".into(),
            role: Role::User,
            content: content.into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        }
    }

    fn assistant_msg(id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            session_id: "s1".into(),
            role: Role::Assistant,
            content: content.into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 2,
            estimated_tokens: None,
        }
    }

    #[test]
    fn resume_history_keeps_approval_as_final_user_turn() {
        // service.rs resume construction: the current turn's empty assistant
        // (stream target) must be trimmed before the synthetic approval
        // instruction is pushed, otherwise it lands between turns in the
        // prompt and providers reject the request.
        let mut history = vec![
            user_msg("u1", "重构登录模块"),
            assistant_msg("a1", "计划：1. 拆分 service 2. 加测试"),
            assistant_msg("a2", ""),
        ];
        let mut resume = trim_empty_assistant_tail(history.clone());
        resume.push(user_msg("u2", "计划已批准，现在执行"));

        assert_eq!(resume.len(), 3);
        assert_eq!(resume.last().unwrap().id, "u2");
        assert!(resume
            .iter()
            .all(|m| !(m.role == Role::Assistant && m.content.trim().is_empty())));

        let (prior, current, pending_tail) = split_for_compact(&resume);
        assert_eq!(current.unwrap().id, "u2");
        assert_eq!(prior.last().unwrap().id, "a1");
        assert!(pending_tail.is_empty());

        // The un-trimmed construction (the bug) leaks the empty assistant into
        // the prompt prior — guard the invariant at the split layer too.
        history.push(user_msg("u2", "计划已批准，现在执行"));
        let (prior, current, _) = split_for_compact(&history);
        assert_eq!(current.unwrap().id, "u2");
        assert!(prior
            .iter()
            .any(|m| m.role == Role::Assistant && m.content.trim().is_empty()));
    }

    #[test]
    fn trim_only_removes_empty_assistant_tail() {
        let with_content = vec![assistant_msg("a1", "内容")];
        assert_eq!(trim_empty_assistant_tail(with_content).len(), 1);

        let user_tail = vec![user_msg("u1", "hi"), assistant_msg("a1", "回复")];
        assert_eq!(trim_empty_assistant_tail(user_tail).len(), 2);

        let empty_tail = vec![user_msg("u1", "hi"), assistant_msg("a1", "")];
        let trimmed = trim_empty_assistant_tail(empty_tail);
        assert_eq!(trimmed.len(), 1);
        assert_eq!(trimmed[0].id, "u1");
    }

    #[tokio::test]
    async fn compacts_when_usage_exceeds_threshold() {
        let big = "word ".repeat(20_000);
        let mut history = Vec::new();
        for index in 0..8 {
            history.push(user_msg(&format!("u{index}"), &big));
            history.push(assistant_msg(&format!("a{index}"), &big));
        }
        history.push(user_msg("current", "latest question"));

        let result =
            prepare_history_for_prompt(&history, &RequestContext::default(), "s1", 8_000, None)
                .await;
        assert_eq!(
            result.notice.as_ref().map(|n| n.kind),
            Some(ContextNoticeKind::Compacted)
        );
        assert!(result.messages.iter().any(|m| m.id.starts_with("compact-")));
        assert!(result.messages.last().is_some_and(|m| m.id == "current"));
    }

    #[tokio::test]
    async fn llm_summary_preferred_when_available() {
        let big = "word ".repeat(20_000);
        let mut history = Vec::new();
        for index in 0..8 {
            history.push(user_msg(&format!("u{index}"), &big));
            history.push(assistant_msg(&format!("a{index}"), &big));
        }
        history.push(user_msg("current", "latest question"));

        let summarizer = FakeSummarizer {
            text: "LLM compact summary about the earlier work.".into(),
            fail: false,
        };
        let result = prepare_history_for_prompt(
            &history,
            &RequestContext::default(),
            "s1",
            8_000,
            Some(&summarizer),
        )
        .await;
        let summary = result
            .messages
            .iter()
            .find(|m| m.id.starts_with("compact-"))
            .expect("summary");
        assert!(summary.content.contains("LLM compact summary"));
    }

    #[tokio::test]
    async fn falls_back_to_mechanical_when_llm_fails() {
        let big = "word ".repeat(20_000);
        let mut history = Vec::new();
        for index in 0..8 {
            history.push(user_msg(&format!("u{index}"), &big));
            history.push(assistant_msg(&format!("a{index}"), &big));
        }
        history.push(user_msg("current", "latest question"));

        let summarizer = FakeSummarizer {
            text: String::new(),
            fail: true,
        };
        let result = prepare_history_for_prompt(
            &history,
            &RequestContext::default(),
            "s1",
            8_000,
            Some(&summarizer),
        )
        .await;
        let summary = result
            .messages
            .iter()
            .find(|m| m.id.starts_with("compact-"))
            .expect("summary");
        assert!(summary.content.contains("- User:"));
    }

    #[tokio::test]
    async fn stays_quiet_below_compact_threshold() {
        let medium = "x".repeat(9_000);
        let history = vec![
            user_msg("u1", &medium),
            assistant_msg("a1", &medium),
            user_msg("current", "hi"),
        ];
        let estimated =
            estimate_history_tokens(&history[..2], &RequestContext::default(), history.last());
        let window = ((estimated as f32) / 0.74).round().max(1.0) as usize;
        assert!((estimated as f32 / window as f32) < COMPACT_TRIGGER_RATIO);

        let result =
            prepare_history_for_prompt(&history, &RequestContext::default(), "s1", window, None)
                .await;
        assert!(result.notice.is_none());
        assert!(!result.messages.iter().any(|m| m.id.starts_with("compact-")));
    }

    #[test]
    fn estimate_tokens_is_nonzero_for_text() {
        assert!(estimate_tokens("hello world") > 0);
    }

    #[tokio::test]
    async fn compacts_short_thread_when_tokens_exceed_window() {
        // Two huge prior turns used to skip compact (needed 9+ compactable
        // messages). A 1M window can fill from a handful of tool-heavy turns.
        let big = "word ".repeat(20_000);
        let history = vec![
            user_msg("u0", &big),
            assistant_msg("a0", &big),
            user_msg("u1", &big),
            assistant_msg("a1", &big),
            user_msg("current", "latest question"),
        ];

        let result =
            prepare_history_for_prompt(&history, &RequestContext::default(), "s1", 8_000, None)
                .await;
        assert_eq!(
            result.notice.as_ref().map(|n| n.kind),
            Some(ContextNoticeKind::Compacted)
        );
        assert!(result.messages.iter().any(|m| m.id.starts_with("compact-")));
        let post = measure_context_usage(&result.messages, &RequestContext::default(), None, 8_000);
        let pre = measure_context_usage(&history, &RequestContext::default(), None, 8_000);
        assert!(post.estimated_tokens < pre.estimated_tokens);
    }

    fn tool_msg(id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            session_id: "s1".into(),
            role: Role::Tool,
            content: content.into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: Some("c1".into()),
            name: Some("read_file".into()),
            status: MessageStatus::Done,
            timestamp: 3,
            estimated_tokens: None,
        }
    }

    #[tokio::test]
    async fn folds_tool_results_after_current_user() {
        let big = "word ".repeat(8_000);
        let mut messages = vec![user_msg("u0", "please inspect the repo")];
        for index in 0..6 {
            messages.push(assistant_msg(&format!("a{index}"), "calling tool"));
            messages.push(tool_msg(&format!("t{index}"), &big));
        }

        let result = compact_after_user(&messages, 0, "s1", None)
            .await
            .expect("should fold in-flight tool output");
        assert!(result.folded_count >= 2);
        assert_eq!(result.messages[0].id, "u0");
        assert!(result.messages.iter().any(|m| m.id.starts_with("compact-")));
    }

    #[test]
    fn usage_meter_starts_from_last_summary() {
        let big = "x".repeat(8_000);
        let history = vec![
            user_msg("u0", &big),
            assistant_msg("a0", &big),
            summary_message("s1", "earlier work folded", 2),
            user_msg("u1", "follow up"),
        ];
        let with_fold = measure_context_usage(&history, &RequestContext::default(), None, 64_000);
        let without_old =
            measure_context_usage(&history[2..], &RequestContext::default(), None, 64_000);
        assert_eq!(with_fold.estimated_tokens, without_old.estimated_tokens);
        let mut raw_all = estimate_tokens(SYSTEM_PROMPT);
        for message in &history {
            raw_all += estimate_message_tokens(message);
        }
        assert!(with_fold.estimated_tokens < raw_all);
    }
}
