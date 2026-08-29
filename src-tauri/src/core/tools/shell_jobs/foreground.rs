use super::constants::*;
use super::output::{
    drain_available, drain_until_quiet, format_duration, format_streams, settle_ambiguous,
    spawn_drain_thread,
};
use super::process::terminate_process_tree;
use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::shell_judge::{judge_shell_completion, CompletionVerdict};
use crate::runtime::encoding::decode_process_bytes;
use crate::runtime::terminal::prepare_powershell;
#[cfg(windows)]
use crate::core::tools::sandbox::restricted_process;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Guard rails for a running foreground command.
pub(crate) struct WaitPolicy {
    /// Absolute ceiling — a safety net, not the normal way a command ends.
    pub(crate) ceiling: Duration,
    /// How long a command may make no progress at all before it counts stuck.
    pub(crate) stall: Duration,
}

/// Everything the wait loop knows about a command at one tick.
pub(crate) struct WaitSnapshot {
    pub(crate) elapsed: Duration,
    /// Time since the last sign of progress: new output, or CPU time burned
    /// anywhere in the process tree.
    pub(crate) since_progress: Duration,
    /// Time since the last model consultation, if there has been one.
    pub(crate) since_judge: Option<Duration>,
    /// Current backoff interval between periodic consultations.
    pub(crate) judge_interval: Duration,
    pub(crate) judge_rounds_left: usize,
    /// Set once a consultation during the current stall failed to confirm
    /// that work is still in progress.
    pub(crate) stall_unconfirmed: bool,
    /// Whether process-tree activity can be measured at all. Without that
    /// signal a silent command is never assumed to be stuck, because "quiet"
    /// and "stuck" are indistinguishable from output alone.
    pub(crate) activity_measurable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaitAction {
    KeepWaiting,
    /// Ask the model whether the task is already finished.
    Judge,
    /// Absolute ceiling reached.
    Timeout,
    /// No output, no CPU, and no confirmation that work continues.
    Stalled,
}

/// Decide what to do with a still-running command. Progress — not elapsed
/// time — is what keeps a command alive, so a long build that keeps working
/// runs to completion while a process that does nothing at all is reclaimed
/// quickly.
pub(crate) fn next_wait_action(policy: &WaitPolicy, snap: &WaitSnapshot) -> WaitAction {
    if snap.elapsed >= policy.ceiling {
        return WaitAction::Timeout;
    }
    if snap.activity_measurable && snap.since_progress >= policy.stall {
        if snap.stall_unconfirmed || snap.judge_rounds_left == 0 {
            return WaitAction::Stalled;
        }
        return WaitAction::Judge;
    }
    let due_for_judge = snap
        .since_judge
        .map(|since| since >= snap.judge_interval)
        .unwrap_or(true);
    if snap.judge_rounds_left > 0 && snap.elapsed >= IDLE_CHECK_GRACE && due_for_judge {
        return WaitAction::Judge;
    }
    WaitAction::KeepWaiting
}

/// Runs a shell command in the foreground until it completes, stalls, or is cancelled.
pub fn run_foreground(
    command: &str,
    cwd: Option<&std::path::Path>,
    cancelled: &AtomicBool,
    judge: Option<&ToolContext>,
) -> Result<String, ToolError> {
    let policy = WaitPolicy {
        ceiling: Duration::from_secs(crate::core::tools::sandbox::shell_timeout_secs()),
        stall: Duration::from_secs(crate::core::tools::sandbox::shell_stall_timeout_secs()),
    };
    run_foreground_with_policy(command, cwd, cancelled, judge, policy)
}

pub(crate) fn run_foreground_with_policy(
    command: &str,
    cwd: Option<&std::path::Path>,
    cancelled: &AtomicBool,
    judge: Option<&ToolContext>,
    policy: WaitPolicy,
) -> Result<String, ToolError> {
    let restricted = crate::core::tools::sandbox::restricted_shell();
    let mut child = if restricted {
        #[cfg(windows)]
        {
            restricted_process::spawn_powershell(command, cwd, cancelled)?
        }
        #[cfg(not(windows))]
        {
            let mut cmd = Command::new("powershell");
            prepare_powershell(&mut cmd, command);
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }
            cmd.spawn()?
        }
    } else {
        let mut cmd = Command::new("powershell");
        prepare_powershell(&mut cmd, command);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.spawn()?
    };
    let probe = crate::core::tools::process_stats::ActivityProbe::attach(&child);
    let activity_measurable = probe.is_measurable();
    let mut cpu_at_progress = probe.cpu_time();
    let stdout_rx = spawn_drain_thread(child.stdout.take());
    let stderr_rx = spawn_drain_thread(child.stderr.take());
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let mut stdout_eof = false;
    let mut stderr_eof = false;

