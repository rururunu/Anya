use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::encoding::decode_process_bytes;
use crate::runtime::terminal::{prepare_command, prepare_powershell};

use super::error::ToolError;
use crate::core::tools::context::ToolContext;
use crate::core::tools::shell_judge::{judge_shell_completion, CompletionVerdict};
#[cfg(windows)]
use crate::core::tools::sandbox::restricted_process;

const WAIT_POLL: Duration = Duration::from_millis(100);
const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
const BACKGROUND_OUTPUT_MAX_CHARS: usize = 256 * 1024;
const BACKGROUND_OUTPUT_MAX_BYTES: usize = BACKGROUND_OUTPUT_MAX_CHARS * 4;
/// How long to keep draining output after the direct child exits before
/// treating "no EOF yet" as ambiguous (a descendant may hold the pipe).
const DRAIN_GRACE: Duration = Duration::from_secs(2);
/// Pause between model completion checks when the verdict is "still running".
const JUDGE_RECHECK: Duration = Duration::from_secs(5);
/// Max model consultations per job; caps worst-case latency and token spend.
const JUDGE_ROUNDS: usize = 2;
const NOTE_JUDGED_FINISHED: &str =
    "\n[note: process exited but a child process still holds the output pipe; model judged the task finished]";
const NOTE_JUDGED_RUNNING: &str =
    "\n[note: process exited but a child process still holds the output pipe; model judged the task still running]";
const NOTE_JUDGED_UNKNOWN: &str =
    "\n[note: process exited but a child process still holds the output pipe; completion could not be confirmed]";
/// Grace period before the first idle-completion check on a still-running
/// foreground child. Some wrapper processes spawn a long-lived background
/// service and then linger instead of exiting once the real work is done —
/// so a child that hasn't exited isn't proof the task is still in progress.
/// Waiting this long avoids judging ordinary fast commands.
const IDLE_CHECK_GRACE: Duration = Duration::from_secs(15);
/// Re-check interval after the grace period, doubling each round up to
/// [`IDLE_CHECK_MAX_INTERVAL`]. Deliberately time-based rather than
/// silence-based: a lingering process can keep producing low-signal repeated
/// output (status polling, keep-alive lines) that would otherwise reset a
/// "quiet for N seconds" timer forever and mask real completion. The model
/// judge reads the actual content and decides — the timer only controls how
/// often we bother asking.
const IDLE_CHECK_MIN_INTERVAL: Duration = Duration::from_secs(5);
const IDLE_CHECK_MAX_INTERVAL: Duration = Duration::from_secs(60);
/// Max idle-triggered model consultations per foreground run; independent of
/// [`JUDGE_ROUNDS`] (which only applies after the direct child has exited).
/// With backoff up to [`IDLE_CHECK_MAX_INTERVAL`] this comfortably spans
/// multi-minute commands without excessive model calls.
const IDLE_JUDGE_ROUNDS: usize = 12;
const NOTE_IDLE_FINISHED: &str = "\n[note: the process had not exited, but its output showed the task was already done; model judged it finished and the lingering process was terminated]";
const NOTE_STALLED: &str = "\n[note: no new output and no CPU activity anywhere in the process tree, and completion could not be confirmed — the command was treated as stuck. If it was in fact waiting on something slow, raise shell_stall_timeout_secs.]";

#[derive(Debug)]
pub struct ShellJob {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub command: String,
    pub output: String,
    raw_output: Vec<u8>,
    pub done: bool,
    pub exit_code: Option<i32>,
    cwd: Option<std::path::PathBuf>,
    child: Option<Child>,
    cancelled: Arc<AtomicBool>,
    started: Instant,
    /// When output last arrived. Reported alongside the status so a caller can
    /// tell "running and working" from "running but silent for a long time"
    /// without guessing from a stale view of the log.
    last_output_at: Option<Instant>,
}

pub struct ShellJobStore {
    jobs: Mutex<HashMap<String, ShellJob>>,
    next_id: Mutex<u32>,
}

