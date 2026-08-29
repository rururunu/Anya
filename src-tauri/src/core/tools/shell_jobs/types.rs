use super::constants::*;
use super::output::{
    append_bounded, format_job_status, format_job_status_with_output,
    spawn_output_reader, take_tail_chars,
};
use super::process::terminate_process_tree;
use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::shell_judge::{judge_shell_completion, CompletionVerdict};
use crate::runtime::terminal::prepare_powershell;
#[cfg(windows)]
use crate::core::tools::sandbox::restricted_process;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct ShellJob {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub command: String,
    pub output: String,
    pub(super) raw_output: Vec<u8>,
    pub done: bool,
    pub exit_code: Option<i32>,
    pub(super) cwd: Option<std::path::PathBuf>,
    pub(super) child: Option<Child>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) started: Instant,
    pub(super) last_output_at: Option<Instant>,
}

pub struct ShellJobStore {
    pub(in crate::core::tools::shell_jobs) jobs: Mutex<HashMap<String, ShellJob>>,
    next_id: Mutex<u32>,
}

impl ShellJobStore {
    /// Creates a new empty shell job store.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        })
    }

    /// Spawns a command in the background and returns its job id.
    pub fn spawn_background(
        self: &Arc<Self>,
        command: String,
        cwd: Option<&std::path::Path>,
        cancelled: Arc<AtomicBool>,
        judge: Option<ToolContext>,
    ) -> Result<String, ToolError> {
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
                            guard
                                .get(job_id)
                                .map(|job| job.output.clone())
                                .unwrap_or_default()
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

    /// Reads job output with optional tail line and character limits.
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

    /// Blocks until a background job finishes or the wait timeout expires.
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

    /// Terminates a running background job and marks it done.
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
