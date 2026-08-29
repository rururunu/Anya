use std::time::Duration;

pub(super) const WAIT_POLL: Duration = Duration::from_millis(100);
pub(super) const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const BACKGROUND_OUTPUT_MAX_CHARS: usize = 256 * 1024;
pub(super) const BACKGROUND_OUTPUT_MAX_BYTES: usize = BACKGROUND_OUTPUT_MAX_CHARS * 4;
pub(super) const DRAIN_GRACE: Duration = Duration::from_secs(2);
pub(super) const JUDGE_RECHECK: Duration = Duration::from_secs(5);
pub(super) const JUDGE_ROUNDS: usize = 2;
pub(super) const NOTE_JUDGED_FINISHED: &str =
    "\n[note: process exited but a child process still holds the output pipe; model judged the task finished]";
pub(super) const NOTE_JUDGED_RUNNING: &str =
    "\n[note: process exited but a child process still holds the output pipe; model judged the task still running]";
pub(super) const NOTE_JUDGED_UNKNOWN: &str =
    "\n[note: process exited but a child process still holds the output pipe; completion could not be confirmed]";
pub(super) const IDLE_CHECK_GRACE: Duration = Duration::from_secs(15);
pub(super) const IDLE_CHECK_MIN_INTERVAL: Duration = Duration::from_secs(5);
pub(super) const IDLE_CHECK_MAX_INTERVAL: Duration = Duration::from_secs(60);
pub(super) const IDLE_JUDGE_ROUNDS: usize = 12;
pub(super) const NOTE_IDLE_FINISHED: &str = "\n[note: the process had not exited, but its output showed the task was already done; model judged it finished and the lingering process was terminated]";
pub(super) const NOTE_STALLED: &str = "\n[note: no new output and no CPU activity anywhere in the process tree, and completion could not be confirmed — the command was treated as stuck. If it was in fact waiting on something slow, raise shell_stall_timeout_secs.]";