impl ShellJobStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        })
    }

    pub fn spawn_background(
        self: &Arc<Self>,
        command: String,
        cwd: Option<&std::path::Path>,
        cancelled: Arc<AtomicBool>,
        judge: Option<ToolContext>,
    ) -> Result<String, ToolError> {
        // Funnel restricted execution through a single spawn boundary so we can
        // swap the implementation (token + CreateProcessAsUserW) without
        // touching the job-output plumbing.
        let mut child = if crate::core::tools::sandbox::restricted_shell() {
            #[cfg(windows)]
            {
            restricted_process::spawn_powershell(&command, cwd, &cancelled)?
            }
            #[cfg(not(windows))]
            {
                let mut cmd = Command::new("powershell");
                prepare_powershell(&mut cmd, &command);
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
            prepare_powershell(&mut cmd, &command);
            // Never inherit stdin: this process runs headless (no console), so an
            // inherited/undefined stdin handle can make some child launchers
            // (notably JVM-based CLI wrappers) block during shutdown instead of
            // exiting once their real work is done.
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }
            cmd.spawn()?
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let id = {
            let mut guard = self
                .next_id
                .lock()
                .map_err(|_| ToolError::new("job lock"))?;
            let id = format!("job-{}", *guard);
            *guard += 1;
            id
        };

        // Insert before spawning the waiter — otherwise finish_job can miss the
        // entry and leave the job stuck as running forever.
        {
            let mut guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
            guard.insert(
                id.clone(),
                ShellJob {
                    id: id.clone(),
                    command,
                    output: String::new(),
                    raw_output: Vec::new(),
                    done: false,
                    exit_code: None,
                    cwd: cwd.map(std::path::Path::to_path_buf),
                    child: Some(child),
                    cancelled,
                    started: Instant::now(),
                    last_output_at: None,
                },
            );
        }

        let readers_done = Arc::new(AtomicBool::new(false));
        let stdout_reader = stdout.map(|stream| {
            spawn_output_reader(
                Arc::clone(self),
                id.clone(),
                stream,
                Arc::clone(&readers_done),
            )
        });
        let stderr_reader = stderr.map(|stream| {
            spawn_output_reader(
                Arc::clone(self),
                id.clone(),
                stream,
                Arc::clone(&readers_done),
            )
        });
        let store = Arc::clone(self);
        let job_id = id.clone();
        std::thread::spawn(move || {
            store.finish_job(&job_id, stdout_reader, stderr_reader, readers_done, judge);
        });

        Ok(id)
    }

    /// Take the child under a short lock, wait outside the lock, then publish.
    fn finish_job(
        &self,
        job_id: &str,
        stdout_reader: Option<std::thread::JoinHandle<()>>,
        stderr_reader: Option<std::thread::JoinHandle<()>>,
        readers_done: Arc<AtomicBool>,
        judge: Option<ToolContext>,
    ) {
        let (mut child, cancelled, command) = {
            let mut guard = match self.jobs.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let job = match guard.get_mut(job_id) {
                Some(job) => job,
                None => return,
            };
            match job.child.take() {
                Some(child) => (child, Arc::clone(&job.cancelled), job.command.clone()),
                None => return,
            }
        };

        let was_cancelled = loop {
            if cancelled.load(Ordering::Relaxed) {
                terminate_process_tree(&mut child);
                break true;
            }
            match child.try_wait() {
                Ok(Some(_)) => break false,
                Ok(None) => std::thread::sleep(WAIT_POLL),
                Err(_) => break false,
            }
        };
        let exit_code = child.wait().ok().and_then(|status| status.code());

        // The direct child exited, but a descendant (daemon, detached worker)
        // may still hold the output pipe, so EOF may never arrive. Give the
        // readers a short grace window; if they finish, the job is clean.
        let mut clean = false;
        let grace_deadline = Instant::now() + DRAIN_GRACE;
        while Instant::now() < grace_deadline {
            if readers_done.load(Ordering::Relaxed) {
                clean = true;
                break;
            }
            std::thread::sleep(WAIT_POLL);
        }
        let mut note = String::new();
        if !clean {
            let mut rounds = 0;
            while rounds < JUDGE_ROUNDS {
                if readers_done.load(Ordering::Relaxed) {
                    clean = true;
                    break;
                }
                rounds += 1;
                let verdict = match &judge {
                    Some(ctx) => {
                        let output = {
                            let guard = match self.jobs.lock() {
                                Ok(guard) => guard,
                                Err(_) => return,
                            };
                            guard.get(job_id).map(|job| job.output.clone()).unwrap_or_default()
                        };
                        judge_shell_completion(ctx, &command, &output)
                    }
                    None => CompletionVerdict::Unknown,
                };
                match verdict {
                    CompletionVerdict::Finished => {
                        note = NOTE_JUDGED_FINISHED.to_string();
                        break;
                    }
                    CompletionVerdict::Running if rounds < JUDGE_ROUNDS => {
                        std::thread::sleep(JUDGE_RECHECK);
                    }
                    CompletionVerdict::Running => {
                        note = NOTE_JUDGED_RUNNING.to_string();
                        break;
                    }
                    CompletionVerdict::Unknown => {
                        note = NOTE_JUDGED_UNKNOWN.to_string();
                        break;
                    }
                }
            }
        }

        if clean {
            if let Some(reader) = stdout_reader {
                let _ = reader.join();
            }
            if let Some(reader) = stderr_reader {
                let _ = reader.join();
            }
        }
        // Otherwise the readers are detached on drop and keep draining into the
        // store (bounded buffers) until the pipe closes; done is already
        // observable, so callers never hang on EOF.

        if let Ok(mut guard) = self.jobs.lock() {
            if let Some(job) = guard.get_mut(job_id) {
                if was_cancelled {
                    append_bounded(&mut job.output, "\n[cancelled]\n");
                }
                if !note.is_empty() {
                    append_bounded(&mut job.output, &note);
                }
                job.exit_code = exit_code;
                job.done = true;
                crate::core::context::provider::environment_provider::record_shell_execution(
                    &job.command,
                    job.cwd.as_deref(),
                    &format_job_status(job),
                );
            }
        }
    }

    pub fn read_output_limited(
        &self,
        job_id: &str,
        tail_lines: Option<usize>,
        max_chars: Option<usize>,
    ) -> Result<String, ToolError> {
        let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        let mut output = job.output.clone();
        if let Some(lines) = tail_lines.filter(|value| *value > 0) {
            let all: Vec<&str> = output.lines().collect();
            output = all[all.len().saturating_sub(lines)..].join("\n");
        }
        if let Some(limit) = max_chars.filter(|value| *value > 0) {
            output = take_tail_chars(&output, limit);
        }
        Ok(format_job_status_with_output(job, &output))
    }

    pub fn wait_job(
        &self,
        job_id: &str,
        context: &crate::core::tools::context::ToolContext,
    ) -> Result<String, ToolError> {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            context.ensure_not_cancelled()?;
            {
                let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
                let job = guard
                    .get(job_id)
                    .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
                if job.done {
                    return Ok(format_job_status(job));
                }
            }
            if Instant::now() >= deadline {
                let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
                let job = guard
                    .get(job_id)
                    .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
                return Ok(format!(
                    "status: still running after waiting {}s\n{}",
                    WAIT_TIMEOUT.as_secs(),
                    format_job_status_with_output(job, &job.output),
                ));
            }
            std::thread::sleep(WAIT_POLL);
        }
    }

    pub fn kill(&self, job_id: &str) -> Result<String, ToolError> {
        let mut guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get_mut(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        if let Some(mut child) = job.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        job.done = true;
        if job.exit_code.is_none() {
            job.exit_code = Some(-1);
        }
        Ok("killed".into())
    }
}

