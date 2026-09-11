//! Clean, concise conversation title generation and normalization.

use std::sync::Arc;
use tauri::async_runtime;
use tokio::sync::mpsc;

use crate::core::ai::provider::AIProvider;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::limits::truncate_chars;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent};

pub const FALLBACK_MAX_WORDS: usize = 10;
pub const FALLBACK_MAX_BYTES: usize = 160;
pub const MAX_TITLE_BYTES: usize = 160;
pub const AI_TITLE_MAX_CHARS: usize = 48;

const TITLE_SYSTEM_PROMPT: &str = "\
You are an expert conversation title generator. Your only job is to create a concise, accurate title summarizing the user's primary goal or task in the conversation.\n\
\n\
RULES:\n\
1. Focus strictly on what the user wants to accomplish (e.g., feature to build, bug to fix, or topic asked). Never title after side tasks, debug steps, or minor follow-up questions.\n\
2. Reply with ONLY the title in plain text. Absolutely NO quotes, NO markdown formatting (no bold/backticks/headers), NO emoji, and NO trailing punctuation.\n\
3. Never include conversational filler or prefixes (e.g., never say 'Title:', '标题：', '好的', 'Here is', etc.).\n\
4. If the conversation is in Chinese, generate a concise Chinese title of 4 to 10 characters (e.g. 'Vite配置优化', '用户登录模块重构', 'Docker端口冲突排查'). If in English or another language, use 3 to 6 words.\n\
5. Avoid vague generic labels (e.g. do not output '代码修改', 'Bug修复', '问题咨询', '查看报错').";

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
        '\u{0000}'..='\u{0008}' | '\u{000B}' | '\u{000C}' | '\u{000E}'..='\u{001F}' | '\u{007F}'..='\u{009F}'
            | '\u{200B}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}' | '\u{FEFF}'
    )
}

fn clean_title_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = true;
    for ch in input.chars() {
        if ch.is_whitespace() || is_control_char(ch) {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
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
    truncate_title_utf8(&clean_title_text(input), max_bytes)
        .trim_end()
        .to_string()
}

/// Clean up and extract title from raw AI output.
pub fn clean_ai_title(value: &str) -> String {
    let mut text = value.trim();

    // Strip code fence if enclosed
    if text.starts_with("```") {
        if let Some(end) = text[3..].find("```") {
            text = text[3..3 + end].trim();
        }
    }

    // Strip markdown headers (#, ##)
    while text.starts_with('#') {
        text = text.trim_start_matches('#').trim_start();
    }

    // Strip list markers ("- ", "* ", "1. ")
    if let Some(rest) = text.strip_prefix("- ").or_else(|| text.strip_prefix("* ")) {
        text = rest.trim_start();
    } else if let Some(idx) = text.find(". ") {
        if idx <= 3 && text[..idx].chars().all(|c| c.is_ascii_digit()) {
            text = text[idx + 2..].trim_start();
        }
    }

    // Strip conversational prefixes
    let prefixes = [
        "title:",
        "title：",
        "标题:",
        "标题：",
        "会话标题:",
        "会话标题：",
        "topic:",
        "topic：",
        "主题:",
        "主题：",
        "好的，为您生成的标题是：",
        "好的，为你生成的标题是：",
        "为您生成的标题是：",
        "为你生成的标题是：",
        "建议标题：",
        "建议标题:",
        "here is the title:",
        "suggested title:",
        "title is:",
    ];
    while let Some(prefix) = prefixes
        .iter()
        .find(|p| text.to_lowercase().starts_with(**p))
    {
        text = text[prefix.len()..].trim_start();
    }

    // Strip inline backticks and markdown delimiters
    let mut s = text.replace('`', "");
    for delim in ["**", "*", "__", "_"] {
        if s.starts_with(delim) && s.ends_with(delim) && s.len() >= delim.len() * 2 {
            s = s[delim.len()..s.len() - delim.len()].trim().to_string();
        }
    }

    // Strip outer quotes
    for prefix in ['"', '\'', '「', '『', '《', '“', '‘'] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.trim_start().to_string();
            break;
        }
    }
    const TRAILING: &[char] = &[
        '"', '\'', '」', '』', '》', '”', '’', '.', '。', '!', '！', '?', '？', ':', '：', ';',
        '；',
    ];
    for suffix in TRAILING {
        if let Some(rest) = s.strip_suffix(*suffix) {
            s = rest.trim_end().to_string();
            break;
        }
    }

    clean_title_text(&s)
}