    let started = Instant::now();
    let mut last_progress = Instant::now();
    let mut last_judge: Option<Instant> = None;
    let mut judge_rounds_left = IDLE_JUDGE_ROUNDS;
    let mut judge_interval = IDLE_CHECK_MIN_INTERVAL;
    let mut stall_unconfirmed = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let exit_code = child.wait().ok().and_then(|status| status.code());
                drain_until_quiet(
                    &stdout_rx,
                    &stderr_rx,
                    &mut stdout_bytes,
                    &mut stderr_bytes,
                    &mut stdout_eof,
                    &mut stderr_eof,
                );
                let note = if stdout_eof && stderr_eof {
                    String::new()
                } else {
                    settle_ambiguous(
                        judge,
                        command,
                        &stdout_rx,
                        &stderr_rx,
                        &mut stdout_bytes,
                        &mut stderr_bytes,
                        &mut stdout_eof,
                        &mut stderr_eof,
                    )
                };
                let result = format!(
                    "exit_code: {}\nduration: {}\n{}{}",
                    exit_code.unwrap_or(-1),
                    format_duration(started.elapsed()),
                    format_streams(
                        &decode_process_bytes(&stdout_bytes),
                        &decode_process_bytes(&stderr_bytes),
                    ),
                    note,
                );
                crate::core::context::provider::environment_provider::record_shell_execution(
                    command, cwd, &result,
                );
                return Ok(result);
            }
            Ok(None) => {
                if cancelled.load(Ordering::Relaxed) {
                    terminate_process_tree(&mut child);
                    let _ = child.wait();
                    tracing::debug!(pid = child.id(), "foreground shell command cancelled");
                    return Err(ToolError::cancelled());
                }
                if drain_available(
                    &stdout_rx,
                    &stderr_rx,
                    &mut stdout_bytes,
                    &mut stderr_bytes,
                    &mut stdout_eof,
                    &mut stderr_eof,
                ) {
                    last_progress = Instant::now();
                    stall_unconfirmed = false;
                    cpu_at_progress = probe.cpu_time();
                } else if let (Some(now_cpu), Some(baseline)) = (probe.cpu_time(), cpu_at_progress)
                {
                    if now_cpu.saturating_sub(baseline)
                        >= crate::core::tools::process_stats::CPU_PROGRESS_EPSILON
                    {
                        last_progress = Instant::now();
                        stall_unconfirmed = false;
                        cpu_at_progress = Some(now_cpu);
                    }
                }

                let action = next_wait_action(
                    &policy,
                    &WaitSnapshot {
                        elapsed: started.elapsed(),
                        since_progress: last_progress.elapsed(),
                        since_judge: last_judge.map(|at| at.elapsed()),
                        judge_interval,
                        judge_rounds_left: if judge.is_some() {
                            judge_rounds_left
                        } else {
                            0
                        },
                        stall_unconfirmed,
                        activity_measurable,
                    },
                );
                match action {
                    WaitAction::KeepWaiting => std::thread::sleep(WAIT_POLL),
                    WaitAction::Judge => {
                        let ctx = match judge {
                            Some(ctx) => ctx,
                            None => {
                                std::thread::sleep(WAIT_POLL);
                                continue;
                            }
                        };
                        last_judge = Some(Instant::now());
                        judge_rounds_left = judge_rounds_left.saturating_sub(1);
                        judge_interval = std::cmp::min(judge_interval * 2, IDLE_CHECK_MAX_INTERVAL);
                        let output = format_streams(
                            &decode_process_bytes(&stdout_bytes),
                            &decode_process_bytes(&stderr_bytes),
                        );
                        match judge_shell_completion(ctx, command, &output) {
                            CompletionVerdict::Finished => {
                                terminate_process_tree(&mut child);
                                let exit_code = child.wait().ok().and_then(|status| status.code());
                                drain_until_quiet(
                                    &stdout_rx,
                                    &stderr_rx,
                                    &mut stdout_bytes,
                                    &mut stderr_bytes,
                                    &mut stdout_eof,
                                    &mut stderr_eof,
                                );
                                let result = format!(
                                    "exit_code: {}\nduration: {}\n{}{}",
                                    exit_code.unwrap_or(-1),
                                    format_duration(started.elapsed()),
                                    format_streams(
                                        &decode_process_bytes(&stdout_bytes),
                                        &decode_process_bytes(&stderr_bytes),
                                    ),
                                    NOTE_IDLE_FINISHED,
                                );
                                crate::core::context::provider::environment_provider::record_shell_execution(
                                    command, cwd, &result,
                                );
                                return Ok(result);
                            }
                            CompletionVerdict::Running => {
                                last_progress = Instant::now();
                                stall_unconfirmed = false;
                                cpu_at_progress = probe.cpu_time();
                            }
                            CompletionVerdict::Unknown => stall_unconfirmed = true,
                        }
                    }
                    WaitAction::Timeout | WaitAction::Stalled => {
                        terminate_process_tree(&mut child);
                        let exit_code = child.wait().ok().and_then(|status| status.code());
                        drain_until_quiet(
                            &stdout_rx,
                            &stderr_rx,
                            &mut stdout_bytes,
                            &mut stderr_bytes,
                            &mut stdout_eof,
                            &mut stderr_eof,
                        );
                        let reason = if action == WaitAction::Stalled {
                            format!(
                                "command made no progress for {} and was stopped",
                                format_duration(policy.stall),
                            )
                        } else {
                            format!(
                                "command hit the {} ceiling and was stopped",
                                format_duration(policy.ceiling),
                            )
                        };
                        let note = if action == WaitAction::Stalled {
                            NOTE_STALLED
                        } else {
                            ""
                        };
                        let result = format!(
                            "{reason} (duration: {}, exit_code: {:?})\n{}{}",
                            format_duration(started.elapsed()),
                            exit_code,
                            format_streams(
                                &decode_process_bytes(&stdout_bytes),
                                &decode_process_bytes(&stderr_bytes),
                            ),
                            note,
                        );
                        crate::core::context::provider::environment_provider::record_shell_execution(
                            command, cwd, &result,
                        );
                        return Err(ToolError::new(result));
                    }
                }
            }
            Err(error) => return Err(ToolError::new(error.to_string())),
        }
    }
}