fn format_job_status(job: &ShellJob) -> String {
    format_job_status_with_output(job, &job.output)
}

fn format_job_status_with_output(job: &ShellJob, output: &str) -> String {
    let mut header = format!(
        "status: {}\nexit_code: {:?}\nelapsed: {}",
        if job.done { "done" } else { "running" },
        job.exit_code,
        format_duration(job.started.elapsed()),
    );
    if !job.done {
        let silence = match job.last_output_at {
            Some(at) => format!("{} ago", format_duration(at.elapsed())),
            None => "never".to_string(),
        };
        header.push_str(&format!("\nlast_output: {silence}"));
    }
    format!("{header}\n{output}")
}

fn spawn_output_reader<R: Read + Send + 'static>(
    store: Arc<ShellJobStore>,
    job_id: String,
    mut stream: R,
    done: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if let Ok(mut jobs) = store.jobs.lock() {
                        if let Some(job) = jobs.get_mut(&job_id) {
                            append_raw_bounded(job, &buffer[..read]);
                        }
                    }
                }
                Err(_) => break,
            }
        }
        done.store(true, Ordering::Relaxed);
    })
}

fn append_raw_bounded(job: &mut ShellJob, chunk: &[u8]) {
    job.last_output_at = Some(Instant::now());
    job.raw_output.extend_from_slice(chunk);
    if job.raw_output.len() > BACKGROUND_OUTPUT_MAX_BYTES {
        let keep = BACKGROUND_OUTPUT_MAX_BYTES;
        let drain = job.raw_output.len() - keep;
        job.raw_output.drain(..drain);
    }
    job.output = decode_process_bytes(&job.raw_output);
    let count = job.output.chars().count();
    if count > BACKGROUND_OUTPUT_MAX_CHARS {
        job.output = take_tail_chars(&job.output, BACKGROUND_OUTPUT_MAX_CHARS);
    }
}