fn is_conversational_filler(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    const FILLERS: &[&str] = &[
        "好的",
        "为你生成",
        "为您生成",
        "建议标题",
        "根据对话",
        "here is",
        "sure,",
        "sure!",
        "certainly",
        "the title",
        "i suggest",
    ];
    FILLERS.iter().any(|f| lower.starts_with(f))
}

fn is_vague_title(title: &str) -> bool {
    let cleaned = clean_title_text(title);
    let compact: String = cleaned
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    const VAGUE_TITLES: &[&str] = &[
        "代码修改",
        "修改代码",
        "bug修复",
        "修复bug",
        "问题咨询",
        "查看报错",
        "解决报错",
        "排查报错",
        "日常问候",
        "感谢交流",
        "技术支持",
        "代码分析",
        "fixbug",
        "fixissue",
        "codereview",
        "generalquestion",
        "使用子agent",
        "阅读代码",
        "readcode",
        "readcodebase",
    ];
    VAGUE_TITLES
        .iter()
        .any(|vague| compact.eq_ignore_ascii_case(vague))
}

fn pick_generated_title(content: &str, reasoning: &str) -> String {
    let check_line = |line: &str| -> Option<String> {
        if is_conversational_filler(line) {
            return None;
        }
        let cleaned = clean_ai_title(line);
        if !cleaned.is_empty()
            && cleaned.chars().count() <= AI_TITLE_MAX_CHARS * 2
            && !is_vague_title(&cleaned)
        {
            Some(cleaned)
        } else {
            None
        }
    };

    for line in content.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(title) = check_line(line) {
            return title;
        }
    }

    let from_content = clean_ai_title(content);
    if !from_content.is_empty() && !is_vague_title(&from_content) {
        return from_content;
    }

    for line in reasoning
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .rev()
    {
        if let Some(title) = check_line(line) {
            return title;
        }
    }

    String::new()
}

/// Fallback title extracted cleanly from the user's primary prompt.
pub fn fallback_session_title(input: &str, max_words: usize, max_bytes: usize) -> String {
    let cleaned = clean_title_text(input);
    if cleaned.is_empty() {
        return String::new();
    }

    let mut subject = cleaned.as_str();
    for prefix in [
        "请帮我",
        "请问",
        "麻烦帮我",
        "帮我",
        "麻烦",
        "请",
        "could you please ",
        "please ",
        "help me ",
    ] {
        if let Some(rest) = subject.strip_prefix(prefix) {
            subject = rest.trim_start();
            break;
        }
    }

    let first_sentence = subject
        .split(&['。', '！', '？', '!', '?', '\n'][..])
        .next()
        .unwrap_or(subject)
        .trim();

    let title: String = if first_sentence
        .chars()
        .any(|c| c >= '\u{4E00}' && c <= '\u{9FFF}')
    {
        let first_clause = first_sentence
            .split(&['，', ',', '；', ';'][..])
            .next()
            .unwrap_or(first_sentence)
            .trim();
        if first_clause.chars().count() >= 4 && first_clause.chars().count() <= 28 {
            first_clause.to_string()
        } else {
            first_sentence.chars().take(28).collect()
        }
    } else {
        first_sentence
            .split_whitespace()
            .take(max_words)
            .collect::<Vec<_>>()
            .join(" ")
    };

    truncate_title_utf8(&title, max_bytes)
        .trim_end()
        .to_string()
}

fn smart_truncate_title(title: &str, max_chars: usize) -> String {
    if title.chars().count() <= max_chars {
        return title.to_string();
    }
    let truncated = truncate_chars(title, max_chars);
    if truncated.contains(' ') {
        if let Some((head, _)) = truncated.rsplit_once(' ') {
            if head.chars().count() >= max_chars / 2 {
                return head.trim_end().to_string();
            }
        }
    }
    truncated
}

