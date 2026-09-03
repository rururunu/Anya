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

use crate::core::chat::agent_loop::post_edit_verify::is_mutation_tool;
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

pub const IMAGE_REQUIRED_CHALLENGE: &str = concat!(
    "[System] No image was generated this turn. You described a picture that does not exist. ",
    "Call `generate_image` now with a complete prompt. Do not write a caption, ",
    "say the image is ready, or offer a follow-up edit until the tool returns markdown.",
);

pub const OPEN_TASKS_CHALLENGE: &str = concat!(
    "[System] Completion rejected: plan/todo items are still open. ",
    "Mark each finished step with `complete_plan_step` and evidence ",
    "(path or check command), or cancel out-of-scope steps explicitly, then answer.",
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
#[derive(Debug)]
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
    /// Image chat mode: a turn cannot finish without a successful generate_image.
    require_image: bool,
    image_succeeded: bool,
    image_retries: u32,
    /// Open plan/todo items still pending this turn (from update_tasks).
    open_task_count: u32,
    open_tasks_retries: u32,
    /// Only nudge Companion share when the session originated from the phone app.
    require_share_deliverable: bool,
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

    pub fn require_image(&mut self) {
        self.require_image = true;
    }

    pub fn require_share_deliverable(&mut self) {
        self.require_share_deliverable = true;
    }

    /// Capture path-like targets from the latest real user goal (skip plan-approve boilerplate).
    pub fn capture_goal_from_request(&mut self, request: &ChatRequest) {
        let Some(user) = request.messages.iter().rev().find(|message| {
            message.role == Role::User && !is_plan_approve_boilerplate(&message.content)
        }) else {
            return;
        };
        self.goal_paths = extract_goal_paths(&user.content);
    }

    /// Record a batch with mutation pass first, then verification — fixes parallel
    /// waves where a read arrives before a write in the outcome list.
    pub fn record_tool_outcomes(&mut self, tools: &ToolManager, outcomes: &[ToolOutcome]) {
        for outcome in outcomes {
            self.record_side_effects(tools, outcome);
        }
        for outcome in outcomes {
            if !outcome.success {
                continue;
            }
            if outcome.tool_name == "generate_image" {
                self.image_succeeded = true;
                self.mutation_succeeded = true;
                self.verification_succeeded = true;
            } else if provides_completion_evidence(outcome) {
                self.mutation_succeeded = true;
                self.verification_succeeded = false;
            }
        }
        for outcome in outcomes {
            if !outcome.success || !self.mutation_succeeded {
                continue;
            }
            if provides_verification_evidence(outcome, &self.mutated_paths) {
                self.verification_succeeded = true;
            }
        }
    }

    /// Fold one tool outcome (single-item wrapper around [`Self::record_tool_outcomes`]).
    pub fn record_tool_outcome(&mut self, tools: &ToolManager, outcome: &ToolOutcome) {
        self.record_tool_outcomes(tools, std::slice::from_ref(outcome));
    }

    /// Merge paths a sub-agent mutated into this parent gate.
    pub fn note_subagent_mutations(&mut self, paths: impl IntoIterator<Item = String>) {
        let mut any = false;
        for path in paths {
            if path.is_empty() {
                continue;
            }
            self.mutated_paths.insert(path);
            any = true;
        }
        if any {
            self.mutation_succeeded = true;
            self.verification_succeeded = false;
            self.wrote_files = true;
        }
    }

    /// Track open plan/todo items from `update_tasks` payloads.
    #[allow(dead_code)]
    pub fn note_open_task_count(&mut self, count: u32) {
        self.open_task_count = count;
    }

    fn record_side_effects(&mut self, _tools: &ToolManager, outcome: &ToolOutcome) {
        if outcome.tool_name == "update_tasks" {
            if let Some(count) = count_open_tasks_from_args(&outcome.arguments) {
                self.open_task_count = count;
            }
        }
        if outcome.tool_name == "complete_plan_step" && outcome.success {
            self.open_task_count = self.open_task_count.saturating_sub(1);
        }
        if outcome.success
            && matches!(
                outcome.tool_name.as_str(),
                "run_subagent" | "run_parallel_subagents" | "run_skill"
            )
        {
            let paths =
                crate::core::tools::agent::extract_touched_paths_from_subagent_result(&outcome.result);
            if !paths.is_empty() {
                self.note_subagent_mutations(paths);
            }
        }
        if !outcome.success {
            return;
        }
        let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
        let count = self.success_repeats.entry(key).or_insert(0);
        *count += 1;
        if *count >= MAX_NO_PROGRESS_REPEATS {
            self.stalled = true;
        }
        if provides_completion_evidence(outcome) {
            for path in extract_paths_from_args(&outcome.arguments) {
                self.mutated_paths.insert(path);
            }
        }
        if is_share_tool(&outcome.tool_name) {
            self.shared_deliverable = true;
        }
        if is_write_tool(&outcome.tool_name) || is_mutation_tool(&outcome.tool_name) {
            self.wrote_files = true;
        }
        if looks_like_local_dev_server(&outcome.tool_name, &outcome.arguments) {
            self.started_local_server = true;
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

        if self.require_image && !self.image_succeeded {
            if self.image_retries < MAX_COMPLETION_RETRIES {
                self.image_retries += 1;
                push_completion_feedback(
                    request,
                    user_msg_index,
                    content,
                    reasoning,
                    IMAGE_REQUIRED_CHALLENGE,
                );
                return ChallengeOutcome::ContinueWithChallenge {
                    status_kind: "require_image".to_string(),
                };
            }
            return ChallengeOutcome::Finish {
                content: image_missing_message(&content),
                reasoning: non_empty(reasoning),
                finish_reason: Some("missing_image".to_string()),
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

        if self.mutation_succeeded
            && self.open_task_count > 0
            && self.open_tasks_retries < MAX_COMPLETION_RETRIES
        {
            self.open_tasks_retries += 1;
            push_completion_feedback(
                request,
                user_msg_index,
                content,
                reasoning,
                OPEN_TASKS_CHALLENGE,
            );
            return ChallengeOutcome::ContinueWithChallenge {
                status_kind: "open_tasks".to_string(),
            };
        }

        if self.require_share_deliverable
            && (self.wrote_files || self.started_local_server)
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

        let mut final_content = content;
        let completion_rejected = reject_unverified_completion(
            &mut final_content,
            request,
            self.mutation_succeeded,
            self.verification_succeeded,
            self.mutation_succeeded && self.open_task_count > 0,
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
        self.goal_paths.iter().all(|goal| {
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
    let value: serde_json::Value =
        serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null);
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
        let cleaned = trim_path_token(token);
        if looks_like_path(cleaned) {
            paths.insert(cleaned.to_string());
        }
    }
    // Only tokens inside backticks — `split` on a string with no backticks
    // yields the whole sentence, which must not be treated as a path.
    if content.contains('`') {
        for (index, part) in content.split('`').enumerate() {
            if index % 2 == 0 {
                continue;
            }
            let trimmed = trim_path_token(part.trim());
            if looks_like_path(trimmed) {
                paths.insert(trimmed.to_string());
            }
        }
    }
    paths
}

fn trim_path_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            ',' | ';' | ':' | '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '.' | '!' | '?'
        )
    })
}

fn looks_like_path(value: &str) -> bool {
    if value.len() < 3 || value.len() > 240 || value.contains(' ') {
        return false;
    }
    let has_sep = value.contains('/') || value.contains('\\');
    if has_sep {
        return true;
    }
    // Extension-only paths must look like `name.ext` with a short alphanumeric ext.
    // Reject version-like tokens (`1.0.0`) that have multiple dots.
    let Some((stem, ext)) = value.rsplit_once('.') else {
        return false;
    };
    if stem.is_empty() || stem.starts_with('.') || stem.contains('.') {
        return false;
    }
    let ext_ok = (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric());
    ext_ok && stem.chars().any(|c| c.is_ascii_alphanumeric())
}

fn extract_path_from_args(arguments: &str) -> Option<String> {
    extract_paths_from_args(arguments).into_iter().next()
}

fn extract_paths_from_args(arguments: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(arguments) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for key in ["path", "file_path", "from", "to"] {
        if let Some(path) = value
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|p| !p.is_empty())
        {
            paths.push(path.to_string());
        }
    }
    if let Some(input) = value.get("input").and_then(|v| v.as_str()) {
        for line in input.lines() {
            for marker in [
                "*** Update File: ",
                "*** Add File: ",
                "*** Delete File: ",
            ] {
                if let Some(path) = line.strip_prefix(marker) {
                    let path = path.trim();
                    if !path.is_empty() {
                        paths.push(path.to_string());
                    }
                }
            }
        }
    }
    paths
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

/// Successful file mutations prove that work happened. Orchestration-only tools
/// and bare shell commands do not count as completion evidence.
fn provides_completion_evidence(outcome: &ToolOutcome) -> bool {
    is_mutation_tool(&outcome.tool_name)
}

fn provides_verification_evidence(
    outcome: &ToolOutcome,
    mutated_paths: &HashSet<String>,
) -> bool {
    if outcome.tool_name == "read_file" {
        let Some(path) = extract_path_from_args(&outcome.arguments) else {
            return false;
        };
        return mutated_paths
            .iter()
            .any(|mutated| paths_match(mutated, &path));
    }
    if outcome.tool_name != "run_shell" || !outcome.success {
        return false;
    }
    let Some(command) = extract_shell_command(&outcome.arguments) else {
        return false;
    };
    let command = command.to_ascii_lowercase();
    const CHECK_MARKERS: &[&str] = &[
        "cargo test",
        "cargo check",
        "pytest",
        "python -m pytest",
        "python -m unittest",
        "pnpm check",
        "pnpm test",
        "pnpm build",
        "npm run check",
        "npm run -s check",
        "npm run typecheck",
        "npm run -s typecheck",
        "npm run lint",
        "npm run -s lint",
        "npm test",
        "npm run build",
        "vue-tsc",
        "tsc --noemit",
        "tsc -p",
        "git diff --",
    ];
    CHECK_MARKERS.iter().any(|marker| command.contains(marker))
}

fn extract_shell_command(arguments: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(arguments).ok()?;
    value
        .get("command")
        .and_then(|v| v.as_str())
        .filter(|c| !c.is_empty())
        .map(str::to_string)
}

fn is_plan_approve_boilerplate(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with("计划已批准")
        || trimmed.to_ascii_lowercase().starts_with("plan approved")
        || trimmed.contains("本回合写操作已解除限制")
        || trimmed.contains("write tools are unlocked for this turn")
}

fn count_open_tasks_from_args(arguments: &str) -> Option<u32> {
    let value: serde_json::Value = serde_json::from_str(arguments).ok()?;
    let tasks = value.get("tasks")?.as_array()?;
    let open = tasks
        .iter()
        .filter(|task| {
            let status = task
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            !matches!(status, "completed" | "cancelled" | "done")
        })
        .count();
    Some(open as u32)
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
    has_open_tasks: bool,
) -> bool {
    if mutation_succeeded && verification_succeeded && !has_open_tasks {
        return false;
    }
    if crate::runtime::tool::is_question_only_request(request) {
        return false;
    }
    // After retries are exhausted: rewrite when there was mutation without
    // verification, open tasks remain, or a completion claim without mutation.
    let should_rewrite = (mutation_succeeded && !verification_succeeded)
        || has_open_tasks
        || (!mutation_succeeded && has_completion_claim(content));
    if !should_rewrite {
        return false;
    }
    *content = if content
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        if has_open_tasks {
            "未完成：仍有未勾选的计划步骤。请为已完成的步骤提供证据并标记完成，或明确取消超出范围的步骤。".to_string()
        } else if mutation_succeeded {
            "未验证完成：虽然执行了修改，但没有成功检查修改后的结果，因此不能确认任务真的完成。请运行读取检查、测试或构建验证。".to_string()
        } else {
            "未完成：本轮没有任何修改类工具成功执行，因此无法确认发生了实际改动。请重新执行所需操作，或明确说明当前阻塞项。".to_string()
        }
    } else if has_open_tasks {
        "Not completed: plan/todo items are still open. Complete them with evidence or cancel out-of-scope steps before claiming done.".to_string()
    } else if mutation_succeeded {
        "Completion not verified: a modification ran, but its result was not successfully checked. Run a read-back, test, build, or equivalent verification before claiming completion.".to_string()
    } else {
        "Not completed: no modifying tool succeeded in this turn, so no actual change can be verified. Run the required operation or state the current blocker explicitly.".to_string()
    };
    true
}

