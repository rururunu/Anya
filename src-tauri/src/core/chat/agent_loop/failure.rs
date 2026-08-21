use std::collections::HashMap;

use crate::core::chat::limits::{
    CONSECUTIVE_TOOL_FAILURE_CHALLENGE, MAX_CONSECUTIVE_TOOL_FAILURES,
};
use crate::core::tools::plan_mode::PLAN_GATE_BLOCKED;

use super::types::ToolOutcome;

/// Injected once when the same tool+args fails with the same error again,
/// giving the model a chance to change strategy before a hard stop.
pub const IDENTICAL_ERROR_CHALLENGE: &str = concat!(
    "[System] Identical tool failure: the same tool call with the same ",
    "arguments returned the same error again. Do NOT retry that exact call. ",
    "Read the error, then change strategy — different path/arguments, a ",
    "different tool, request permission, or stop and report the blocker ",
    "clearly to the user.",
);

/// Injected once when several *different* tool steps have failed in a row.
pub const CONSECUTIVE_FAILURE_CHALLENGE: &str = concat!(
    "[System] Several tool calls have failed in a row. Do NOT repeat the ",
    "same approach. Change strategy: different tool or arguments, ask the ",
    "user for the missing fact (exact date, URL, site, credentials), or ",
    "stop and report the blocker clearly. Last errors:\n",
);

/// Stop the turn as soon as plan mode rejects a writer — do not let the
/// model "change strategy" with a different Shell command (same gate).
pub const PLAN_GATE_STOP_REASON: &str =
    "计划尚未批准，Shell 和写文件已暂停。请点「批准并执行」，或批准后再发消息继续。";

/// How many identical (tool+args+error) failures are allowed before hard stop.
/// 1 = first failure recorded; 2 = challenge; 3 = stop.
const IDENTICAL_ERROR_STOP_AFTER: u32 = 3;

/// Result of folding a tool batch into the failure circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureAction {
    /// Keep the agent loop going.
    Continue,
    /// Inject a challenge and run another model turn (do not stop yet).
    Challenge {
        status_kind: String,
        message: String,
    },
    /// Hard-stop the turn with a user-visible reason.
    Stop { reason: String },
}

/// 失败熔断与同错误防重复：连续失败先挑战换策略，超过硬停阈值才停止；
/// 同一工具以相同参数反复返回同一错误时，先挑战一次，再犯才硬停。
#[derive(Default)]
pub struct FailureBreaker {
    consecutive_tool_failures: u32,
    /// tool|args → (error text, consecutive identical count)
    repeated_tool_errors: HashMap<String, (String, u32)>,
    /// Keys that already received an identical-error challenge this turn.
    challenged_keys: std::collections::HashSet<String>,
    /// Whether the consecutive-failure challenge was already injected.
    consecutive_challenged: bool,
}

