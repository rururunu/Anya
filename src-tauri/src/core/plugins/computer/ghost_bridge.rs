//! Dedicated GhostSession worker thread (COM affinity) + sync API for tools.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::core::tools::error::ToolError;

enum Cmd {
    Call {
        plugin_id: String,
        action: String,
        args: Value,
        reply: Sender<Result<String, String>>,
    },
    Stop {
        reply: Sender<()>,
    },
}

static TX: OnceLock<Mutex<Sender<Cmd>>> = OnceLock::new();

fn ensure_worker() -> Result<Sender<Cmd>, ToolError> {
    if let Some(tx) = TX.get() {
        return Ok(tx
            .lock()
            .map_err(|_| ToolError::new("ghost worker lock"))?
            .clone());
    }
    let (tx, rx) = mpsc::channel::<Cmd>();
    thread::Builder::new()
        .name("anya-ghost".into())
        .spawn(move || worker_main(rx))
        .map_err(|e| ToolError::new(format!("spawn ghost worker: {e}")))?;
    let _ = TX.set(Mutex::new(tx.clone()));
    thread::sleep(Duration::from_millis(50));
    Ok(tx)
}

fn worker_main(rx: Receiver<Cmd>) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(error = %e, "ghost tokio runtime failed");
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Cmd::Call { reply, .. } => {
                        let _ = reply.send(Err(format!("ghost runtime: {e}")));
                    }
                    Cmd::Stop { reply } => {
                        let _ = reply.send(());
                    }
                }
            }
            return;
        }
    };

    let session = match ghost_session::GhostSession::new() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "GhostSession::new failed");
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Cmd::Call { reply, .. } => {
                        let _ = reply.send(Err(format!("GhostSession init: {e}")));
                    }
                    Cmd::Stop { reply } => {
                        let _ = reply.send(());
                    }
                }
            }
            return;
        }
    };

    // Match Ghost MCP: background-only by default so every window-scoped verb
    // stays on posted/UIA paths. Unlock so rare chords (ctrl+s) can temporarily
    // raise; Anya still launches apps on the *user* desktop (not launch_hidden).
    ghost_session::engine::focus::set_lock(false);
    let _ = session.set_focus_policy("background");

    rt.block_on(async move {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                Cmd::Call {
                    plugin_id,
                    action,
                    args,
                    reply,
                } => {
                    // Re-assert unlock every call — env/default lock must not block canvas drag.
                    ghost_session::engine::focus::set_lock(false);
                    let out =
                        super::ghost_ops::dispatch(&session, &plugin_id, &action, &args).await;
                    let _ = reply.send(out);
                }
                Cmd::Stop { reply } => {
                    session.stop();
                    session.clear_anchor();
                    session.reset();
                    let _ = reply.send(());
                }
            }
        }
    });
}

/// Run a computer-use tool on the Ghost worker thread.
pub fn call(plugin_id: &str, action: &str, args: &Value) -> Result<String, ToolError> {
    let tx = ensure_worker()?;
    let (reply_tx, reply_rx) = mpsc::channel();
    tx.send(Cmd::Call {
        plugin_id: plugin_id.to_string(),
        action: action.to_string(),
        args: args.clone(),
        reply: reply_tx,
    })
    .map_err(|_| ToolError::new("ghost worker disconnected"))?;
    reply_rx
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| ToolError::new("ghost worker timeout"))?
        .map_err(ToolError::new)
}

/// Abort in-flight Ghost work, clear window anchor (HUD end / chat finished / plugin disable).
pub fn stop_session() {
    ghost_session::engine::input::hotkey::trigger_stop();
    let Some(guard) = TX.get() else {
        ghost_session::engine::input::hotkey::reset_stop();
        return;
    };
    let Ok(tx) = guard.lock().map(|g| g.clone()) else {
        return;
    };
    let (reply_tx, reply_rx) = mpsc::channel();
    if tx.send(Cmd::Stop { reply: reply_tx }).is_ok() {
        let _ = reply_rx.recv_timeout(Duration::from_secs(5));
    }
}
