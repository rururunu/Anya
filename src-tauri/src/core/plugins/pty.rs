//! ConPTY sessions owned by the plugin host.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::tools::error::ToolError;
use crate::core::tools::memory::plugins_dir;

pub type HostEventFn = Arc<dyn Fn(Value) + Send + Sync>;

struct LivePty {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send>>,
}

pub struct PtyHub {
    sessions: Mutex<HashMap<String, LivePty>>,
    events: HostEventFn,
}

impl PtyHub {
    pub fn new(events: HostEventFn) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            events,
        }
    }

    pub fn open(
        &self,
        plugin_id: &str,
        cols: u16,
        rows: u16,
        cwd: Option<&str>,
        shell: Option<&str>,
    ) -> Result<String, ToolError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(2),
                cols: cols.max(8),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ToolError::new(format!("open pty: {e}")))?;
        let shell = shell
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_shell());
        let mut cmd = CommandBuilder::new(shell);
        if let Some(cwd) = cwd.map(str::trim).filter(|s| !s.is_empty()) {
            cmd.cwd(cwd);
        }
        apply_plugin_bin_path(&mut cmd);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| ToolError::new(format!("spawn shell: {e}")))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| ToolError::new(format!("pty reader: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| ToolError::new(format!("pty writer: {e}")))?;
        let id = format!("pty-{}", Uuid::new_v4().simple());
        let live = LivePty {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
        };
        self.sessions
            .lock()
            .map_err(|_| ToolError::new("pty lock"))?
            .insert(id.clone(), live);
        let events = Arc::clone(&self.events);
        let session = id.clone();
        let plugin = plugin_id.to_string();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        events(json!({
                            "event": "pty.data",
                            "pluginId": plugin,
                            "session": session,
                            "data": data,
                        }));
                    }
                    Err(_) => break,
                }
            }
            events(json!({
                "event": "pty.exit",
                "pluginId": plugin,
                "session": session,
            }));
        });
        Ok(id)
    }

    pub fn write(&self, session: &str, data: &str) -> Result<(), ToolError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| ToolError::new("pty lock"))?;
        let live = sessions
            .get(session)
            .ok_or_else(|| ToolError::new("pty session not found"))?;
        let mut writer = live
            .writer
            .lock()
            .map_err(|_| ToolError::new("pty writer lock"))?;
        writer
            .write_all(data.as_bytes())
            .map_err(|e| ToolError::new(e.to_string()))?;
        writer.flush().map_err(|e| ToolError::new(e.to_string()))
    }

    pub fn resize(&self, session: &str, cols: u16, rows: u16) -> Result<(), ToolError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| ToolError::new("pty lock"))?;
        let live = sessions
            .get(session)
            .ok_or_else(|| ToolError::new("pty session not found"))?;
        let master = live
            .master
            .lock()
            .map_err(|_| ToolError::new("pty master lock"))?;
        master
            .resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(8),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ToolError::new(e.to_string()))
    }

    pub fn kill(&self, session: &str) -> Result<(), ToolError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ToolError::new("pty lock"))?;
        if let Some(live) = sessions.remove(session) {
            if let Ok(mut child) = live.child.lock() {
                let _ = child.kill();
            }
        }
        Ok(())
    }

    pub fn kill_all(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for (_, live) in sessions.drain() {
                if let Ok(mut child) = live.child.lock() {
                    let _ = child.kill();
                }
            }
        }
    }
}

fn default_shell() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "/bin/bash"
    }
}

/// Official plugins may drop shims in `<plugin>/bin` (e.g. `anya`). Prepend those dirs.
pub(crate) fn plugin_bin_dirs(plugins_root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Ok(entries) = fs::read_dir(plugins_root) else {
        return dirs;
    };
    let mut entries: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let bin = entry.path().join("bin");
        if bin.is_dir() {
            dirs.push(bin);
        }
    }
    dirs
}

fn apply_plugin_bin_path(cmd: &mut CommandBuilder) {
    let dirs = plugin_bin_dirs(&plugins_dir());
    if dirs.is_empty() {
        return;
    }
    let sep = if cfg!(windows) { ";" } else { ":" };
    let extra = dirs
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(sep);
    let old = std::env::var("PATH").unwrap_or_default();
    let next = if old.is_empty() {
        extra
    } else {
        format!("{extra}{sep}{old}")
    };
    cmd.env("PATH", next);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_bin_dirs_lists_bin_folders() {
        let root =
            std::env::temp_dir().join(format!("anya-plugin-bins-{}", Uuid::new_v4().simple()));
        let bin = root.join("sample").join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(root.join("terminal")).unwrap();
        let dirs = plugin_bin_dirs(&root);
        assert_eq!(dirs, vec![bin]);
        let _ = fs::remove_dir_all(root);
    }
}
