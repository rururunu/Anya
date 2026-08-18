//! Restricted process spawning entry point.
//!
//! The long-term goal (per `Agent 开发能力补齐计划`) is to spawn commands with:
//! - a restricted Windows token (CreateRestrictedToken)
//! - Low integrity level (TokenIntegrityLevel)
//! - CreateProcessAsUserW for the actual process launch
//!
//! This file currently provides the unified spawn interface and keeps the
//! existing Job Object + env scrubbing behavior, so we can safely refactor
//! call sites to funnel through a single boundary.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;

use crate::core::tools::error::ToolError;
use crate::core::tools::sandbox::{assign_restricted_job, scrub_sensitive_env};
use crate::runtime::terminal::prepare_powershell;

/// Spawn `powershell -NoProfile -NonInteractive -Command ...` with stdout/stderr
/// piped and stdin detached.
///
/// When `restricted_shell()` is enabled, the process is also assigned to the
/// Job Object and sensitive env vars are removed.
///
/// Note: This is a placeholder for the future restricted token + CreateProcessAsUserW
/// implementation; call sites already depend on this interface.
pub fn spawn_powershell(
    command: &str,
    cwd: Option<&Path>,
    _cancelled: &AtomicBool,
) -> Result<Child, ToolError> {
    #[cfg(windows)]
    if crate::core::tools::sandbox::restricted_shell() {
        if let Err(error) = try_prepare_restricted_token() {
            tracing::warn!(%error, "restricted token preflight failed; falling back");
        }
    }

    let guarded = wrap_with_workspace_guards(command, cwd);
    let mut cmd = Command::new("powershell");
    prepare_powershell(&mut cmd, &guarded);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    // Keep the existing safety behavior until we wire the restricted token
    // creation + CreateProcessAsUserW.
    if crate::core::tools::sandbox::restricted_shell() {
        scrub_sensitive_env(&mut cmd);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| ToolError::new(format!("failed to start powershell: {e}")))?;

    if crate::core::tools::sandbox::restricted_shell() {
        assign_restricted_job(&mut child);
    }

    Ok(child)
}

#[cfg(windows)]
fn try_prepare_restricted_token() -> Result<(), String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        CreateRestrictedToken, TokenPrimary, DISABLE_MAX_PRIVILEGE, TOKEN_ADJUST_DEFAULT,
        TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut current = windows::Win32::Foundation::HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            &mut current,
        )
        .map_err(|e| format!("OpenProcessToken: {e}"))?;

        let mut restricted = windows::Win32::Foundation::HANDLE::default();
        let result = CreateRestrictedToken(
            current,
            DISABLE_MAX_PRIVILEGE,
            None,
            None,
            None,
            &mut restricted,
        )
        .map_err(|e| format!("CreateRestrictedToken: {e}"));

        // Validate we can duplicate as primary (the required token class for
        // CreateProcessAsUserW path, added in a later step).
        if result.is_ok() {
            let mut primary = windows::Win32::Foundation::HANDLE::default();
            let _ = windows::Win32::Security::DuplicateTokenEx(
                restricted,
                TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE,
                None,
                windows::Win32::Security::SecurityImpersonation,
                TokenPrimary,
                &mut primary,
            )
            .map_err(|e| format!("DuplicateTokenEx: {e}"))?;
            if !primary.is_invalid() {
                let _ = CloseHandle(primary);
            }
        }

        if !restricted.is_invalid() {
            let _ = CloseHandle(restricted);
        }
        if !current.is_invalid() {
            let _ = CloseHandle(current);
        }
        result
    }
}

fn wrap_with_workspace_guards(command: &str, cwd: Option<&Path>) -> String {
    let Some(root) = cwd else {
        return command.to_string();
    };
    let root = root
        .to_string_lossy()
        .replace('\'', "''")
        .replace('\\', "\\\\");
    // Guard common file-mutating cmdlets in-process so even script-internal
    // operations are checked against workspace boundaries.
    format!(
        r#"
$__anyaRoot = [System.IO.Path]::GetFullPath('{root}')
function __anyaAssertPath([string]$path) {{
  if ([string]::IsNullOrWhiteSpace($path)) {{ return }}
  $full = [System.IO.Path]::GetFullPath((Join-Path $__anyaRoot $path))
  if (-not $full.StartsWith($__anyaRoot, [System.StringComparison]::OrdinalIgnoreCase)) {{
    throw "workspace boundary denied: $full"
  }}
}}
function Set-Content {{ param([string]$Path, [Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; Microsoft.PowerShell.Management\Set-Content -Path $Path @Rest }}
function Add-Content {{ param([string]$Path, [Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; Microsoft.PowerShell.Management\Add-Content -Path $Path @Rest }}
function Out-File {{ param([string]$FilePath, [Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $FilePath; Microsoft.PowerShell.Utility\Out-File -FilePath $FilePath @Rest }}
function New-Item {{ param([string]$Path, [Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; Microsoft.PowerShell.Management\New-Item -Path $Path @Rest }}
function Copy-Item {{ param([string]$Path,[string]$Destination,[Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; __anyaAssertPath $Destination; Microsoft.PowerShell.Management\Copy-Item -Path $Path -Destination $Destination @Rest }}
function Move-Item {{ param([string]$Path,[string]$Destination,[Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; __anyaAssertPath $Destination; Microsoft.PowerShell.Management\Move-Item -Path $Path -Destination $Destination @Rest }}
function Remove-Item {{ param([string]$Path,[Parameter(ValueFromRemainingArguments=$true)]$Rest) __anyaAssertPath $Path; Microsoft.PowerShell.Management\Remove-Item -Path $Path @Rest }}
{command}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_command_with_workspace_assertions() {
        let root = std::path::Path::new(r"C:\repo");
        let wrapped = wrap_with_workspace_guards("Write-Output 'ok'", Some(root));
        assert!(wrapped.contains("__anyaAssertPath"));
        assert!(wrapped.contains("Set-Content"));
        assert!(wrapped.contains("Write-Output 'ok'"));
    }
}
