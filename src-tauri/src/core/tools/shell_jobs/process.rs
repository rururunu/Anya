use crate::runtime::terminal::prepare_command;
use std::process::{Child, Command, Stdio};

/// Kills a process and its entire child tree.
pub fn terminate_process_tree(child: &mut Child) {
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
