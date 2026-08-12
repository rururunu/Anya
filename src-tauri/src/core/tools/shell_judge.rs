//! Model-based completion judgment for shell jobs.
//!
//! A shell command's direct child can exit while a descendant (Gradle daemon,
//! detached worker, service) still holds the output pipe, so EOF never
//! arrives. Instead of hanging on EOF or guessing with a fixed delay, the
//! accumulated output is handed to the model, which decides whether the task
//! actually finished. Every failure path degrades to
//! [`CompletionVerdict::Unknown`].

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::core::ai::provider::AIProvider;
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, RequestContext, Role, StreamEvent,
};
use crate::core::tools::context::ToolContext;

const JUDGE_TIMEOUT: Duration = Duration::from_secs(20);
const JUDGE_MAX_TOKENS: u32 = 8;
const JUDGE_OUTPUT_TAIL_CHARS: usize = 6000;

const SYSTEM_PROMPT: &str = "You are a process-completion detector for command-line tools. \
Given a command and its latest output, decide whether the task has finished running. \
Reply with exactly one token and nothing else: FINISHED when the output shows the task \
completed (a result summary, build result, final status, or a returned prompt); RUNNING \
when the output shows work still in progress or an expected long-lived process; UNKNOWN \
when the output is empty, truncated, or ambiguous.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionVerdict {
    /// Output shows the task completed (success or failure).
    Finished,
    /// Output shows work still in progress.
    Running,
    /// No provider, empty/ambiguous output, or the model call failed.
    Unknown,
}

/// Ask the model whether `command` has finished, given its latest output.
/// Bounded by [`JUDGE_TIMEOUT`]; never panics.
pub fn judge_shell_completion(ctx: &ToolContext, command: &str, output: &str) -> CompletionVerdict {
    let Some(provider) = ctx.provider.clone() else {
        return CompletionVerdict::Unknown;
    };
    let output = tail_chars(output, JUDGE_OUTPUT_TAIL_CHARS);
    if output.trim().is_empty() {
        return CompletionVerdict::Unknown;
    }

    let request = ChatRequest {
        request_id: format!("shell-judge-{}", now_millis()),
        session_id: "shell-judge".to_string(),
        messages: vec![
            ChatMessage {
                id: "judge-system".into(),
                session_id: "shell-judge".into(),
                role: Role::System,
                content: SYSTEM_PROMPT.into(),
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
                id: "judge-user".into(),
                session_id: "shell-judge".into(),
                role: Role::User,
                content: format!("Command:\n{command}\n\nLatest output:\n{output}"),
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
        provider: Some(provider.id().to_string()),
        stream: true,
        tools: Arc::from([]),
        temperature: Some(0.0),
        max_tokens: Some(JUDGE_MAX_TOKENS),
    };

    let answer = block_on(async {
        tokio::time::timeout(JUDGE_TIMEOUT, collect_answer(provider, request)).await
    });
    match answer {
        Ok(Ok(text)) => parse_verdict(&text),
        _ => CompletionVerdict::Unknown,
    }
}

async fn collect_answer(
    provider: Arc<dyn AIProvider>,
    request: ChatRequest,
) -> Result<String, String> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(16);
    let provider_task = tauri::async_runtime::spawn(async move {
        let _ = provider.stream(request, tx).await;
    });
    let mut content = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Delta(text) => {
                content.push_str(&text);
                if content.len() > 1024 {
                    break;
                }
            }
            StreamEvent::TurnComplete { content: complete, .. } => {
                content = complete;
                break;
            }
            StreamEvent::Error(error) => return Err(error),
            StreamEvent::Finish => break,
            _ => {}
        }
    }
    let _ = provider_task.await;
    Ok(content)
}

fn parse_verdict(text: &str) -> CompletionVerdict {
    let token = text
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    match token.as_str() {
        "FINISHED" => CompletionVerdict::Finished,
        "RUNNING" => CompletionVerdict::Running,
        _ => CompletionVerdict::Unknown,
    }
}

fn tail_chars(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    value.chars().skip(count.saturating_sub(limit)).collect()
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

/// Bridge a sync tool thread onto the async runtime, same pattern as
/// `core::tools::agent::block_on_tool_future`.
fn block_on<F: Future>(future: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => tauri::async_runtime::block_on(future),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_parsing_is_strict() {
        assert_eq!(parse_verdict("FINISHED"), CompletionVerdict::Finished);
        assert_eq!(parse_verdict("finished"), CompletionVerdict::Finished);
        assert_eq!(parse_verdict(" FINISHED\n"), CompletionVerdict::Finished);
        assert_eq!(parse_verdict("RUNNING"), CompletionVerdict::Running);
        assert_eq!(parse_verdict("UNKNOWN"), CompletionVerdict::Unknown);
        assert_eq!(parse_verdict(""), CompletionVerdict::Unknown);
        assert_eq!(parse_verdict("The build finished"), CompletionVerdict::Unknown);
        assert_eq!(parse_verdict("maybe"), CompletionVerdict::Unknown);
    }

    #[test]
    fn tail_chars_keeps_the_end() {
        assert_eq!(tail_chars("abcdef", 3), "def");
        assert_eq!(tail_chars("ab", 3), "ab");
    }
}
