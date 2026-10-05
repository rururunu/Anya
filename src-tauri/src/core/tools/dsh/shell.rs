//! dsh pwsh/bash lifecycle: foreground deadlines promote, explicit jobs survive.
use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::shell_jobs::terminate_process_tree;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Output {
    text: String,
    done: bool,
    exit: Option<i32>,
    cursor: usize,
    notified: bool,
    stopped: bool,
    clipped: bool,
}
struct Job {
    id: String,
    session: String,
    command: String,
    output: Arc<Mutex<Output>>,
    stop: Arc<AtomicBool>,
}
fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Arc<Job>>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[cfg(test)]
mod output_tests {
    use super::*;
    #[test]
    fn bounded_output_reports_clipping_and_preserves_utf8() {
        let output = Arc::new(Mutex::new(Output::default()));
        let bytes = "测".repeat(400_000).into_bytes();
        drain(std::io::Cursor::new(bytes), Arc::clone(&output))
            .join()
            .unwrap();
        let output = output.lock().unwrap();
        assert!(output.clipped);
        assert!(output.text.len() <= 1024 * 1024);
        assert!(!output.text.contains('\u{fffd}'));
    }
}
pub(super) fn clear_session(session_id: &str) {
    let child_prefix = format!("{session_id}-sub-");
    if let Ok(mut store) = jobs().lock() {
        store.retain(|_, record| {
            let keep = record.session != session_id && !record.session.starts_with(&child_prefix);
            if !keep {
                record.stop.store(true, Ordering::Relaxed);
            }
            keep
        });
    }
}
fn job(ctx: &ToolContext, id: &str) -> Result<Arc<Job>, ToolError> {
    jobs()
        .lock()
        .map_err(|_| ToolError::new("Job lock failed"))?
        .get(id)
        .filter(|j| j.session == ctx.session_id)
        .cloned()
        .ok_or_else(|| ToolError::new(format!("Unknown job: {id}")))
}
fn drain(
    mut stream: impl Read + Send + 'static,
    output: Arc<Mutex<Output>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut pending = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let count = match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&buffer[..count]);
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                Err(_) => pending.len(),
            };
            let text = String::from_utf8_lossy(&pending[..valid]);
            if let Ok(mut out) = output.lock() {
                if out.text.len() < 1024 * 1024 {
                    let available = (1024 * 1024 - out.text.len()).min(text.len());
                    let mut end = available;
                    while !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    out.text.push_str(&text[..end]);
                    out.clipped |= end < text.len();
                } else if !text.is_empty() {
                    out.clipped = true;
                }
            }
            pending.drain(..valid);
        }
        if !pending.is_empty() {
            if let Ok(mut out) = output.lock() {
                let text = String::from_utf8_lossy(&pending);
                let mut end = (1024 * 1024usize)
                    .saturating_sub(out.text.len())
                    .min(text.len());
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                out.text.push_str(&text[..end]);
                out.clipped |= end < text.len();
            }
        }
    })
}
fn start(ctx: &ToolContext, args: &Value, name: &str) -> Result<Arc<Job>, ToolError> {
    let command = super::string(args, "command")?;
    super::string(args, "description")?;
    crate::core::rules::RuleEngine::authorize_tool("run_shell", args)?;
    // Preserve the application's command safety rules and approval boundary.
    crate::core::tools::sandbox::reject_source_file_shell_writes(command)?;
    crate::core::tools::sandbox::reject_user_plugin_hunt(command)?;
    let pass_all = crate::core::tools::tool_approval::shared_tool_approval_store()
        .mode_for_session(ctx.root_session_id())
        == crate::models::settings::ToolApprovalMode::AlwaysAllow;
    if !pass_all {
        crate::core::tools::sandbox::reject_workspace_escape_writes(
            command,
            Some(&ctx.workspace_root),
        )?;
    }
    let directory = match args["workdir"].as_str() {
        Some(raw) => super::files::path(ctx, raw, false, name)?,
        None => ctx.workspace_root.clone(),
    };
    if !directory.is_dir() {
        return Err(ToolError::new("workdir must be a directory"));
    }
    let mut process = Command::new(if cfg!(windows) { "powershell" } else { "bash" });
    if cfg!(windows) {
        crate::runtime::terminal::prepare_powershell(&mut process, command);
    } else {
        process.args(["-lc", command]);
    }
    process
        .current_dir(&directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x08000000);
    }
    let stop = Arc::new(AtomicBool::new(false));
    #[cfg(windows)]
    let mut child = if crate::core::tools::sandbox::restricted_shell() {
        crate::core::tools::sandbox::restricted_process::spawn_powershell(
            command,
            Some(&directory),
            &stop,
        )?
    } else {
        process.spawn()?
    };
    #[cfg(not(windows))]
    let mut child = process.spawn()?;
    let state = Arc::new(Mutex::new(Output::default()));
    let stdout = child
        .stdout
        .take()
        .map(|stream| drain(stream, Arc::clone(&state)));
    let stderr = child
        .stderr
        .take()
        .map(|stream| drain(stream, Arc::clone(&state)));
    let record = Arc::new(Job {
        id: uuid::Uuid::new_v4().to_string(),
        session: ctx.session_id.clone(),
        command: command.to_string(),
        output: state,
        stop,
    });
    jobs()
        .lock()
        .map_err(|_| ToolError::new("Job lock failed"))?
        .insert(record.id.clone(), Arc::clone(&record));
    let task = Arc::clone(&record);
    std::thread::spawn(move || {
        let exit = loop {
            if task.stop.load(Ordering::Relaxed) {
                terminate_process_tree(&mut child);
                break child.wait().ok().and_then(|s| s.code()).or(Some(1));
            }
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) => std::thread::sleep(Duration::from_millis(30)),
                Err(_) => {
                    terminate_process_tree(&mut child);
                    let _ = child.wait();
                    break Some(1);
                }
            }
        };
        if let Some(reader) = stdout {
            let _ = reader.join();
        }
        if let Some(reader) = stderr {
            let _ = reader.join();
        }
        if let Ok(mut output) = task.output.lock() {
            if output.clipped {
                output.text.push_str(
                    "\n[Output truncated at 1 MiB; rerun with a narrower output filter.]\n",
                );
            }
            output.done = true;
            output.exit = exit;
            output.stopped = task.stop.load(Ordering::Relaxed);
        }
    });
    Ok(record)
}

