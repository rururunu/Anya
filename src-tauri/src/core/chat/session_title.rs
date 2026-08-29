//! Session title normalization, deterministic fallback, and async LLM titles.

use std::sync::Arc;

use tauri::async_runtime;
use tokio::sync::mpsc;

use crate::core::ai::provider::AIProvider;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::limits::truncate_chars;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent,
};

pub const FALLBACK_MAX_WORDS: usize = 8;
pub const FALLBACK_MAX_BYTES: usize = 80;
pub const MAX_TITLE_BYTES: usize = 80;
pub const AI_TITLE_MAX_CHARS: usize = 24;

const CJK_SOFT_BREAKS: &[char] = &['和', '与', '及', '、', '或'];

const CLAUSE_SEPARATORS: &[char] = &['，', ',', '。', '.', '；', ';', '\n'];

const DELIVERABLE_MARKERS: &[&str] = &[
    "请给我",
    "请帮我",
    "给我",
    "帮我",
    "想要",
    "需要",
    "输出",
    "生成",
    "总结",
    "分析",
    "could you ",
    "can you ",
    "help me ",
    "give me ",
    "i need ",
    "i want ",
    "output ",
    "generate ",
    "summarize ",
    "analyze ",
];

const PROCEDURAL_PREFIXES: &[&str] = &[
    "请帮我",
    "请给我",
    "请",
    "帮我",
    "给我",
    "麻烦",
    "能否",
    "可以",
    "想要",
    "需要",
    "please ",
    "please",
    "could you ",
    "can you ",
    "help me ",
    "give me ",
];

const METHODOLOGY_HINTS: &[&str] = &[
    "使用子agent",
    "使用子 agent",
    "用子agent",
    "用子 agent",
    "subagent",
    "sub-agent",
    "sub agent",
    "阅读代码",
    "读代码",
    "阅读这个项目的代码",
    "read the code",
    "read codebase",
    "read the codebase",
    "并行探索",
    "并行阅读",
    "use subagent",
    "using subagent",
];

const TITLE_SYSTEM_PROMPT: &str = "You create a very short conversation title that names the user's desired outcome (topic, deliverable, or task result)—NOT the method they use to get there.\n\
Reply with ONLY the title: plain text, no quotes, no trailing punctuation, no explanation, no emoji.\n\
Never title with process words alone (e.g. \"use subagent\", \"read code\", \"使用子agent\", \"阅读代码\").\n\
Keep it to 2-6 words (under 24 characters). If the conversation is not in English, reply in the same language as the user's message.";

/// How a session title was produced. `User` pins the title against automatic updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTitleSource {
    Fallback,
    Auto,
    User,
}

impl SessionTitleSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fallback => "fallback",
            Self::Auto => "auto",
            Self::User => "user",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "fallback" => Some(Self::Fallback),
            "auto" => Some(Self::Auto),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

fn is_control_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{0000}'..='\u{0008}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000E}'..='\u{001F}'
            | '\u{007F}'..='\u{009F}'
    ) || matches!(
        ch,
        '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
    )
}

fn clean_title_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = true;
    for ch in input.chars() {
        if ch == '\n' || ch == '\t' || ch == '\r' {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        if is_control_char(ch) {
            continue;
        }
        out.push(ch);
        prev_space = false;
    }
    out.trim().to_string()
}

/// Truncate to a UTF-8 byte budget without splitting a code point.
pub fn truncate_title_utf8(input: &str, max_bytes: usize) -> String {
    if max_bytes == 0 {
        return String::new();
    }
    if input.len() <= max_bytes {
        return input.to_string();
    }
    let mut used = 0usize;
    let mut output = String::new();
    for ch in input.chars() {
        let bytes = ch.len_utf8();
        if used + bytes > max_bytes {
            break;
        }
        output.push(ch);
        used += bytes;
    }
    output
}

