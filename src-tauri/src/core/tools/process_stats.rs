//! Process-tree activity sampling for running commands.
//!
//! Whether a command is still doing work cannot be answered from its output
//! alone. A quiet build can be busy for minutes without printing anything,
//! while a wrapper process whose real work is already done can keep printing
//! low-signal keep-alive lines forever. Consumed CPU time separates the two.
//!
//! On Windows the spawned process is attached to a dedicated accounting job
//! object: job membership is inherited by every descendant, so the totals
//! cover the whole tree even when a child detaches or reparents itself. The
//! job carries no limits and no kill-on-close flag — it only counts, so
//! dropping the probe never disturbs the processes.
//!
//! Everywhere else (and whenever attaching fails) sampling reports `None`,
//! and callers fall back to output-only progress detection.

use std::process::Child;
use std::time::Duration;

/// Minimum CPU time a process tree must consume before it counts as
/// "still working" while producing no output.
pub const CPU_PROGRESS_EPSILON: Duration = Duration::from_millis(200);

pub struct ActivityProbe {
    #[cfg(windows)]
    job: Option<windows::Win32::Foundation::HANDLE>,
}

impl ActivityProbe {
    /// Start accounting for `child` and everything it spawns. Never fails:
    /// an unusable probe simply reports no samples.
    #[cfg(windows)]
    pub fn attach(child: &Child) -> Self {
        Self {
            job: attach_job(child),
        }
    }

    #[cfg(not(windows))]
    pub fn attach(child: &Child) -> Self {
        let _ = child;
        Self {}
    }

    /// Total CPU time (user + kernel) consumed by the process tree so far,
    /// or `None` when this platform/probe cannot measure it.
    #[cfg(windows)]
    pub fn cpu_time(&self) -> Option<Duration> {
        use windows::Win32::System::JobObjects::{
            JobObjectBasicAccountingInformation, QueryInformationJobObject,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        };

        let job = self.job?;
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        unsafe {
            QueryInformationJobObject(
                job,
                JobObjectBasicAccountingInformation,
                &mut info as *mut _ as *mut _,
                std::mem::size_of_val(&info) as u32,
                None,
            )
            .ok()?;
        }
        // Both totals are in 100-nanosecond units.
        let ticks = info.TotalUserTime as u64 + info.TotalKernelTime as u64;
        Some(Duration::from_nanos(ticks.saturating_mul(100)))
    }

    #[cfg(not(windows))]
    pub fn cpu_time(&self) -> Option<Duration> {
        None
    }

    /// Whether this probe can produce samples at all. Callers use it to stay
    /// conservative: without a reliable activity signal, a silent command is
    /// never assumed to be stuck.
    pub fn is_measurable(&self) -> bool {
        self.cpu_time().is_some()
    }
}

#[cfg(windows)]
fn attach_job(child: &Child) -> Option<windows::Win32::Foundation::HANDLE> {
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};

    unsafe {
        let job = CreateJobObjectW(None, PCWSTR::null()).ok()?;
        if job.is_invalid() {
            return None;
        }
        let process = HANDLE(child.as_raw_handle() as *mut _);
        if AssignProcessToJobObject(job, process).is_err() {
            close_handle(job);
            return None;
        }
        Some(job)
    }
}

#[cfg(windows)]
fn close_handle(handle: windows::Win32::Foundation::HANDLE) {
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(handle);
    }
}

#[cfg(windows)]
impl Drop for ActivityProbe {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            close_handle(job);
        }
    }
}
