//! Execution sandbox policy: workspace write isolation, hardened shell denylist,
//! and optional restricted-shell limits (timeout + Windows Job Object).

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use super::error::ToolError;
use super::path::normalize_path;

#[cfg(windows)]
#[path = "sandbox/restricted_process.rs"]
pub(crate) mod restricted_process;

static ALLOW_OUTSIDE_WRITES: AtomicBool = AtomicBool::new(false);
static RESTRICTED_SHELL: AtomicBool = AtomicBool::new(false);
static SHELL_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(3600);
static SHELL_STALL_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(120);

/// Process-wide sandbox knobs — updated from settings without AppHandle.
pub fn configure(
    allow_outside_workspace_writes: bool,
    restricted_shell: bool,
    shell_timeout_secs: u64,
    shell_stall_timeout_secs: u64,
) {
    ALLOW_OUTSIDE_WRITES.store(allow_outside_workspace_writes, Ordering::Relaxed);
    RESTRICTED_SHELL.store(restricted_shell, Ordering::Relaxed);
    SHELL_TIMEOUT_SECS.store(shell_timeout_secs.max(5), Ordering::Relaxed);
    SHELL_STALL_TIMEOUT_SECS.store(shell_stall_timeout_secs.max(5), Ordering::Relaxed);
}

pub fn allow_outside_workspace_writes() -> bool {
    ALLOW_OUTSIDE_WRITES.load(Ordering::Relaxed)
}

pub fn restricted_shell() -> bool {
    RESTRICTED_SHELL.load(Ordering::Relaxed)
}

/// Absolute ceiling for a single foreground command.
pub fn shell_timeout_secs() -> u64 {
    SHELL_TIMEOUT_SECS.load(Ordering::Relaxed)
}

/// How long a foreground command may make no progress before it is treated
/// as stuck (no new output and no CPU consumed by its process tree).
pub fn shell_stall_timeout_secs() -> u64 {
    SHELL_STALL_TIMEOUT_SECS.load(Ordering::Relaxed)
}

/// Reject clearly destructive / privilege-escalating shell commands.
pub fn reject_dangerous_shell(command: &str) -> Result<(), ToolError> {
    let decoded = extract_powershell_encodedcommand(command)
        .and_then(|payload| base64_decode_utf16le_or_utf8(&payload).ok());
    let normalized = normalize_shell_command(command);
    let scan_input = decoded
        .map(|d| format!("{normalized}\n# decoded -EncodedCommand\n{d}"))
        .unwrap_or(normalized.clone());
    let denied = [
        "git reset --hard",
        "git clean -fd",
        "git clean -f -d",
        "remove-item -recurse -force",
        "remove-item -force -recurse",
        "rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        "rm -rf $home",
        "del /s /q",
        "rd /s /q",
        "rmdir /s /q",
        "format-volume",
        "format c:",
        "format d:",
        "clear-disk",
        "diskpart",
        "shutdown /s",
        "shutdown /r",
        "stop-computer",
        "restart-computer",
        "reg delete",
        "reg.exe delete",
        "curl|iex",
        "curl | iex",
        "wget|iex",
        "wget | iex",
        "iwr | iex",
        "invoke-expression (invoke-webrequest",
        "iex (iwr",
        "start-bitstransfer",
        "set-executionpolicy bypass",
        "disable-defender",
        "net user administrator",
        "takeown /f",
        "icacls .* /grant everyone",
    ];
    if let Some(rule) = denied.iter().find(|rule| scan_input.contains(*rule)) {
        return Err(ToolError::new(format!(
            "rule denied dangerous shell command: {rule}"
        )));
    }
    // PowerShell Remove-Item with -Recurse anywhere in the token stream.
    if scan_input.contains("remove-item")
        && (scan_input.contains("-recurse") || scan_input.contains(" -r "))
        && (scan_input.contains("-force") || scan_input.contains(" -f "))
    {
        return Err(ToolError::new(
            "rule denied dangerous shell command: Remove-Item -Recurse -Force",
        ));
    }
    Ok(())
}