fn image_missing_message(content: &str) -> String {
    if content
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        "未能生成图片：这一轮没有成功调用生图工具，所以没有新图。请再发送一次，或检查设置 → 生图。"
            .to_string()
    } else {
        "No image was generated: generate_image did not succeed this turn, so there is no new picture. Send again, or check Settings → Image."
            .to_string()
    }
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

    fn empty_request() -> ChatRequest {
        ChatRequest {
            request_id: "r".into(),
            session_id: "s".into(),
            messages: vec![],
            context: crate::core::runtime::RequestContext::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        }
    }

    #[test]
    fn image_mode_rejects_caption_without_generate_image() {
        let mut gate = CompletionGate::new();
        gate.require_image();
        let mut request = empty_request();
        let mut user_idx = None;
        let first = gate.evaluate_final_answer(
            &mut request,
            &mut user_idx,
            "新图来了 📸 衣服换成了雪碧风格。".into(),
            String::new(),
            Some("stop".into()),
        );
        match first {
            ChallengeOutcome::ContinueWithChallenge { status_kind } => {
                assert_eq!(status_kind, "require_image");
            }
            other => panic!("expected require_image challenge, got {other:?}"),
        }
        let second = gate.evaluate_final_answer(
            &mut request,
            &mut user_idx,
            "已经生成好了。".into(),
            String::new(),
            Some("stop".into()),
        );
        match second {
            ChallengeOutcome::Finish { content, finish_reason, .. } => {
                assert!(content.contains("未能生成图片"));
                assert_eq!(finish_reason.as_deref(), Some("missing_image"));
            }
            other => panic!("expected missing_image finish, got {other:?}"),
        }
    }

    #[test]
    fn verification_requires_read_of_mutated_path() {
        use crate::core::tools::context::Tool;
        use crate::core::tools::registry::ToolRegistry;
        use crate::runtime::ToolManager;
        use std::sync::Arc;

        struct WriteTool;
        impl Tool for WriteTool {
            fn name(&self) -> &str {
                "write_file"
            }
            fn description(&self) -> &str {
                "w"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            fn read_only(&self) -> bool {
                false
            }
            fn execute(
                &self,
                _ctx: &crate::core::tools::context::ToolContext,
                _args: serde_json::Value,
            ) -> Result<String, crate::core::tools::error::ToolError> {
                Ok("ok".into())
            }
        }
        struct ReadTool;
        impl Tool for ReadTool {
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
        registry.register(Arc::new(WriteTool));
        registry.register(Arc::new(ReadTool));
        let tools = ToolManager::new(registry);
        let mut gate = CompletionGate::new();
        gate.record_tool_outcomes(
            &tools,
            &[
                ToolOutcome {
                    call_id: "1".into(),
                    tool_name: "write_file".into(),
                    arguments: r#"{"path":"src/a.txt","content":"x"}"#.into(),
                    result: "ok".into(),
                    success: true,
                    user_denied: false,
                },
                ToolOutcome {
                    call_id: "2".into(),
                    tool_name: "read_file".into(),
                    arguments: r#"{"path":"noise.txt"}"#.into(),
                    result: "noise".into(),
                    success: true,
                    user_denied: false,
                },
            ],
        );
        let mut request = empty_request();
        let mut user_idx = None;
        match gate.evaluate_final_answer(
            &mut request,
            &mut user_idx,
            "here is the change".into(),
            String::new(),
            Some("stop".into()),
        ) {
            ChallengeOutcome::ContinueWithChallenge { status_kind } => {
                assert_eq!(status_kind, "verify_completion");
            }
            other => panic!("expected verify_completion, got {other:?}"),
        }
    }

    #[test]
    fn goal_coverage_requires_all_paths() {
        use crate::core::tools::context::Tool;
        use crate::core::tools::registry::ToolRegistry;
        use crate::runtime::ToolManager;
        use std::sync::Arc;

        struct WriteTool;
        impl Tool for WriteTool {
            fn name(&self) -> &str {
                "write_file"
            }
            fn description(&self) -> &str {
                "w"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            fn read_only(&self) -> bool {
                false
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
        registry.register(Arc::new(WriteTool));
        let tools = ToolManager::new(registry);
        let mut gate = CompletionGate::new();
        let mut request = empty_request();
        request.messages.push(ChatMessage {
            id: "u".into(),
            session_id: "s".into(),
            role: Role::User,
            content: "edit a.txt and b.txt".into(),
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
        gate.capture_goal_from_request(&request);
        gate.record_tool_outcome(
            &tools,
            &ToolOutcome {
                call_id: "1".into(),
                tool_name: "write_file".into(),
                arguments: r#"{"path":"a.txt","content":"1"}"#.into(),
                result: "ok".into(),
                success: true,
                user_denied: false,
            },
        );
        let mut user_idx = None;
        match gate.evaluate_final_answer(
            &mut request,
            &mut user_idx,
            "done".into(),
            String::new(),
            Some("stop".into()),
        ) {
            ChallengeOutcome::ContinueWithChallenge { status_kind } => {
                assert_eq!(status_kind, "goal_coverage");
            }
            other => panic!("expected goal_coverage, got {other:?}"),
        }
    }

    #[test]
    fn skips_plan_approve_when_capturing_goals() {
        let mut gate = CompletionGate::new();
        let mut request = empty_request();
        request.messages.push(ChatMessage {
            id: "u1".into(),
            session_id: "s".into(),
            role: Role::User,
            content: "Please edit `src/main.rs`".into(),
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
        request.messages.push(ChatMessage {
            id: "u2".into(),
            session_id: "s".into(),
            role: Role::User,
            content: "计划已批准。现在按批准的计划执行，本回合写操作已解除限制。".into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        });
        gate.capture_goal_from_request(&request);
        assert!(gate.goal_paths.iter().any(|p| p.contains("main.rs")));
    }

    #[test]
    fn write_outcome_records_mutated_path() {
        use crate::core::tools::context::Tool;
        use crate::core::tools::registry::ToolRegistry;
        use crate::runtime::ToolManager;
        use std::sync::Arc;

        struct WriteTool;
        impl Tool for WriteTool {
            fn name(&self) -> &str {
                "write_file"
            }
            fn description(&self) -> &str {
                "w"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            fn read_only(&self) -> bool {
                false
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
        registry.register(Arc::new(WriteTool));
        let tools = ToolManager::new(registry);
        let mut gate = CompletionGate::new();
        let mut request = empty_request();
        request.messages.push(ChatMessage {
            id: "u".into(),
            session_id: "s".into(),
            role: Role::User,
            content: "Write src/a.txt to alpha.".into(),
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
        gate.capture_goal_from_request(&request);
        assert_eq!(gate.goal_paths.len(), 1, "goals: {:?}", gate.goal_paths);
        gate.record_tool_outcome(
            &tools,
            &ToolOutcome {
                call_id: "1".into(),
                tool_name: "write_file".into(),
                arguments: r#"{"path":"src/a.txt","content":"alpha"}"#.into(),
                result: "written".into(),
                success: true,
                user_denied: false,
            },
        );
        assert!(
            gate.goal_paths_covered(),
            "mutated={:?} goals={:?}",
            gate.mutated_paths,
            gate.goal_paths
        );
        let mut user_idx = None;
        match gate.evaluate_final_answer(
            &mut request,
            &mut user_idx,
            "任务完成".into(),
            String::new(),
            Some("stop".into()),
        ) {
            ChallengeOutcome::ContinueWithChallenge { status_kind } => {
                assert_ne!(status_kind, "goal_coverage");
                assert_eq!(status_kind, "verify_completion");
            }
            other => panic!("expected verify_completion, got {other:?}"),
        }
    }
}