impl FailureBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a batch of tool outcomes (from one step) into the failure state.
    ///
    /// A parallel wave counts as **one** consecutive failure, not one per
    /// child. Otherwise `run_parallel_subagents` / three read-only tools in a
    /// single model step trip the breaker immediately and the turn never gets
    /// a chance to change strategy.
    pub fn check(&mut self, outcomes: &[ToolOutcome]) -> FailureAction {
        let mut action = FailureAction::Continue;
        let mut any_success = false;
        let mut any_failure = false;
        let mut batch_errors: Vec<String> = Vec::new();

        for outcome in outcomes {
            if outcome.user_denied {
                continue;
            }
            if outcome.success {
                any_success = true;
                continue;
            }
            if is_plan_gate_error(&outcome.result) {
                return FailureAction::Stop {
                    reason: PLAN_GATE_STOP_REASON.to_string(),
                };
            }
            any_failure = true;
            batch_errors.push(format!(
                "- `{}`: {}",
                display_tool_name(&outcome.tool_name),
                truncate_error(&outcome.result, 240)
            ));

            let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
            let next_count = match self.repeated_tool_errors.get(&key) {
                Some((previous, count)) if previous == &outcome.result => count + 1,
                _ => 1,
            };
            self.repeated_tool_errors
                .insert(key.clone(), (outcome.result.clone(), next_count));

            // Prefer the more specific identical-error stop over the generic
            // consecutive-failure breaker when both would fire.
            if next_count >= IDENTICAL_ERROR_STOP_AFTER {
                return FailureAction::Stop {
                    reason: format!(
                        "工具 `{}` 以相同参数反复返回同一错误，已停止重试。请换路径/参数/工具后再试，或发送「继续」。",
                        display_tool_name(&outcome.tool_name)
                    ),
                };
            }

            // Second identical failure → challenge once (first was only recorded).
            if next_count >= 2 && !self.challenged_keys.contains(&key) {
                self.challenged_keys.insert(key);
                action = FailureAction::Challenge {
                    status_kind: "identical_error".into(),
                    message: format!(
                        "{IDENTICAL_ERROR_CHALLENGE}\n\nLast error from `{}`:\n{}",
                        display_tool_name(&outcome.tool_name),
                        truncate_error(&outcome.result, 800)
                    ),
                };
            }
        }

        if any_success {
            self.consecutive_tool_failures = 0;
            self.consecutive_challenged = false;
            return action;
        }
        if !any_failure {
            return action;
        }

        self.consecutive_tool_failures += 1;
        if self.consecutive_tool_failures >= MAX_CONSECUTIVE_TOOL_FAILURES {
            return FailureAction::Stop {
                reason: format!(
                    "工具连续失败 {} 次，已触发熔断。请换一种做法，或发送「继续」让我接着处理。",
                    MAX_CONSECUTIVE_TOOL_FAILURES
                ),
            };
        }
        if self.consecutive_tool_failures >= CONSECUTIVE_TOOL_FAILURE_CHALLENGE
            && !self.consecutive_challenged
            && matches!(action, FailureAction::Continue)
        {
            self.consecutive_challenged = true;
            let errors = if batch_errors.is_empty() {
                "- (none)".to_string()
            } else {
                batch_errors.join("\n")
            };
            action = FailureAction::Challenge {
                status_kind: "consecutive_failure".into(),
                message: format!("{CONSECUTIVE_FAILURE_CHALLENGE}{errors}"),
            };
        }
        action
    }
}

fn display_tool_name(name: &str) -> &str {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "未知工具"
    } else {
        trimmed
    }
}

fn truncate_error(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(max_chars).collect();
    format!("{cut}…")
}