fn append_bounded(output: &mut String, chunk: &str) {
    output.push_str(chunk);
    let count = output.chars().count();
    if count > BACKGROUND_OUTPUT_MAX_CHARS {
        *output = take_tail_chars(output, BACKGROUND_OUTPUT_MAX_CHARS);
    }
}

fn take_tail_chars(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    value.chars().skip(count.saturating_sub(limit)).collect()
}

/// Only commands expected to stay alive are allowed to allocate a background
/// job. This prevents routine reads, tests, builds, Git, and Docker inspection
/// from being turned into noisy `job-n` handles merely to avoid waiting.
pub fn background_allowed(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let persistent_markers = [
        "get-content -wait",
        "tail -f",
        "docker logs -f",
        "docker logs --follow",
        "docker compose logs -f",
        "docker compose logs --follow",
        "docker-compose logs -f",
        "docker-compose logs --follow",
        "npm run dev",
        "pnpm dev",
        "yarn dev",
        "bun run dev",
        "vite --host",
        "webpack --watch",
        "cargo watch",
        "dotnet watch",
    ];
    if persistent_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    let trimmed = normalized.trim();
    (trimmed.starts_with("docker compose up")
        || trimmed.starts_with("docker-compose up")
        || trimmed.starts_with("docker run"))
        && !normalized
            .split_whitespace()
            .any(|part| part == "-d" || part == "--detach")
}

fn format_streams(stdout: &str, stderr: &str) -> String {
    format!("stdout:\n{stdout}\nstderr:\n{stderr}")
}

enum DrainMsg {
    Chunk(Vec<u8>),
    Eof,
}

