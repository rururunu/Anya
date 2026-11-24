//! Serialize Ghost MCP calls and bind the subprocess to a single computer-use turn.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::core::tools::{context::ToolContext, error::ToolError};
use serde_json::Value;

type Reply = Sender<Result<String, ToolError>>;
enum Command {
    Call {
        action: String,
        args: Value,
        stopped: Arc<AtomicBool>,
        cancelled: Option<Arc<AtomicBool>>,
        reply: Reply,
    },
    Stop(Option<Sender<()>>),
}

static WORKER: Mutex<Option<Sender<Command>>> = Mutex::new(None);

fn ensure_worker() -> Result<Sender<Command>, ToolError> {
    let mut worker = WORKER
        .lock()
        .map_err(|_| ToolError::new("Ghost worker lock"))?;
    if let Some(tx) = worker.as_ref() {
        return Ok(tx.clone());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| ToolError::new(format!("Create Ghost runtime: {err}")))?;
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("anya-ghost-mcp".into())
        .spawn(move || runtime.block_on(worker_main(rx)))
        .map_err(|err| ToolError::new(format!("Start Ghost worker: {err}")))?;
    *worker = Some(tx.clone());
    Ok(tx)
}

async fn worker_main(rx: Receiver<Command>) {
    let mut child = None;
    while let Ok(command) = rx.recv() {
        match command {
            Command::Stop(done) => {
                if let Some(process) = child.take() {
                    super::ghost_process::GhostProcess::stop(process, false).await;
                }
                if let Some(done) = done {
                    let _ = done.send(());
                }
            }
            Command::Call {
                action,
                args,
                stopped,
                cancelled,
                reply,
            } => {
                if is_stopped(&stopped, cancelled.as_deref()) {
                    let _ = reply.send(Err(ToolError::cancelled()));
                    continue;
                }
                let dispatch = async {
                    if child.is_none() {
                        child = Some(super::ghost_release::start().await?);
                        // A previous HUD stop can leave Ghost's Windows stop
                        // event set. Only clear it when starting a new run.
                        child
                            .as_mut()
                            .unwrap()
                            .call("ghost_reset", serde_json::json!({}))
                            .await?;
                    }
                    if is_stopped(&stopped, cancelled.as_deref()) {
                        return Err("Computer Use stopped before dispatch".to_owned());
                    }
                    super::ghost_ops::dispatch(child.as_mut().unwrap(), &action, &args).await
                };
                let result = tokio::select! {
                    biased;
                    _ = wait_for_stop(&stopped, cancelled.as_deref()) => Err(ToolError::cancelled()),
                    outcome = tokio::time::timeout(Duration::from_secs(120), dispatch) => match outcome {
                        Ok(outcome) => outcome.map_err(ToolError::new),
                        Err(_) => {
                            stopped.store(true, Ordering::Release);
                            Err(ToolError::new("Computer Use timed out. Inspect the target before retrying any dispatched action."))
                        }
                    },
                };
                if let Some(process) = child.as_mut() {
                    let transport_failed = process.transport_failed();
                    if transport_failed || is_stopped(&stopped, cancelled.as_deref()) {
                        child.take().unwrap().stop(true).await;
                        if transport_failed {
                            stopped.store(true, Ordering::Release);
                        }
                    }
                }
                let _ = reply.send(result);
            }
        }
    }
}

pub(super) fn call_with_cancel(
    action: &str,
    args: &Value,
    stopped: Arc<AtomicBool>,
    ctx: Option<&ToolContext>,
) -> Result<String, ToolError> {
    let tx = ensure_worker()?;
    let (reply, response) = mpsc::channel();
    tx.send(Command::Call {
        action: action.to_string(),
        args: args.clone(),
        stopped,
        cancelled: ctx.map(|ctx| Arc::clone(&ctx.cancelled)),
        reply,
    })
    .map_err(|_| ToolError::new("Ghost worker disconnected"))?;
    response
        .recv()
        .map_err(|_| ToolError::new("Ghost worker disconnected"))?
}

fn is_stopped(stopped: &AtomicBool, cancelled: Option<&AtomicBool>) -> bool {
    stopped.load(Ordering::Acquire) || cancelled.is_some_and(|flag| flag.load(Ordering::Acquire))
}

async fn wait_for_stop(stopped: &AtomicBool, cancelled: Option<&AtomicBool>) {
    while !is_stopped(stopped, cancelled) {
        tokio::time::sleep(Duration::from_millis(35)).await;
    }
}

pub(super) fn stop_session(wait: bool) {
    let worker = WORKER.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(tx) = worker.as_ref() {
        if wait {
            let (done, response) = mpsc::channel();
            if tx.send(Command::Stop(Some(done))).is_ok() {
                let _ = response.recv_timeout(Duration::from_secs(2));
            }
        } else {
            let _ = tx.send(Command::Stop(None));
        }
    }
}
