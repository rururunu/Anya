use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::ghost_bridge;
use crate::core::tools::{context::ToolContext, error::ToolError};
use crate::services::computer_use_hud;

struct ComputerRun {
    message_id: Option<String>,
    turn_cancelled: Option<Arc<AtomicBool>>,
    stopped: Arc<AtomicBool>,
}

// A single desktop input engine is shared by all turns. Keep its window anchor
// with the owning turn until the stop/reset command has been queued.
static ACTIVE: Mutex<Option<ComputerRun>> = Mutex::new(None);

pub(super) fn execute(
    action: &str,
    args: &Value,
    ctx: Option<&ToolContext>,
) -> Result<String, ToolError> {
    let message_id = ctx.map(|ctx| ctx.assistant_message_id.as_str());
    let stopped = {
        let mut active = ACTIVE.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(ctx) = ctx {
            ctx.ensure_not_cancelled()?;
        }
        match active.as_ref() {
            Some(run) if message_id.is_some() && run.message_id.as_deref() == message_id => {
                if run.stopped.load(Ordering::Acquire) {
                    return Err(ToolError::new("Computer Use already stopped for this turn"));
                }
                Arc::clone(&run.stopped)
            }
            Some(_) => return Err(ToolError::new("Computer Use is busy in another run")),
            None => {
                let stopped = Arc::new(AtomicBool::new(false));
                *active = Some(ComputerRun {
                    message_id: message_id.map(str::to_owned),
                    turn_cancelled: ctx.map(|ctx| Arc::clone(&ctx.cancelled)),
                    stopped: Arc::clone(&stopped),
                });
                computer_use_hud::open();
                stopped
            }
        }
    };
    let result = ghost_bridge::call_with_cancel(action, args, Arc::clone(&stopped), ctx);
    if ctx.is_none() || ctx.is_some_and(ToolContext::is_cancelled) {
        stop_matching(|run| Arc::ptr_eq(&run.stopped, &stopped), false);
    }
    result
}

pub(super) fn stop(message_id: Option<&str>) -> Option<String> {
    stop_matching(
        |run| match message_id {
            Some(id) => run.message_id.as_deref() == Some(id),
            None => true,
        },
        message_id.is_none(),
    )
}

fn stop_matching(matches: impl FnOnce(&ComputerRun) -> bool, cancel_turn: bool) -> Option<String> {
    let mut active = ACTIVE.lock().unwrap_or_else(|err| err.into_inner());
    if !active.as_ref().is_some_and(matches) {
        return None;
    }
    let run = active.take()?;
    run.stopped.store(true, Ordering::Release);
    if cancel_turn {
        if let Some(cancelled) = run.turn_cancelled {
            cancelled.store(true, Ordering::Release);
        }
    }
    ghost_bridge::stop_session(false);
    computer_use_hud::close();
    run.message_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_finish_leaves_the_active_turn_running() {
        let stopped = Arc::new(AtomicBool::new(false));
        {
            let mut active = ACTIVE.lock().unwrap();
            assert!(active.is_none());
            *active = Some(ComputerRun {
                message_id: Some("current-message".into()),
                turn_cancelled: None,
                stopped: Arc::clone(&stopped),
            });
        }
        assert_eq!(stop(Some("other-message")), None);
        assert!(!stopped.load(Ordering::Acquire));
        assert_eq!(
            stop(Some("current-message")).as_deref(),
            Some("current-message")
        );
        assert!(stopped.load(Ordering::Acquire));
    }
}