pub fn is_plan_gate_error(result: &str) -> bool {
    result.contains(PLAN_GATE_BLOCKED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::limits::MAX_CONSECUTIVE_TOOL_FAILURES;
    use crate::core::tools::plan_mode::PLAN_GATE_BLOCKED;

    fn outcome(tool: &str, args: &str, result: &str, success: bool) -> ToolOutcome {
        ToolOutcome {
            call_id: "1".into(),
            tool_name: tool.into(),
            arguments: args.into(),
            result: result.into(),
            success,
            user_denied: false,
        }
    }

    #[test]
    fn identical_error_challenges_before_stop() {
        let mut breaker = FailureBreaker::new();

        assert_eq!(
            breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]),
            FailureAction::Continue
        );
        match breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]) {
            FailureAction::Challenge { status_kind, .. } => {
                assert_eq!(status_kind, "identical_error");
            }
            other => panic!("expected challenge, got {other:?}"),
        }
        match breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]) {
            FailureAction::Stop { reason } => {
                assert!(reason.contains("write_file"), "{reason}");
                assert!(reason.contains("相同参数"), "{reason}");
            }
            other => panic!("expected stop, got {other:?}"),
        }
    }

    #[test]
    fn identical_error_stop_names_blank_tool() {
        let mut breaker = FailureBreaker::new();
        let blank = || outcome("", "{}", "malformed tool call: empty tool name", false);
        breaker.check(&[blank()]);
        breaker.check(&[blank()]);
        match breaker.check(&[blank()]) {
            FailureAction::Stop { reason } => {
                assert!(reason.contains("未知工具"), "{reason}");
                assert!(!reason.contains("工具 ``"), "{reason}");
            }
            other => panic!("expected stop, got {other:?}"),
        }
    }

    #[test]
    fn consecutive_failures_challenge_then_stop() {
        let mut breaker = FailureBreaker::new();
        assert!(matches!(
            breaker.check(&[outcome("a", "1", "e1", false)]),
            FailureAction::Continue
        ));
        assert!(matches!(
            breaker.check(&[outcome("b", "2", "e2", false)]),
            FailureAction::Continue
        ));
        match breaker.check(&[outcome("c", "3", "e3", false)]) {
            FailureAction::Challenge { status_kind, message } => {
                assert_eq!(status_kind, "consecutive_failure");
                assert!(message.contains("Last errors"), "{message}");
                assert!(message.contains("`c`"), "{message}");
            }
            other => panic!("expected challenge on the third step, got {other:?}"),
        }
        for step in 4..MAX_CONSECUTIVE_TOOL_FAILURES {
            assert!(
                matches!(
                    breaker.check(&[outcome(
                        &format!("t{step}"),
                        &format!("{step}"),
                        "err",
                        false
                    )]),
                    FailureAction::Continue
                ),
                "step {step} should continue after the challenge"
            );
        }
        match breaker.check(&[outcome("stop", "x", "err", false)]) {
            FailureAction::Stop { reason } => assert!(reason.contains("连续失败")),
            other => panic!("expected stop, got {other:?}"),
        }
    }

    #[test]
    fn parallel_failures_in_one_batch_count_as_one() {
        let mut breaker = FailureBreaker::new();
        let batch = |step: u32| {
            [
                outcome(
                    "run_subagent",
                    &format!(r#"{{"prompt":"a{step}"}}"#),
                    "e1",
                    false,
                ),
                outcome(
                    "run_subagent",
                    &format!(r#"{{"prompt":"b{step}"}}"#),
                    "e2",
                    false,
                ),
                outcome(
                    "run_subagent",
                    &format!(r#"{{"prompt":"c{step}"}}"#),
                    "e3",
                    false,
                ),
            ]
        };
        assert!(
            matches!(breaker.check(&batch(1)), FailureAction::Continue),
            "three parallel failures must not trip the consecutive breaker in one step"
        );
        assert!(matches!(
            breaker.check(&batch(2)),
            FailureAction::Continue
        ));
        match breaker.check(&batch(3)) {
            FailureAction::Challenge { status_kind, .. } => {
                assert_eq!(status_kind, "consecutive_failure");
            }
            other => panic!("expected challenge on the third step, got {other:?}"),
        }
    }

    #[test]
    fn mixed_success_in_a_batch_resets_consecutive_regardless_of_order() {
        let mut breaker = FailureBreaker::new();
        assert!(matches!(
            breaker.check(&[outcome("a", "1", "e1", false)]),
            FailureAction::Continue
        ));
        assert!(matches!(
            breaker.check(&[outcome("b", "2", "e2", false)]),
            FailureAction::Continue
        ));
        // Failures first, success last — previously order-dependent.
        assert!(matches!(
            breaker.check(&[
                outcome("c", "3", "e3", false),
                outcome("d", "4", "ok", true),
            ]),
            FailureAction::Continue
        ));
        assert!(matches!(
            breaker.check(&[outcome("e", "5", "e5", false)]),
            FailureAction::Continue
        ));
        assert!(matches!(
            breaker.check(&[outcome("f", "6", "e6", false)]),
            FailureAction::Continue
        ));
        match breaker.check(&[outcome("g", "7", "e7", false)]) {
            FailureAction::Challenge { status_kind, .. } => {
                assert_eq!(status_kind, "consecutive_failure");
            }
            other => panic!("expected challenge after reset + 3 new failures, got {other:?}"),
        }
    }

    #[test]
    fn plan_gate_stops_on_first_blocked_writer() {
        let mut breaker = FailureBreaker::new();
        match breaker.check(&[outcome(
            "run_shell",
            r#"{"command":"ls"}"#,
            PLAN_GATE_BLOCKED,
            false,
        )]) {
            FailureAction::Stop { reason } => {
                assert!(reason.contains("批准"), "{reason}");
                assert!(
                    reason.contains(crate::core::tools::plan_mode::PLAN_GATE_STOP_HINT),
                    "{reason}"
                );
                assert!(!reason.contains("连续失败"), "{reason}");
                assert!(!reason.contains("换路径"), "{reason}");
            }
            other => panic!("expected immediate stop, got {other:?}"),
        }
    }

    #[test]
    fn plan_gate_does_not_invite_a_different_shell_command() {
        let mut breaker = FailureBreaker::new();
        let _ = breaker.check(&[outcome(
            "run_shell",
            r#"{"command":"a"}"#,
            PLAN_GATE_BLOCKED,
            false,
        )]);
        // A second, different command must not even be considered — first
        // blocked writer already stopped the turn.
        match breaker.check(&[outcome(
            "run_shell",
            r#"{"command":"b"}"#,
            PLAN_GATE_BLOCKED,
            false,
        )]) {
            FailureAction::Stop { reason } => assert!(reason.contains("批准"), "{reason}"),
            other => panic!("expected stop, got {other:?}"),
        }
    }
}
