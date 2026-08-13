//! Honest-completion enforcement.
//!
//! A task-like final answer that claims completion without any successful
//! modifying tool (or without a verifying check) is challenged and sent back
//! so the model either actually executes the work or explicitly admits
//! nothing was changed — claiming done is not accepted at face value.
//!
//! [`CompletionGate`] owns the small bit of per-turn state (whether a
//! mutation/verification happened, and how many times each challenge has
//! already been issued) and exposes a single `evaluate_final_answer` entry
//! point that the loop calls whenever the model returns with no tool calls.
//! Adding another challenge policy later (e.g. a minimal-coding-specific
//! check) is meant to be an additive change here, not a change to the loop.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, Role};
use crate::runtime::ToolManager;

use super::types::{non_empty, now_millis, ToolOutcome};

/// How many times an unverified completion claim is challenged and sent back
/// before the final answer is replaced with an explicit unverified result.
pub const MAX_COMPLETION_RETRIES: u32 = 1;

/// Successful identical tool+args repeats that count as a no-progress loop.
pub const MAX_NO_PROGRESS_REPEATS: u32 = 3;

/// Injected after the model's final answer claims completion without any
/// successful modifying tool. The model must either actually execute the
/// work or explicitly admit nothing was changed — claiming done is not
/// accepted.
pub const COMPLETION_CHALLENGE: &str = concat!(
    "[System] Completion claim rejected: no modifying tool succeeded this turn. ",
    "Do not restate prior reasoning. Either call the required tools now, ",
    "or clearly say what was not changed and what is blocking.",
);

pub const VERIFICATION_CHALLENGE: &str = concat!(
    "[System] Changes ran but were not verified. ",
    "Do not restate prior reasoning. Read back the modified files (or inspect ",
    "the checkpoint/diff), or run a test/build check, then report the verified ",
    "result or the failure.",
);

pub const GOAL_COVERAGE_CHALLENGE: &str = concat!(
    "[System] Completion claim rejected: mentioned target paths were not ",
    "touched by a successful modifying tool this turn. Edit those paths or ",
    "explicitly state they are out of scope / blocked.",
);

pub const STALL_CHALLENGE: &str = concat!(
    "[System] Tool loop stall detected: the same tool+arguments succeeded ",
    "repeatedly without progress. Do not repeat that call. Change strategy ",
    "(different tool, different arguments, or read back evidence) or stop ",
    "and report the blocker.",
);

pub const SHARE_DELIVERABLE_CHALLENGE: &str = concat!(
    "[System] This turn edited files or started a local web server, but you have ",
    "not shared the deliverable. Before finishing, call `share_to_companion` for ",
    "the files the user should open, and/or `share_preview_url` for a local ",
    "http://127.0.0.1:... preview. Do not wrap files in HTML. Then answer.",
);

/// Global ablation switch used by the eval harness (`--no-challenge`).
static CHALLENGES_DISABLED: AtomicBool = AtomicBool::new(false);

pub fn set_challenges_enabled(enabled: bool) {
    CHALLENGES_DISABLED.store(!enabled, Ordering::Relaxed);
}

pub fn challenges_enabled() -> bool {
    !CHALLENGES_DISABLED.load(Ordering::Relaxed)
}

/// What the loop should do after evaluating a tool-call-free final answer.
pub enum ChallengeOutcome {
    /// A challenge was appended to `request` as a user message; the loop
    /// should emit the given status and run another provider turn.
    ContinueWithChallenge { status_kind: String },
    /// No further challenge applies; this is the turn's actual final answer.
    Finish {
        content: String,
        reasoning: Option<String>,
        finish_reason: Option<String>,
    },
}