fn finalize_generated_title(
    content: &str,
    reasoning: &str,
    fallback_prompt: &str,
) -> Result<String, String> {
    let mut title = pick_generated_title(content, reasoning);
    if title.is_empty() || is_vague_title(&title) {
        title = fallback_session_title(fallback_prompt, FALLBACK_MAX_WORDS, FALLBACK_MAX_BYTES);
    }
    if title.is_empty() {
        return Err("empty title".into());
    }
    let normalized = normalize_session_title(&title, MAX_TITLE_BYTES);
    if normalized.is_empty() {
        return Err("empty title".into());
    }
    Ok(smart_truncate_title(&normalized, AI_TITLE_MAX_CHARS))
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Schedule asynchronous LLM title generation on first turn.
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
        match generate_session_title(provider, &trimmed, &trimmed).await {
            Ok(title) => {
                conversation.set_session_title(
                    &session_id,
                    title.clone(),
                    SessionTitleSource::Auto,
                );
                event_bus.emit(BusEvent::ChatSessionTitleUpdated { session_id, title });
            }
            Err(error) => eprintln!("failed to generate session title: {error}"),
        }
    });
}

fn simple_chat_message(role: Role, content: String) -> ChatMessage {
    ChatMessage {
        id: format!("title-{role:?}"),
        session_id: "title".into(),
        role,
        content,
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

/// Generate a short, accurate title from conversation context (awaits provider stream).
pub async fn generate_session_title(
    provider: Arc<dyn AIProvider>,
    context_text: &str,
    fallback_prompt: &str,
) -> Result<String, String> {
    let prompt_content = format!(
        "<conversation>\n{}\n</conversation>\n\n\
        Generate a concise title summarizing the PRIMARY GOAL of the above conversation. Plain text only, no quotes, no punctuation.",
        truncate_chars(context_text.trim(), 1200)
    );

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(16);
    let request = ChatRequest {
        request_id: format!("title-{}", now_millis()),
        session_id: "title".to_string(),
        messages: vec![
            simple_chat_message(Role::System, TITLE_SYSTEM_PROMPT.into()),
            simple_chat_message(Role::User, prompt_content),
        ],
        context: Default::default(),
        provider: Some(provider.id().to_string()),
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: Some(0.2),
        max_tokens: Some(256),
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

    finalize_generated_title(&content, &reasoning, fallback_prompt)
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
    fn fallback_strips_polite_prefixes() {
        assert_eq!(
            fallback_session_title("请帮我实现一个登录界面", 5, 80),
            "实现一个登录界面"
        );
    }

    #[test]
    fn clean_ai_title_strips_markdown_and_prefixes() {
        assert_eq!(
            clean_ai_title("**Vite 热更新失效排查**"),
            "Vite 热更新失效排查"
        );
        assert_eq!(
            clean_ai_title("Title: Docker 端口冲突解决."),
            "Docker 端口冲突解决"
        );
        assert_eq!(
            clean_ai_title("标题：工作区重命名实现！"),
            "工作区重命名实现"
        );
        assert_eq!(
            clean_ai_title("好的，为您生成的标题是：**用户登录逻辑重构**"),
            "用户登录逻辑重构"
        );
        assert_eq!(
            clean_ai_title("# `useWorkbenchSessions` 重构"),
            "useWorkbenchSessions 重构"
        );
    }

    #[test]
    fn pick_and_finalize_titles() {
        let content = "好的，为你推荐以下会话标题：\n\n**Vue3 路由守卫配置**";
        assert_eq!(pick_generated_title(content, ""), "Vue3 路由守卫配置");
        assert_eq!(
            finalize_generated_title("代码修改", "", "修复登录按钮点击无效").unwrap(),
            "修复登录按钮点击无效"
        );
    }

    #[test]
    fn smart_truncate_and_clause_fallback() {
        assert_eq!(
            smart_truncate_title("Configure automated GitHub Actions workflow", 30),
            "Configure automated GitHub"
        );
        assert_eq!(
            fallback_session_title("实现用户登录功能，并且加上记住密码选项", 8, 160),
            "实现用户登录功能"
        );
    }

    #[test]
    fn source_round_trip() {
        assert_eq!(
            SessionTitleSource::parse("user"),
            Some(SessionTitleSource::User)
        );
        assert_eq!(
            SessionTitleSource::parse("AUTO"),
            Some(SessionTitleSource::Auto)
        );
        assert_eq!(SessionTitleSource::parse("nope"), None);
    }
}