/// Move a stream into a thread that forwards chunks (and EOF) over a channel.
/// If no stream was provided, EOF is reported immediately. The thread stays
/// alive while a descendant holds the pipe; after the receiver is dropped it
/// exits on the next successful read (send fails).
fn spawn_drain_thread<R: Read + Send + 'static>(
    stream: Option<R>,
) -> std::sync::mpsc::Receiver<DrainMsg> {
    let (tx, rx) = std::sync::mpsc::channel::<DrainMsg>();
    match stream {
        Some(mut stream) => {
            std::thread::spawn(move || {
                let mut buffer = [0u8; 4096];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(read) => {
                            if tx.send(DrainMsg::Chunk(buffer[..read].to_vec())).is_err() {
                                return;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = tx.send(DrainMsg::Eof);
            });
        }
        None => {
            let _ = tx.send(DrainMsg::Eof);
        }
    }
    rx
}

/// Collect whatever output has arrived within [`DRAIN_GRACE`], tracking EOF
/// per stream. Returns once both streams hit EOF or the window elapses.
fn drain_until_quiet(
    stdout_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stderr_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stdout_bytes: &mut Vec<u8>,
    stderr_bytes: &mut Vec<u8>,
    stdout_eof: &mut bool,
    stderr_eof: &mut bool,
) {
    let deadline = Instant::now() + DRAIN_GRACE;
    while !(*stdout_eof && *stderr_eof) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let step = std::cmp::min(remaining, Duration::from_millis(50));
        if !*stdout_eof {
            match stdout_rx.recv_timeout(step) {
                Ok(DrainMsg::Chunk(bytes)) => stdout_bytes.extend_from_slice(&bytes),
                Ok(DrainMsg::Eof) => *stdout_eof = true,
                Err(_) => {}
            }
        }
        if !*stderr_eof {
            loop {
                match stderr_rx.try_recv() {
                    Ok(DrainMsg::Chunk(bytes)) => stderr_bytes.extend_from_slice(&bytes),
                    Ok(DrainMsg::Eof) => {
                        *stderr_eof = true;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

/// Drain whatever has already arrived without blocking, tracking per-stream
/// EOF and bounding total retained bytes. Returns whether any new bytes were
/// read, so callers can reset an idle timer.
fn drain_available(
    stdout_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stderr_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stdout_bytes: &mut Vec<u8>,
    stderr_bytes: &mut Vec<u8>,
    stdout_eof: &mut bool,
    stderr_eof: &mut bool,
) -> bool {
    let mut activity = false;
    if !*stdout_eof {
        loop {
            match stdout_rx.try_recv() {
                Ok(DrainMsg::Chunk(bytes)) => {
                    stdout_bytes.extend_from_slice(&bytes);
                    activity = true;
                }
                Ok(DrainMsg::Eof) => {
                    *stdout_eof = true;
                    break;
                }
                Err(_) => break,
            }
        }
    }
    if !*stderr_eof {
        loop {
            match stderr_rx.try_recv() {
                Ok(DrainMsg::Chunk(bytes)) => {
                    stderr_bytes.extend_from_slice(&bytes);
                    activity = true;
                }
                Ok(DrainMsg::Eof) => {
                    *stderr_eof = true;
                    break;
                }
                Err(_) => break,
            }
        }
    }
    cap_tail_bytes(stdout_bytes);
    cap_tail_bytes(stderr_bytes);
    activity
}

/// Keep only the tail of a growing buffer so a long-idle foreground command
/// can't accumulate unbounded memory before the model gets a chance to judge
/// completion.
fn cap_tail_bytes(buffer: &mut Vec<u8>) {
    if buffer.len() > BACKGROUND_OUTPUT_MAX_BYTES {
        let drain = buffer.len() - BACKGROUND_OUTPUT_MAX_BYTES;
        buffer.drain(..drain);
    }
}

/// The direct child exited but a descendant still holds the output pipe.
/// Ask the model (when available) whether the task is done; otherwise fall
/// back to publishing what we have. Bounded by [`JUDGE_ROUNDS`] consultations.
fn settle_ambiguous(
    judge: Option<&ToolContext>,
    command: &str,
    stdout_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stderr_rx: &std::sync::mpsc::Receiver<DrainMsg>,
    stdout_bytes: &mut Vec<u8>,
    stderr_bytes: &mut Vec<u8>,
    stdout_eof: &mut bool,
    stderr_eof: &mut bool,
) -> String {
    let mut rounds = 0;
    loop {
        if *stdout_eof && *stderr_eof {
            return String::new();
        }
        rounds += 1;
        let verdict = match judge {
            Some(ctx) => {
                let output = format_streams(
                    &decode_process_bytes(stdout_bytes),
                    &decode_process_bytes(stderr_bytes),
                );
                judge_shell_completion(ctx, command, &output)
            }
            None => CompletionVerdict::Unknown,
        };
        match verdict {
            CompletionVerdict::Finished => return NOTE_JUDGED_FINISHED.to_string(),
            CompletionVerdict::Running if rounds < JUDGE_ROUNDS => {
                std::thread::sleep(JUDGE_RECHECK);
                drain_until_quiet(
                    stdout_rx,
                    stderr_rx,
                    stdout_bytes,
                    stderr_bytes,
                    stdout_eof,
                    stderr_eof,
                );
            }
            CompletionVerdict::Running => return NOTE_JUDGED_RUNNING.to_string(),
            CompletionVerdict::Unknown => return NOTE_JUDGED_UNKNOWN.to_string(),
        }
    }
}

/// Guard rails for a running foreground command.
struct WaitPolicy {
    /// Absolute ceiling — a safety net, not the normal way a command ends.
    ceiling: Duration,
    /// How long a command may make no progress at all before it counts stuck.
    stall: Duration,
}

/// Everything the wait loop knows about a command at one tick.
struct WaitSnapshot {
    elapsed: Duration,
    /// Time since the last sign of progress: new output, or CPU time burned
    /// anywhere in the process tree.
    since_progress: Duration,
    /// Time since the last model consultation, if there has been one.
    since_judge: Option<Duration>,
    /// Current backoff interval between periodic consultations.
    judge_interval: Duration,
    judge_rounds_left: usize,
    /// Set once a consultation during the current stall failed to confirm
    /// that work is still in progress.
    stall_unconfirmed: bool,
    /// Whether process-tree activity can be measured at all. Without that
    /// signal a silent command is never assumed to be stuck, because "quiet"
    /// and "stuck" are indistinguishable from output alone.
    activity_measurable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaitAction {
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
fn next_wait_action(policy: &WaitPolicy, snap: &WaitSnapshot) -> WaitAction {
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

fn format_duration(value: Duration) -> String {
    if value < Duration::from_secs(1) {
        format!("{}ms", value.as_millis())
    } else {
        format!("{:.1}s", value.as_secs_f64())
    }
}

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

fn run_foreground_with_policy(
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
        // See spawn_background: never inherit stdin for headless child processes.
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.spawn()?
    };
    // Accounting only — this never kills anything, so background services the
    // command starts on purpose keep running after it returns.
    let probe = crate::core::tools::process_stats::ActivityProbe::attach(&child);
    let activity_measurable = probe.is_measurable();
    let mut cpu_at_progress = probe.cpu_time();
    // Take the pipes immediately: a large output can otherwise fill the pipe
    // buffer and stall the child before we ever call read.
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
                } else if let (Some(now_cpu), Some(baseline)) = (probe.cpu_time(), cpu_at_progress) {
                    // Silent but busy: a command can compile or link for
                    // minutes without printing anything, so CPU burned
                    // anywhere in the tree counts as progress too.
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
                        judge_rounds_left: if judge.is_some() { judge_rounds_left } else { 0 },
                        stall_unconfirmed,
                        activity_measurable,
                    },
                );
                match action {
                    WaitAction::KeepWaiting => std::thread::sleep(WAIT_POLL),
                    WaitAction::Judge => {
                        // The child being alive doesn't prove the task is
                        // still in progress: a wrapper process can spawn a
                        // long-lived service, collect its result, and then
                        // linger. Let the model read the actual output and
                        // decide, on a backoff so low-signal keep-alive
                        // output can neither trigger nor postpone this.
                        let ctx = match judge {
                            Some(ctx) => ctx,
                            None => {
                                std::thread::sleep(WAIT_POLL);
                                continue;
                            }
                        };
                        last_judge = Some(Instant::now());
                        judge_rounds_left = judge_rounds_left.saturating_sub(1);
                        judge_interval =
                            std::cmp::min(judge_interval * 2, IDLE_CHECK_MAX_INTERVAL);
                        let output = format_streams(
                            &decode_process_bytes(&stdout_bytes),
                            &decode_process_bytes(&stderr_bytes),
                        );
                        match judge_shell_completion(ctx, command, &output) {
                            CompletionVerdict::Finished => {
                                terminate_process_tree(&mut child);
                                let exit_code =
                                    child.wait().ok().and_then(|status| status.code());
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
                            // Work continues: grant a fresh stall window.
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

pub(crate) fn terminate_process_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        prepare_command(&mut command);
        let _ = command.status();
    }
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn test_context() -> (ToolContext, std::path::PathBuf) {
        let db_path =
            std::env::temp_dir().join(format!("peek-shell-job-{}.db", uuid::Uuid::new_v4()));
        struct NullBus;
        impl crate::core::event::EventBus for NullBus {
            fn emit(&self, _event: crate::core::event::BusEvent) {}
        }
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: Default::default(),
            session_id: "test".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(
                crate::core::chat::conversation_manager::ConversationManager::new(db_path.clone()),
            ),
            event_bus: Arc::new(NullBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(crate::core::tools::context::AskStore::new()),
            path_permission_store: Arc::new(crate::core::tools::context::PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        (context, db_path)
    }

    /// Fake provider for judge tests: always answers with a fixed verdict
    /// token, regardless of the prompt content.
    struct FixedVerdictProvider {
        token: &'static str,
    }

    #[async_trait::async_trait]
    impl crate::core::ai::provider::AIProvider for FixedVerdictProvider {
        fn id(&self) -> &'static str {
            "fixed-verdict-test-provider"
        }

        async fn stream(
            &self,
            _request: crate::core::runtime::ChatRequest,
            tx: tokio::sync::mpsc::Sender<crate::core::runtime::StreamEvent>,
        ) -> Result<(), crate::core::ai::provider::ProviderError> {
            let _ = tx
                .send(crate::core::runtime::StreamEvent::TurnComplete {
                    content: self.token.to_string(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    finish_reason: None,
                })
                .await;
            Ok(())
        }
    }

    fn context_with_verdict(token: &'static str) -> (ToolContext, std::path::PathBuf) {
        let (mut context, db_path) = test_context();
        context.provider = Some(Arc::new(FixedVerdictProvider { token }));
        (context, db_path)
    }

    #[test]
    fn background_job_is_registered_before_waiter_runs() {
        let store = ShellJobStore::new();
        // A quick command — race used to drop the waiter before insert.
        let id = store
            .spawn_background(
                "Write-Output 'ok'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");
        let (context, db_path) = test_context();
        let status = store.wait_job(&id, &context).expect("wait");
        assert!(
            status.contains("status: done") || status.contains("exit_code:"),
            "unexpected status: {status}"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn foreground_command_stops_when_cancelled() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let started = Instant::now();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            signal.store(true, Ordering::Relaxed);
        });

        let error =
            run_foreground("Start-Sleep -Seconds 30", None, &cancelled, None).unwrap_err();
        canceller.join().unwrap();

        assert!(error.is_cancelled());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn finite_commands_are_not_allowed_in_background() {
        assert!(!background_allowed("git status"));
        assert!(!background_allowed("pnpm build"));
        assert!(!background_allowed("docker compose ps"));
        assert!(!background_allowed("docker compose logs --tail 100"));
        assert!(background_allowed("docker compose logs -f --tail 100"));
        assert!(background_allowed("Get-Content -Wait -Tail 100 app.log"));
    }

    #[test]
    fn background_output_is_readable_before_process_exits() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                "Write-Output 'first'; Start-Sleep -Milliseconds 800; Write-Output 'second'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");

        let deadline = Instant::now() + Duration::from_secs(3);
        let running = loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("first") {
                break status;
            }
            assert!(Instant::now() < deadline, "first log line was not streamed");
            std::thread::sleep(Duration::from_millis(25));
        };
        assert!(running.contains("status: running"));

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("status: done") {
                assert!(status.contains("first"));
                assert!(status.contains("second"));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background command did not finish"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Windows regression: a wrapper script's direct process exits while a
    /// background service it spawned keeps the output pipe open, so EOF
    /// never arrives. Foreground must still return promptly with the output
    /// collected so far.
    #[test]
    fn foreground_returns_when_descendant_holds_pipe() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let command = concat!(
            "Write-Output 'start'; ",
            "cmd /c \"start /b powershell -NoProfile -Command Start-Sleep -Seconds 300\"; ",
            "Start-Sleep -Milliseconds 500; Write-Output 'done'"
        );
        let result = run_foreground(command, None, &cancelled, None).expect("run");
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "foreground command stuck on held pipe: {}s",
            started.elapsed().as_secs()
        );
        assert!(result.contains("done"), "missing final output: {result}");
        assert!(result.contains("note:"), "expected held-pipe note: {result}");
    }

    /// A direct child that never exits on its own (e.g. a wrapper process
    /// whose real work is already done but which keeps running) must still
    /// be recognized as finished once the model judge says so — well before
    /// the hard `shell_timeout_secs` ceiling.
    #[test]
    fn foreground_recovers_when_lingering_process_is_judged_finished() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let (context, db_path) = context_with_verdict("FINISHED");
        // Prints its result immediately, then hangs well past any
        // grace/backoff window instead of exiting.
        let result = run_foreground(
            "Write-Output 'build succeeded'; Start-Sleep -Seconds 300",
            None,
            &cancelled,
            Some(&context),
        )
        .expect("run");
        assert!(
            started.elapsed() < Duration::from_secs(45),
            "lingering process was not reclaimed promptly: {}s",
            started.elapsed().as_secs()
        );
        assert!(result.contains("build succeeded"), "missing output: {result}");
        assert!(
            result.contains("note:"),
            "expected idle-completion note: {result}"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    /// While the model judge keeps saying the task is still running, the
    /// process must not be killed early — it should keep waiting.
    #[test]
    fn foreground_keeps_waiting_while_judged_running() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (context, db_path) = context_with_verdict("RUNNING");
        let started = Instant::now();
        let result = run_foreground(
            "Write-Output 'compiling'; Start-Sleep -Milliseconds 900",
            None,
            &cancelled,
            Some(&context),
        )
        .expect("run");
        // The command finishes on its own well before any idle check would
        // fire, so the RUNNING verdict is never even consulted.
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(result.contains("compiling"));
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    fn snapshot() -> WaitSnapshot {
        WaitSnapshot {
            elapsed: Duration::from_secs(1),
            since_progress: Duration::from_secs(1),
            since_judge: None,
            judge_interval: IDLE_CHECK_MIN_INTERVAL,
            judge_rounds_left: IDLE_JUDGE_ROUNDS,
            stall_unconfirmed: false,
            activity_measurable: true,
        }
    }

    fn policy() -> WaitPolicy {
        WaitPolicy {
            ceiling: Duration::from_secs(3600),
            stall: Duration::from_secs(120),
        }
    }

    #[test]
    fn a_command_making_progress_is_left_alone() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(900),
            since_progress: Duration::from_secs(2),
            since_judge: Some(Duration::from_secs(1)),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &snap),
            WaitAction::KeepWaiting,
            "a long command that keeps working must not be interrupted"
        );
    }

    #[test]
    fn the_ceiling_wins_over_everything_else() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(3600),
            since_progress: Duration::from_millis(10),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &snap), WaitAction::Timeout);
    }

    #[test]
    fn a_stall_asks_the_model_before_giving_up() {
        let stalled = WaitSnapshot {
            elapsed: Duration::from_secs(300),
            since_progress: Duration::from_secs(120),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &stalled), WaitAction::Judge);

        // Consulted, and completion could not be confirmed.
        let unconfirmed = WaitSnapshot {
            stall_unconfirmed: true,
            ..stalled
        };
        assert_eq!(next_wait_action(&policy(), &unconfirmed), WaitAction::Stalled);

        // No judge available (or rounds exhausted): the stall alone decides.
        let no_judge = WaitSnapshot {
            judge_rounds_left: 0,
            ..stalled
        };
        assert_eq!(next_wait_action(&policy(), &no_judge), WaitAction::Stalled);
    }

    /// Without a usable activity signal, "quiet" and "stuck" are the same
    /// thing from the outside — so nothing may be killed for being quiet.
    #[test]
    fn silence_alone_never_stops_a_command() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(600),
            since_progress: Duration::from_secs(590),
            since_judge: Some(Duration::from_secs(1)),
            stall_unconfirmed: true,
            activity_measurable: false,
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &snap), WaitAction::KeepWaiting);
    }

    #[test]
    fn periodic_checks_respect_the_grace_period_and_backoff() {
        let too_early = WaitSnapshot {
            elapsed: IDLE_CHECK_GRACE - Duration::from_secs(1),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &too_early), WaitAction::KeepWaiting);

        let first_check = WaitSnapshot {
            elapsed: IDLE_CHECK_GRACE,
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &first_check), WaitAction::Judge);

        let waiting_out_backoff = WaitSnapshot {
            elapsed: Duration::from_secs(60),
            since_judge: Some(IDLE_CHECK_MIN_INTERVAL - Duration::from_millis(1)),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &waiting_out_backoff),
            WaitAction::KeepWaiting
        );

        let backoff_elapsed = WaitSnapshot {
            elapsed: Duration::from_secs(60),
            since_judge: Some(IDLE_CHECK_MIN_INTERVAL),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &backoff_elapsed), WaitAction::Judge);
    }

    /// A command that burns CPU without printing anything (compiling,
    /// linking, packing) must be allowed to keep going even though its stall
    /// window has long since passed in output terms.
    #[test]
    fn silent_but_busy_commands_are_not_killed() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let policy = WaitPolicy {
            ceiling: Duration::from_secs(120),
            stall: Duration::from_secs(2),
        };
        let started = Instant::now();
        let result = run_foreground_with_policy(
            "$sw=[Diagnostics.Stopwatch]::StartNew(); $x=0; \
             while ($sw.Elapsed.TotalSeconds -lt 6) { $x++ }; Write-Output \"spun $x times\"",
            None,
            &cancelled,
            None,
            policy,
        )
        .expect("a busy command must not be reported as stuck");
        assert!(
            result.contains("spun"),
            "command was cut short before finishing: {result}"
        );
        assert!(started.elapsed() >= Duration::from_secs(5));
    }

    /// A process that produces nothing and burns no CPU is stuck, and must be
    /// reclaimed after the stall window instead of holding the turn until the
    /// absolute ceiling.
    #[test]
    fn stuck_commands_are_reclaimed_before_the_ceiling() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let policy = WaitPolicy {
            ceiling: Duration::from_secs(600),
            stall: Duration::from_secs(2),
        };
        let started = Instant::now();
        let error = run_foreground_with_policy(
            "Start-Sleep -Seconds 300",
            None,
            &cancelled,
            None,
            policy,
        )
        .expect_err("an idle process must be reported as stuck");
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "stuck command was not reclaimed promptly: {}s",
            started.elapsed().as_secs()
        );
        let message = error.to_string();
        assert!(
            message.contains("no progress"),
            "unexpected stall message: {message}"
        );
    }

    /// Same held-pipe scenario through the background path: the job must be
    /// published as done instead of hanging until the daemon exits.
    #[test]
    fn background_job_finishes_when_descendant_holds_pipe() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                concat!(
                    "Write-Output 'start'; ",
                    "cmd /c \"start /b powershell -NoProfile -Command Start-Sleep -Seconds 300\"; ",
                    "Start-Sleep -Milliseconds 500; Write-Output 'done'"
                )
                .into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");
        let (context, db_path) = test_context();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("status: done") {
                assert!(status.contains("done"), "missing final output: {status}");
                assert!(status.contains("note:"), "expected held-pipe note: {status}");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background job stuck on held pipe: {status}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }
}