/// Tracks whether this turn has produced real, verified work, and how many
/// times each completion challenge has already been issued.
#[derive(Default)]
pub struct CompletionGate {
    mutation_succeeded: bool,
    verification_succeeded: bool,
    empty_completion_retries: u32,
    verification_retries: u32,
    goal_coverage_retries: u32,
    stall_retries: u32,
    /// Paths mentioned in the current-turn user goal (heuristic).
    goal_paths: HashSet<String>,
    /// Paths successfully mutated this turn.
    mutated_paths: HashSet<String>,
    /// Success fingerprints: tool|args → count (no-progress loop detection).
    success_repeats: HashMap<String, u32>,
    stalled: bool,
    wrote_files: bool,
    started_local_server: bool,
    shared_deliverable: bool,
    share_nudge_retries: u32,
    /// Disable all challenges for this gate instance (eval ablation).
    disabled: bool,
}

impl CompletionGate {
    pub fn new() -> Self {
        Self {
            disabled: !challenges_enabled(),
            ..Self::default()
        }
    }

    /// Capture path-like targets from the latest user message before the loop.
    pub fn capture_goal_from_request(&mut self, request: &ChatRequest) {
        let Some(user) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == Role::User)
        else {
            return;
        };
        self.goal_paths = extract_goal_paths(&user.content);
    }

    /// Fold one tool outcome into the mutation/verification state. Must be
    /// called for every outcome in the order the tools ran.
    pub fn record_tool_outcome(&mut self, tools: &ToolManager, outcome: &ToolOutcome) {
        if outcome.success {
            let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
            let count = self.success_repeats.entry(key).or_insert(0);
            *count += 1;
            if *count >= MAX_NO_PROGRESS_REPEATS {
                self.stalled = true;
            }
            if let Some(path) = extract_path_from_args(&outcome.arguments) {
                if !tools.is_read_only(&outcome.tool_name)
                    && provides_completion_evidence(tools, outcome)
                {
                    self.mutated_paths.insert(path);
                }
            }
        }

        if !outcome.success {
            return;
        }
        if is_share_tool(&outcome.tool_name) {
            self.shared_deliverable = true;
        }
        if is_write_tool(&outcome.tool_name) {
            self.wrote_files = true;
        }
        if looks_like_local_dev_server(&outcome.tool_name, &outcome.arguments) {
            self.started_local_server = true;
        }
        if self.mutation_succeeded && provides_verification_evidence(tools, outcome) {
            self.verification_succeeded = true;
        } else if provides_completion_evidence(tools, outcome) {
            self.mutation_succeeded = true;
            self.verification_succeeded = false;
        }
    }

    /// After a tool batch: if the same successful call is looping, inject a
    /// stall challenge before the next model turn.
    pub fn maybe_challenge_stall(
        &mut self,
        request: &mut ChatRequest,
        user_msg_index: &mut Option<usize>,
    ) -> Option<String> {
        if self.disabled || !self.stalled || self.stall_retries >= MAX_COMPLETION_RETRIES {
            return None;
        }
        self.stall_retries += 1;
        self.stalled = false;
        push_challenge_message(request, user_msg_index, STALL_CHALLENGE);
        Some("stall_loop".to_string())
    }

    /// Evaluate a tool-call-free model answer against the honest-completion
    /// policy, deciding whether to challenge it or accept it as final.
    pub fn evaluate_final_answer(
        &mut self,
        request: &mut ChatRequest,
        user_msg_index: &mut Option<usize>,
        content: String,
        reasoning: String,
        finish_reason: Option<String>,
    ) -> ChallengeOutcome {
        if self.disabled {
            return ChallengeOutcome::Finish {
                content,
                reasoning: non_empty(reasoning),
                finish_reason,
            };
        }

        if (self.wrote_files || self.started_local_server)
            && !self.shared_deliverable
            && self.share_nudge_retries < MAX_COMPLETION_RETRIES
        {
            self.share_nudge_retries += 1;
            push_completion_feedback(
                request,
                user_msg_index,
                content,
                reasoning,
                SHARE_DELIVERABLE_CHALLENGE,
            );
            return ChallengeOutcome::ContinueWithChallenge {
                status_kind: "share_deliverable".to_string(),
            };
        }

        if !self.mutation_succeeded
            && !crate::runtime::tool::is_question_only_request(request)
            && has_completion_claim(&content)
            && self.empty_completion_retries < MAX_COMPLETION_RETRIES
        {
            self.empty_completion_retries += 1;
            push_completion_feedback(
                request,
                user_msg_index,
                content,
                reasoning,
                COMPLETION_CHALLENGE,
            );
            return ChallengeOutcome::ContinueWithChallenge {
                status_kind: "reject_empty_completion".to_string(),
            };
        }

        if self.mutation_succeeded
            && !self.goal_paths.is_empty()
            && has_completion_claim(&content)
            && !self.goal_paths_covered()
            && self.goal_coverage_retries < MAX_COMPLETION_RETRIES
        {
            self.goal_coverage_retries += 1;
            push_completion_feedback(
                request,
                user_msg_index,
                content,
                reasoning,
                GOAL_COVERAGE_CHALLENGE,
            );
            return ChallengeOutcome::ContinueWithChallenge {
                status_kind: "goal_coverage".to_string(),
            };
        }

        if self.mutation_succeeded
            && !self.verification_succeeded
            && has_completion_claim(&content)
            && self.verification_retries < MAX_COMPLETION_RETRIES
        {
            self.verification_retries += 1;
            push_completion_feedback(
                request,
                user_msg_index,
                content,
                reasoning,
                VERIFICATION_CHALLENGE,
            );
            return ChallengeOutcome::ContinueWithChallenge {
                status_kind: "verify_completion".to_string(),
            };
        }

        let mut final_content = content;
        let completion_rejected = reject_unverified_completion(
            &mut final_content,
            request,
            self.mutation_succeeded,
            self.verification_succeeded,
        );
        ChallengeOutcome::Finish {
            content: final_content,
            reasoning: non_empty(reasoning),
            finish_reason: if completion_rejected {
                Some("unverified_completion".to_string())
            } else {
                finish_reason
            },
        }
    }

    fn goal_paths_covered(&self) -> bool {
        if self.goal_paths.is_empty() {
            return true;
        }
        self.goal_paths.iter().any(|goal| {
            self.mutated_paths
                .iter()
                .any(|mutated| paths_match(goal, mutated))
        })
    }
}

