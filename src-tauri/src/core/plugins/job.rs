//! Per-plugin Windows Job Object and process-tree kill.

use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

pub struct PluginJob {
    #[cfg(windows)]
    handle: isize,
}

impl PluginJob {
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            Self {
                handle: create_kill_on_close_job(),
            }
        }
        #[cfg(not(windows))]
        {
            Self {}
        }
    }

    #[allow(dead_code)]
    pub fn assign(&self, child: &mut std::process::Child) {
        #[cfg(windows)]
        {
            assign_job(self.handle, child);
        }
        #[cfg(not(windows))]
        {
            let _ = child;
            let _ = self;
        }
    }
}

impl Drop for PluginJob {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            close_job(self.handle);
        }
    }
}

pub fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

#[cfg(windows)]
fn create_kill_on_close_job() -> isize {
    use windows::core::PCWSTR;
    use windows::Win32::System::JobObjects::{
        CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    unsafe {
        let job = CreateJobObjectW(None, PCWSTR::null()).unwrap_or_default();
        if job.is_invalid() {
            return 0;
        }
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        job.0 as isize
    }
}

#[cfg(windows)]
#[allow(dead_code)]
fn assign_job(job: isize, child: &mut std::process::Child) {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::AssignProcessToJobObject;
    if job == 0 {
        return;
    }
    unsafe {
        let process = HANDLE(child.as_raw_handle() as *mut _);
        let _ = AssignProcessToJobObject(HANDLE(job as *mut _), process);
    }
}

#[cfg(windows)]
fn close_job(job: isize) {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    if job == 0 {
        return;
    }
    unsafe {
        let _ = CloseHandle(HANDLE(job as *mut _));
    }
}