/// Lowercases the command, strips PowerShell backticks, and collapses whitespace for denylist matching.
fn normalize_shell_command(command: &str) -> String {
    let lower = command.to_lowercase().replace('`', "");
    let mut out = String::with_capacity(lower.len());
    let mut prev_space = false;
    for ch in lower.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

fn extract_powershell_encodedcommand(command: &str) -> Option<String> {
    let lower = command.to_ascii_lowercase();
    let marker = "-encodedcommand";
    let idx = lower.find(marker)?;
    let after = command[idx + marker.len()..].trim_start();
    if after.is_empty() {
        return None;
    }
    // Drop a leading ':' if input is like `-EncodedCommand:<payload>`.
    let after = after.strip_prefix(':').unwrap_or(after).trim_start();
    let token = if after.starts_with('"') && after.len() >= 2 {
        let rest = &after[1..];
        let end = rest.find('"')?;
        rest[..end].trim().to_string()
    } else if after.starts_with('\'') && after.len() >= 2 {
        let rest = &after[1..];
        let end = rest.find('\'')?;
        rest[..end].trim().to_string()
    } else {
        // Read until whitespace or common PowerShell separators.
        let end = after
            .find(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&'))
            .unwrap_or(after.len());
        after[..end].trim().to_string()
    };
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn base64_decode_utf16le_or_utf8(payload: &str) -> Result<String, ToolError> {
    // PowerShell's `-EncodedCommand` uses UTF-16LE for the base64 payload in
    // typical scenarios, but we accept UTF-8 as a best-effort fallback.
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let bytes = STANDARD
        .decode(payload)
        .map_err(|e| ToolError::new(format!("failed to decode -EncodedCommand base64: {e}")))?;

    if bytes.len() % 2 == 0 {
        let mut u16s = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        if let Ok(s) = String::from_utf16(&u16s) {
            return Ok(s);
        }
    }

    let s = String::from_utf8(bytes)
        .map_err(|e| ToolError::new(format!("decoded payload is not UTF-8: {e}")))?;
    Ok(s)
}

/// Heuristic: block redirects / copy targets that clearly write outside the workspace.
pub fn reject_workspace_escape_writes(
    command: &str,
    workspace: Option<&Path>,
) -> Result<(), ToolError> {
    let Some(workspace) = workspace else {
        return Ok(());
    };
    let workspace = normalize_path(workspace);
    let patterns = escape_write_path_candidates(command);
    for raw in patterns {
        let candidate = Path::new(&raw);
        let resolved = if candidate.is_absolute() {
            normalize_path(candidate)
        } else {
            normalize_path(&workspace.join(candidate))
        };
        if !resolved.starts_with(&workspace) {
            return Err(ToolError::new(format!(
                "shell write target escapes workspace: {}",
                resolved.display()
            )));
        }
    }
    Ok(())
}

fn escape_write_path_candidates(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Out-file / Set-Content / > / >> style targets.
    let markers = [
        ">",
        ">>",
        "| out-file",
        "| set-content",
        "out-file ",
        "set-content ",
        "copy-item ",
        "move-item ",
        "ni ",
        "new-item ",
    ];
    let lower = command.to_ascii_lowercase();
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            let after = &command[idx + marker.len()..];
            if let Some(path) = first_path_token(after) {
                out.push(path);
            }
        }
    }
    out
}

fn first_path_token(s: &str) -> Option<String> {
    let trimmed = s.trim_start_matches([' ', '=', ':']);
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('"') {
        let rest = &trimmed[1..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == '|' || c == ';')
        .unwrap_or(trimmed.len());
    let token = trimmed[..end].trim();
    if token.is_empty() || token.starts_with('-') {
        None
    } else {
        Some(token.to_string())
    }
}

/// Scrub sensitive env vars before spawning a restricted shell.
pub fn scrub_sensitive_env(cmd: &mut std::process::Command) {
    const KEYS: &[&str] = &[
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "MEM0_API_KEY",
        "SERPER_API_KEY",
        "TAVILY_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_ACCESS_KEY_ID",
    ];
    for key in KEYS {
        cmd.env_remove(key);
    }
}

/// Assign the child process to a Windows Job Object with memory/CPU limits.
/// No-op on non-Windows. Best-effort: failures are logged but do not abort.
pub fn assign_restricted_job(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        if let Err(error) = assign_job_windows(child) {
            tracing::warn!(%error, "failed to assign restricted job object");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = child;
    }
}

#[cfg(windows)]
fn assign_job_windows(child: &mut std::process::Child) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicUIRestrictions,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_BASIC_UI_RESTRICTIONS,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        JOB_OBJECT_LIMIT_PROCESS_TIME, JOB_OBJECT_LIMIT_WORKINGSET, JOB_OBJECT_UILIMIT_HANDLES,
        JOB_OBJECT_UILIMIT_READCLIPBOARD, JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    };

    static JOB: OnceLock<isize> = OnceLock::new();
    let job_handle = *JOB.get_or_init(|| unsafe {
        let job = match CreateJobObjectW(None, PCWSTR::null()) {
            Ok(handle) => handle,
            Err(_) => return 0,
        };
        if job.is_invalid() {
            return 0;
        }
        // Keep the kernel-side CPU cap aligned with the configured ceiling,
        // otherwise a legitimately long build would be killed here while the
        // in-process guard rails still consider it healthy.
        let cpu_limit_ticks = (shell_timeout_secs() as i64).saturating_mul(10_000_000);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                PerProcessUserTimeLimit: cpu_limit_ticks,
                MinimumWorkingSetSize: 16 * 1024 * 1024,
                MaximumWorkingSetSize: 512 * 1024 * 1024,
                ActiveProcessLimit: 32,
                LimitFlags: JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_WORKINGSET
                    | JOB_OBJECT_LIMIT_PROCESS_TIME,
                ..Default::default()
            },
            // Hard kill on job handle close to avoid lingering child processes.
            // (Set here even if the OS ignores some fields; best-effort.)
            ProcessMemoryLimit: 512 * 1024 * 1024,
            ..Default::default()
        };
        // Extend LimitFlags with KILL_ON_JOB_CLOSE + ACTIVE_PROCESS.
        // NOTE: `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` uses `LimitFlags` in
        // `BasicLimitInformation`, and `ActiveProcessLimit` in the outer struct.
        info.BasicLimitInformation.LimitFlags |=
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        let _ = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            std::mem::size_of_val(&info) as u32,
        );

        // Reduce cross-process / UI surface. This is best-effort: some
        // systems/versions may ignore parts of the restrictions.
        let mut ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
            UIRestrictionsClass: JOB_OBJECT_UILIMIT_HANDLES
                | JOB_OBJECT_UILIMIT_READCLIPBOARD
                | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
        };
        let _ = SetInformationJobObject(
            job,
            JobObjectBasicUIRestrictions,
            &mut ui as *mut _ as *mut _,
            std::mem::size_of_val(&ui) as u32,
        );
        job.0 as isize
    });
    if job_handle == 0 {
        return Err("CreateJobObjectW failed".into());
    }
    let process = HANDLE(child.as_raw_handle() as *mut _);
    unsafe {
        AssignProcessToJobObject(HANDLE(job_handle as *mut _), process)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::path::PathBuf;

    #[test]
    fn denies_destructive_shell_commands() {
        assert!(reject_dangerous_shell("git reset --hard HEAD~1").is_err());
        assert!(reject_dangerous_shell("Remove-Item -Recurse -Force C:\\temp").is_err());
        assert!(reject_dangerous_shell("curl|iex").is_err());
        assert!(reject_dangerous_shell("cargo test").is_ok());
    }

    #[test]
    fn blocks_encodedcommand_payload() {
        // PowerShell -EncodedCommand typically uses UTF-16LE base64.
        // Payload: "git reset --hard HEAD~1"
        let payload = "git reset --hard HEAD~1";
        let mut utf16le = Vec::new();
        for u in payload.encode_utf16() {
            utf16le.extend_from_slice(&u.to_le_bytes());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(utf16le);

        let cmd = format!("-EncodedCommand {}", encoded);
        assert!(reject_dangerous_shell(&cmd).is_err());
    }

    #[test]
    fn rejects_write_redirects_outside_workspace() {
        let ws = PathBuf::from(r"C:\projects\app");
        assert!(
            reject_workspace_escape_writes(r#"echo hi > C:\Windows\Temp\out.txt"#, Some(&ws))
                .is_err()
        );
        assert!(reject_workspace_escape_writes(r#"echo hi > .\out.txt"#, Some(&ws)).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn job_object_assignment_is_best_effort() {
        // Smoke test: ensure assigning a process to the restricted Job Object
        // doesn't crash and returns Ok (or logs a warning internally).
        //
        // We keep stdout/stderr detached; this is purely for safety wiring.
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 1",
        ]);
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = cmd.spawn().expect("spawn");
        let _ = assign_restricted_job(&mut child);
        let _ = child.kill();
        let _ = child.wait();
    }
}
