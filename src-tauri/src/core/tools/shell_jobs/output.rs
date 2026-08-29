use super::constants::*;
use super::types::{ShellJob, ShellJobStore};
use crate::core::tools::context::ToolContext;
use crate::core::tools::shell_judge::{judge_shell_completion, CompletionVerdict};
use crate::runtime::encoding::decode_process_bytes;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) fn format_job_status(job: &ShellJob) -> String {
    format_job_status_with_output(job, &job.output)
}

pub(super) fn format_job_status_with_output(job: &ShellJob, output: &str) -> String {
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

pub(super) fn spawn_output_reader<R: Read + Send + 'static>(
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

pub(super) fn append_raw_bounded(job: &mut ShellJob, chunk: &[u8]) {
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

pub(super) fn append_bounded(output: &mut String, chunk: &str) {
    output.push_str(chunk);
    let count = output.chars().count();
    if count > BACKGROUND_OUTPUT_MAX_CHARS {
        *output = take_tail_chars(output, BACKGROUND_OUTPUT_MAX_CHARS);
    }
}

pub(super) fn take_tail_chars(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    value.chars().skip(count.saturating_sub(limit)).collect()
}

pub(super) fn format_duration(value: Duration) -> String {
    if value < Duration::from_secs(1) {
        format!("{}ms", value.as_millis())
    } else {
        format!("{:.1}s", value.as_secs_f64())
    }
}

/// Returns whether a shell command is expected to stay alive in the background.
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

pub(super) fn format_streams(stdout: &str, stderr: &str) -> String {
    format!("stdout:\n{stdout}\nstderr:\n{stderr}")
}

pub(super) enum DrainMsg {
    Chunk(Vec<u8>),
    Eof,
}

/// Move a stream into a thread that forwards chunks (and EOF) over a channel.
/// If no stream was provided, EOF is reported immediately. The thread stays
/// alive while a descendant holds the pipe; after the receiver is dropped it
/// exits on the next successful read (send fails).
pub(super) fn spawn_drain_thread<R: Read + Send + 'static>(
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
pub(super) fn drain_until_quiet(
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
pub(super) fn drain_available(
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
pub(super) fn settle_ambiguous(
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