fn wait(
    ctx: &ToolContext,
    record: &Job,
    timeout: usize,
    cancel_owned: bool,
) -> Result<bool, ToolError> {
    let deadline = Instant::now() + Duration::from_millis(timeout as u64);
    loop {
        if ctx.is_cancelled() {
            if cancel_owned {
                record.stop.store(true, Ordering::Relaxed);
            }
            return Err(ToolError::cancelled());
        }
        if record
            .output
            .lock()
            .map_err(|_| ToolError::new("Job lock failed"))?
            .done
        {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
fn status(record: &Job, out: &Output) -> String {
    let status = if out.done {
        if out.stopped {
            "cancelled"
        } else if out.exit == Some(0) {
            "completed"
        } else {
            "failed"
        }
    } else {
        "running"
    };
    format!(
        "[job_id: {}]\n[status: {status}]{}",
        record.id,
        out.exit
            .map(|code| format!("\n[exit code: {code}]"))
            .unwrap_or_default()
    )
}

pub(super) fn execute(ctx: &ToolContext, name: &str, args: &Value) -> Result<String, ToolError> {
    ctx.ensure_not_cancelled()?;
    if matches!(name, "pwsh" | "bash") {
        let timeout = super::positive(args, "timeoutMs", 10000, 120000)?;
        let record = start(ctx, args, name)?;
        if args["run_in_background"].as_bool().unwrap_or(false) {
            return Ok(format!("[job_id: {}]\n[status: running]", record.id));
        }
        if !wait(ctx, &record, timeout, true)? {
            let mut out = record
                .output
                .lock()
                .map_err(|_| ToolError::new("Job lock failed"))?;
            out.cursor = out.text.len();
            return Ok(format!(
                "{}\n[Command moved to the background after {timeout} ms.]\n{}",
                out.text,
                status(&record, &out)
            ));
        }
        let out = record
            .output
            .lock()
            .map_err(|_| ToolError::new("Job lock failed"))?;
        let result = format!("{}\n[exit code: {}]", out.text, out.exit.unwrap_or(1));
        drop(out);
        jobs()
            .lock()
            .map_err(|_| ToolError::new("Job lock failed"))?
            .remove(&record.id);
        return Ok(result);
    }
    if name == "job_list" {
        let store = jobs()
            .lock()
            .map_err(|_| ToolError::new("Job lock failed"))?;
        let records = store.values().filter(|j| j.session == ctx.session_id).map(|j| {
            let out = j.output.lock().map_err(|_| ToolError::new("Job lock failed"))?;
            Ok(json!({"id": j.id, "description": j.command, "status": if out.done { if out.stopped { "cancelled" } else if out.exit == Some(0) { "completed" } else { "failed" } } else { "running" }}))
        }).collect::<Result<Vec<_>, ToolError>>()?;
        return Ok(json!({"jobs": records}).to_string());
    }
    let record = job(ctx, super::string(args, "job_id")?)?;
    if name == "job_kill" {
        record.stop.store(true, Ordering::Relaxed);
        let _ = wait(ctx, &record, 10000, false)?;
    } else if args["wait"].as_bool().unwrap_or(false) {
        let timeout = super::positive(args, "timeout_ms", 30000, 300000)?;
        let _ = wait(ctx, &record, timeout, false)?;
    }
    let mut out = record
        .output
        .lock()
        .map_err(|_| ToolError::new("Job lock failed"))?;
    let text = out.text[out.cursor.min(out.text.len())..].to_string();
    out.cursor = out.text.len();
    if out.done {
        out.notified = true;
    }
    Ok(format!(
        "{}\n{}",
        if text.is_empty() {
            "(no new output)"
        } else {
            &text
        },
        status(&record, &out)
    ))
}

pub fn completion_notices(session: &str) -> Vec<String> {
    let Ok(store) = jobs().lock() else {
        return vec![];
    };
    store
        .values()
        .filter(|j| j.session == session)
        .filter_map(|j| {
            let mut out = j.output.lock().ok()?;
            if !out.done || out.notified {
                return None;
            }
            out.notified = true;
            Some(format!(
                "Background job finished: {}.\n{}\nCollect its result with job_output.",
                j.command,
                status(j, &out)
            ))
        })
        .collect()
}

/// Bounded, cancellable subprocess used by the shell-free rg tools.
pub(super) fn capture(
    ctx: &ToolContext,
    command: &mut Command,
) -> Result<(i32, String, String), ToolError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn()?;
    let output = Arc::new(Mutex::new(Output::default()));
    let errors = Arc::new(Mutex::new(Output::default()));
    let stdout = drain(child.stdout.take().unwrap(), Arc::clone(&output));
    let stderr = drain(child.stderr.take().unwrap(), Arc::clone(&errors));
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut failure = None;
    let code = loop {
        if ctx.is_cancelled() || Instant::now() >= deadline {
            failure = Some(if ctx.is_cancelled() {
                ToolError::cancelled()
            } else {
                ToolError::new("Search timed out")
            });
            terminate_process_tree(&mut child);
            let _ = child.wait();
            break 1;
        }
        match child.try_wait() {
            Ok(Some(exit)) => break exit.code().unwrap_or(1),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                failure = Some(error.into());
                terminate_process_tree(&mut child);
                let _ = child.wait();
                break 1;
            }
        }
    };
    let _ = stdout.join();
    let _ = stderr.join();
    if let Some(error) = failure {
        return Err(error);
    }
    let text = output
        .lock()
        .map_err(|_| ToolError::new("Output lock failed"))?
        .text
        .clone();
    if text.len() >= 1024 * 1024 {
        return Err(ToolError::new(
            "Search output exceeds 1 MiB; narrow pattern or path.",
        ));
    }
    let err = errors
        .lock()
        .map_err(|_| ToolError::new("Output lock failed"))?
        .text
        .clone();
    Ok((code, text, err))
}