/// Normalize accepted title text and enforce the UTF-8 byte budget.
pub fn normalize_session_title(input: &str, max_bytes: usize) -> String {
    truncate_title_utf8(&clean_title_text(input), max_bytes).trim_end().to_string()
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch,
            '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{3040}'..='\u{30FF}'
                | '\u{AC00}'..='\u{D7AF}'
        )
    })
}

fn is_methodology_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    METHODOLOGY_HINTS
        .iter()
        .any(|hint| lower.contains(&hint.to_lowercase()))
}

fn split_clauses(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if CLAUSE_SEPARATORS.contains(&ch) {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                clauses.push(trimmed);
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        clauses.push(trimmed);
    }
    if clauses.is_empty() {
        clauses.push(text.to_string());
    }
    clauses
}

fn strip_deliverable_marker(text: &str) -> Option<String> {
    let trimmed = text.trim();
    for marker in DELIVERABLE_MARKERS {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn strip_procedural_prefix(text: &str) -> String {
    let mut subject = text.trim().to_string();
    loop {
        let mut changed = false;
        for prefix in PROCEDURAL_PREFIXES {
            if let Some(rest) = subject.strip_prefix(prefix) {
                subject = rest.trim_start().to_string();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    for prefix in ["这个", "该", "the ", "this ", "a ", "an "] {
        if let Some(rest) = subject.strip_prefix(prefix) {
            subject = rest.trim_start().to_string();
            break;
        }
    }
    subject
}

/// Pull a deliverable-focused subject from user text (skip leading process clauses).
fn extract_title_subject(input: &str) -> String {
    let cleaned = clean_title_text(input);
    if cleaned.is_empty() {
        return cleaned;
    }

    let clauses = split_clauses(&cleaned);
    for clause in clauses.iter().rev() {
        if is_methodology_text(clause) {
            continue;
        }
        if let Some(after_marker) = strip_deliverable_marker(clause) {
            let subject = strip_procedural_prefix(&after_marker);
            if !subject.is_empty() && !is_methodology_text(&subject) {
                return subject;
            }
        }
        let subject = strip_procedural_prefix(clause);
        if !subject.is_empty() && !is_methodology_text(&subject) {
            return subject;
        }
    }

    let subject = strip_procedural_prefix(&cleaned);
    if !subject.is_empty() && !is_methodology_text(&subject) {
        return subject;
    }

    cleaned
}

fn truncate_cjk_at_soft_break(subject: &str, max_chars: usize, max_bytes: usize) -> Option<String> {
    let chars: Vec<char> = subject.chars().collect();
    if chars.len() <= max_chars {
        return None;
    }

    let mut best: Option<String> = None;
    for (index, ch) in chars.iter().enumerate() {
        if index >= max_chars {
            break;
        }
        if CJK_SOFT_BREAKS.contains(ch) && index >= 3 {
            let candidate: String = chars[..=index].iter().collect();
            let trimmed = truncate_title_utf8(&candidate, max_bytes);
            if trimmed.chars().count() >= 4 {
                best = Some(trimmed);
            }
        }
    }
    best
}

fn truncate_subject_cjk(subject: &str, max_chars: usize, max_bytes: usize) -> String {
    if subject.chars().count() <= max_chars && subject.len() <= max_bytes {
        return subject.to_string();
    }
    if let Some(at_break) = truncate_cjk_at_soft_break(subject, max_chars, max_bytes) {
        return at_break;
    }
    let title: String = subject.chars().take(max_chars).collect();
    truncate_title_utf8(&title, max_bytes).trim_end().to_string()
}

fn truncate_subject_to_budget(subject: &str, max_words: usize, max_bytes: usize) -> String {
    if subject.is_empty() {
        return String::new();
    }
    let title = if contains_cjk(subject) {
        let max_chars = if max_words <= FALLBACK_MAX_WORDS {
            // Tight budget (unit tests): honor max_words as a char cap.
            max_words.max(4)
        } else {
            AI_TITLE_MAX_CHARS
        };
        return truncate_subject_cjk(subject, max_chars, max_bytes);
    } else {
        subject
            .split(' ')
            .filter(|word| !word.is_empty())
            .take(max_words)
            .collect::<Vec<_>>()
            .join(" ")
    };
    truncate_title_utf8(&title, max_bytes).trim_end().to_string()
}

/// Deterministic first-prompt fallback from visible user text.
pub fn fallback_session_title(
    input: &str,
    max_words: usize,
    max_bytes: usize,
) -> String {
    let subject = extract_title_subject(input);
    if contains_cjk(&subject) {
        let max_chars = AI_TITLE_MAX_CHARS;
        return truncate_subject_cjk(&subject, max_chars, max_bytes);
    }
    truncate_subject_to_budget(&subject, max_words, max_bytes)
}

fn clean_ai_title(value: &str) -> String {
    let mut cleaned = value.trim().to_string();
    for prefix in ['"', '\'', '「', '『', '《', '“', '‘'] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            cleaned = rest.trim_start().to_string();
            break;
        }
    }
    for suffix in [
        '"', '\'', '」', '』', '》', '”', '’', '.', '。', '!', '！', '?', '？', ':',
    ] {
        if let Some(rest) = cleaned.strip_suffix(suffix) {
            cleaned = rest.trim_end().to_string();
            break;
        }
    }
    clean_title_text(&cleaned)
}

fn pick_generated_title(content: &str, reasoning: &str) -> String {
    let from_content = clean_ai_title(content);
    if !from_content.is_empty() {
        return from_content;
    }

    if reasoning.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<String> = reasoning
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(clean_ai_title)
        .filter(|line| !line.is_empty())
        .collect();
    for line in lines.iter().rev() {
        if line.chars().count() <= AI_TITLE_MAX_CHARS * 2 {
            return line.clone();
        }
    }

    let from_reasoning = clean_ai_title(reasoning);
    if from_reasoning.is_empty() {
        return String::new();
    }

    if from_reasoning.chars().count() <= AI_TITLE_MAX_CHARS * 2 {
        return from_reasoning;
    }

    String::new()
}

fn finalize_generated_title(
    content: &str,
    reasoning: &str,
    user_text: &str,
) -> Result<String, String> {
    let mut title = pick_generated_title(content, reasoning);
    if title.is_empty() || is_methodology_text(&title) {
        title = fallback_session_title(user_text, FALLBACK_MAX_WORDS, FALLBACK_MAX_BYTES);
    }
    if title.is_empty() {
        return Err("empty title".into());
    }
    let normalized = normalize_session_title(&title, MAX_TITLE_BYTES);
    if normalized.is_empty() {
        return Err("empty title".into());
    }
    Ok(truncate_chars(&normalized, AI_TITLE_MAX_CHARS))
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Schedule asynchronous LLM title generation (first-prompt only unless `force`).
pub fn spawn_auto_session_title(
    conversation: Arc<ConversationManager>,
    event_bus: Arc<dyn EventBus>,
    provider: Arc<dyn AIProvider>,
    session_id: String,
    first_user: String,
    force: bool,
) {
    let trimmed = first_user.trim().to_string();
    if trimmed.is_empty() {
        return;
    }
    if !force {
        let user_turn_count = conversation
            .messages(&session_id)
            .iter()
            .filter(|message| message.role == Role::User)
            .count();
        if user_turn_count != 1 || !conversation.can_auto_update_title(&session_id) {
            return;
        }
    }

    async_runtime::spawn(async move {
        match generate_session_title(provider, &trimmed).await {
            Ok(title) => {
                conversation.set_session_title(&session_id, title.clone(), SessionTitleSource::Auto);
                event_bus.emit(BusEvent::ChatSessionTitleUpdated { session_id, title });
            }
            Err(error) => eprintln!("failed to generate session title: {error}"),
        }
    });
}

/// Generate a short title from the first user message (awaits the provider stream).
pub async fn generate_session_title(
    provider: Arc<dyn AIProvider>,
    first_user: &str,
) -> Result<String, String> {
    let material = format!("User: {}", truncate_chars(first_user, 600));

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(16);
    let request = ChatRequest {
        request_id: format!("title-{}", now_millis()),
        session_id: "title".to_string(),
        messages: vec![
            ChatMessage {
                id: "title-system".into(),
                session_id: "title".into(),
                role: Role::System,
                content: TITLE_SYSTEM_PROMPT.into(),
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
                id: "title-user".into(),
                session_id: "title".into(),
                role: Role::User,
                content: material,
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
        context: Default::default(),
        provider: Some(provider.id().to_string()),
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: Some(0.2),
        max_tokens: Some(64),
    };

    let provider_task = async_runtime::spawn(async move { provider.stream(request, tx).await });

    let mut content = String::new();
    let mut reasoning = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Delta(delta) => content.push_str(&delta),
            StreamEvent::Reasoning(chunk) => reasoning.push_str(&chunk),
            StreamEvent::TurnComplete {
                content: turn_content,
                reasoning: turn_reasoning,
                ..
            } => {
                if !turn_content.is_empty() {
                    content = turn_content;
                }
                if let Some(value) = turn_reasoning.filter(|value| !value.is_empty()) {
                    reasoning = value;
                }
            }
            StreamEvent::Error(message) => return Err(message),
            _ => {}
        }
    }
    provider_task
        .await
        .map_err(|error| format!("title task failed: {error}"))?
        .map_err(|error| error.to_string())?;

    finalize_generated_title(&content, &reasoning, first_user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_controls_and_collapses_whitespace() {
        assert_eq!(
            normalize_session_title("  Hello\t brave\nnew world  ", MAX_TITLE_BYTES),
            "Hello brave new world"
        );
    }

    #[test]
    fn fallback_limits_words_and_bytes() {
        assert_eq!(
            fallback_session_title("one two three four five six seven eight nine", 3, 80),
            "one two three"
        );
        assert_eq!(fallback_session_title("你好世界测试标题", 4, 7), "你好");
    }

    #[test]
    fn fallback_prefers_deliverable_over_method_clause() {
        assert_eq!(
            fallback_session_title(
                "使用子agent阅读这个项目的代码，给我这个项目设计思路和架构图",
                FALLBACK_MAX_WORDS,
                FALLBACK_MAX_BYTES,
            ),
            "项目设计思路和架构图"
        );
    }

    #[test]
    fn fallback_cjk_soft_break_avoids_mid_word_cut() {
        let subject = "项目设计思路和架构图以及部署方案说明";
        let truncated = truncate_subject_cjk(subject, 8, FALLBACK_MAX_BYTES);
        assert_eq!(truncated, "项目设计思路和");
        assert!(!truncated.ends_with('架'));
    }

    #[test]
    fn extract_title_subject_skips_methodology_clause() {
        assert_eq!(
            extract_title_subject(
                "Use subagents to read the codebase, give me project architecture diagram"
            ),
            "project architecture diagram"
        );
    }

    #[test]
    fn finalize_rejects_methodology_llm_title() {
        assert_eq!(
            finalize_generated_title("使用子agent", "", "如何修复登录崩溃问题").unwrap(),
            "如何修复登录崩溃问题"
        );
    }

    #[test]
    fn source_round_trip() {
        assert_eq!(SessionTitleSource::parse("user"), Some(SessionTitleSource::User));
        assert_eq!(SessionTitleSource::parse("AUTO"), Some(SessionTitleSource::Auto));
        assert_eq!(SessionTitleSource::parse("nope"), None);
    }

    #[test]
    fn pick_generated_title_prefers_content_then_reasoning() {
        assert_eq!(
            pick_generated_title("Fix login bug", "long internal reasoning"),
            "Fix login bug"
        );
        assert_eq!(
            pick_generated_title("", "First line\nFix login bug"),
            "Fix login bug"
        );
    }

    #[test]
    fn finalize_generated_title_falls_back_to_user_text() {
        assert_eq!(
            finalize_generated_title("", "", "如何修复登录崩溃问题").unwrap(),
            "如何修复登录崩溃问题"
        );
    }
}