fn is_write_tool(name: &str) -> bool {
    matches!(
        name,
        "write_file"
            | "replace_in_file"
            | "replace_many_in_file"
            | "apply_patch"
            | "edit_notebook_cell"
    )
}

fn is_share_tool(name: &str) -> bool {
    matches!(name, "share_to_companion" | "share_preview_url")
}

fn looks_like_local_dev_server(tool_name: &str, arguments: &str) -> bool {
    if tool_name != "run_shell" {
        return false;
    }
    let value: serde_json::Value = serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null);
    let background = value
        .get("run_in_background")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !background {
        return false;
    }
    let command = value
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or(arguments)
        .to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "npm ",
        "npm.cmd",
        "pnpm ",
        "yarn ",
        "vite",
        "next dev",
        "nuxt",
        "webpack-dev-server",
        "npx ",
    ];
    MARKERS.iter().any(|marker| command.contains(marker))
}

fn paths_match(goal: &str, mutated: &str) -> bool {
    let g = normalize_path_key(goal);
    let m = normalize_path_key(mutated);
    m == g || m.ends_with(&g) || g.ends_with(&m)
}

fn normalize_path_key(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

fn extract_goal_paths(content: &str) -> HashSet<String> {
    let mut paths = HashSet::new();
    for token in content.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| {
            matches!(
                c,
                ',' | ';' | ':' | '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });
        if looks_like_path(cleaned) {
            paths.insert(cleaned.to_string());
        }
    }
    // Also catch `path/to/file.ext` inside backticks.
    for part in content.split('`') {
        let trimmed = part.trim();
        if looks_like_path(trimmed) {
            paths.insert(trimmed.to_string());
        }
    }
    paths
}

fn looks_like_path(value: &str) -> bool {
    if value.len() < 3 || value.len() > 240 {
        return false;
    }
    let has_sep = value.contains('/') || value.contains('\\');
    let has_ext = value.contains('.')
        && value
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|name| name.contains('.') && !name.starts_with('.'));
    has_sep || has_ext && value.chars().any(|c| c.is_ascii_alphanumeric())
}

fn extract_path_from_args(arguments: &str) -> Option<String> {
    // Arguments are stored as a JSON string on ToolOutcome.
    let value: serde_json::Value = serde_json::from_str(arguments).ok()?;
    value
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .or_else(|| {
            value
                .get("file_path")
                .and_then(|v| v.as_str())
                .filter(|p| !p.is_empty())
                .map(str::to_string)
        })
}

pub(crate) fn push_challenge_message(
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    feedback: &str,
) {
    if user_msg_index.is_none() {
        *user_msg_index = Some(request.messages.len());
    }
    request.messages.push(ChatMessage {
        id: format!("msg-{}", now_millis()),
        session_id: request.session_id.clone(),
        role: Role::User,
        content: feedback.to_string(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: now_millis(),
        estimated_tokens: None,
    });
}

fn push_completion_feedback(
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    content: String,
    reasoning: String,
    feedback: &str,
) {
    request.messages.push(ChatMessage {
        id: format!("msg-{}", now_millis()),
        session_id: request.session_id.clone(),
        role: Role::Assistant,
        content,
        reasoning: non_empty(reasoning),
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: now_millis(),
        estimated_tokens: None,
    });
    push_challenge_message(request, user_msg_index, feedback);
}

/// Successful non-read-only tools normally prove that work happened, but
/// orchestration-only tools must not let a model turn task bookkeeping into
/// evidence that the requested change was made.
fn provides_completion_evidence(tools: &ToolManager, outcome: &ToolOutcome) -> bool {
    if tools.is_read_only(&outcome.tool_name) {
        return false;
    }
    !matches!(
        outcome.tool_name.as_str(),
        "update_tasks" | "ask_user" | "complete_plan_step" | "connect_tools"
    )
}

fn provides_verification_evidence(tools: &ToolManager, outcome: &ToolOutcome) -> bool {
    if tools.is_read_only(&outcome.tool_name) {
        // Forced read-back of a mutated file counts as verification.
        if matches!(
            outcome.tool_name.as_str(),
            "read_file" | "search_files" | "list_folder" | "find_files" | "list_symbols"
        ) {
            if let Some(path) = extract_path_from_args(&outcome.arguments) {
                // Gate instance state is not available here — treat any successful
                // read_file as verification evidence when mutations already ran
                // (caller already gated on mutation_succeeded).
                let _ = path;
                return true;
            }
            return outcome.tool_name == "read_file";
        }
        return !matches!(
            outcome.tool_name.as_str(),
            "search_memory" | "list_chats" | "read_chat" | "search_past_chats"
        );
    }
    if outcome.tool_name != "run_shell" {
        return false;
    }
    let command = outcome.arguments.to_ascii_lowercase();
    const CHECK_MARKERS: &[&str] = &[
        " test",
        "test ",
        "cargo test",
        "pytest",
        "unittest",
        "pnpm build",
        "npm run build",
        "npm test",
        "cargo check",
        "tsc",
        "vue-tsc",
        "lint",
        "check",
        "verify",
        "git diff",
        "git status",
    ];
    CHECK_MARKERS.iter().any(|marker| command.contains(marker))
}

/// A change request cannot finish with a completion claim unless a modifying
/// tool succeeded in this turn. Replace the claim instead of displaying it
/// with a caveat, because the original text is still misleading and looks
/// complete.
fn reject_unverified_completion(
    content: &mut String,
    request: &ChatRequest,
    mutation_succeeded: bool,
    verification_succeeded: bool,
) -> bool {
    if mutation_succeeded && verification_succeeded {
        return false;
    }
    if crate::runtime::tool::is_question_only_request(request) {
        return false;
    }
    if !has_completion_claim(content) {
        return false;
    }
    *content = if content
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        if mutation_succeeded {
            "未验证完成：虽然执行了修改，但没有成功检查修改后的结果，因此不能确认任务真的完成。请运行读取检查、测试或构建验证。".to_string()
        } else {
            "未完成：本轮没有任何修改类工具成功执行，因此无法确认发生了实际改动。请重新执行所需操作，或明确说明当前阻塞项。".to_string()
        }
    } else if mutation_succeeded {
        "Completion not verified: a modification ran, but its result was not successfully checked. Run a read-back, test, build, or equivalent verification before claiming completion.".to_string()
    } else {
        "Not completed: no modifying tool succeeded in this turn, so no actual change can be verified. Run the required operation or state the current blocker explicitly.".to_string()
    };
    true
}

fn has_completion_claim(content: &str) -> bool {
    // Keep this strict: weak phrases like "搞定/修改了/done" used to false-trigger
    // an extra model round (and another full think cycle).
    const CLAIMS: &[&str] = &[
        "已完成",
        "全部完成",
        "完成修改",
        "修改完成",
        "修复完成",
        "更新完成",
        "创建完成",
        "写入完成",
        "任务完成",
        "全部搞定",
        "大功告成",
        "successfully completed",
        "task completed",
        "all done",
        "all set",
        "has been fixed",
        "have fixed",
        "has been completed",
        "have completed",
        "implementation is complete",
        "changes are complete",
    ];
    let lower = content.to_ascii_lowercase();
    CLAIMS
        .iter()
        .any(|claim| lower.contains(&claim.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_goal_paths_from_user_text() {
        let paths = extract_goal_paths("Please edit `src/main.rs` and fix README.md");
        assert!(paths.iter().any(|p| p.contains("main.rs")));
        assert!(paths.iter().any(|p| p.contains("README.md")));
    }

    #[test]
    fn stall_challenge_fires_after_repeated_success() {
        use crate::core::tools::context::Tool;
        use crate::core::tools::registry::ToolRegistry;
        use crate::runtime::ToolManager;
        use std::sync::Arc;

        struct StubTool;
        impl Tool for StubTool {
            fn name(&self) -> &str {
                "read_file"
            }
            fn description(&self) -> &str {
                "r"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            fn read_only(&self) -> bool {
                true
            }
            fn execute(
                &self,
                _ctx: &crate::core::tools::context::ToolContext,
                _args: serde_json::Value,
            ) -> Result<String, crate::core::tools::error::ToolError> {
                Ok("ok".into())
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(StubTool));
        let tools = ToolManager::new(registry);
        let mut gate = CompletionGate::new();
        let outcome = ToolOutcome {
            call_id: "1".into(),
            tool_name: "read_file".into(),
            arguments: r#"{"path":"a.rs"}"#.into(),
            result: "ok".into(),
            success: true,
            user_denied: false,
        };
        for _ in 0..MAX_NO_PROGRESS_REPEATS {
            gate.record_tool_outcome(&tools, &outcome);
        }
        assert!(gate.stalled);
    }

    #[test]
    fn detects_background_vite_as_local_server() {
        assert!(looks_like_local_dev_server(
            "run_shell",
            r#"{"command":"npx vite --host","run_in_background":true}"#
        ));
        assert!(!looks_like_local_dev_server(
            "run_shell",
            r#"{"command":"npx vite --host","run_in_background":false}"#
        ));
        assert!(is_write_tool("write_file"));
        assert!(is_share_tool("share_preview_url"));
    }
}
