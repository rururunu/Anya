//! Shared cancellation lifetime for pending tool and filesystem decisions.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::error::ToolError;

pub(crate) fn wait<T>(
    receiver: &Receiver<T>,
    cancelled: Option<&AtomicBool>,
    timeout: Duration,
) -> Result<T, ToolError> {
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(ToolError::cancelled());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ToolError::policy_denied(
                "permission request timed out; the user did not reject it",
            ));
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(value) => {
                if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    return Err(ToolError::cancelled());
                }
                return Ok(value);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(ToolError::policy_denied(
                    "permission answerer unavailable; the user did not reject it",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};

    #[test]
    fn cancelling_pending_decision_unblocks_without_an_answer() {
        let (_sender, receiver) = mpsc::channel::<bool>();
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = cancelled.clone();
        let waiter =
            std::thread::spawn(move || wait(&receiver, Some(&signal), Duration::from_secs(10)));
        cancelled.store(true, Ordering::Relaxed);
        assert!(waiter.join().unwrap().is_err());
    }

    #[test]
    fn cancellation_wins_over_an_already_queued_grant() {
        let (sender, receiver) = mpsc::channel();
        sender.send(true).unwrap();
        let cancelled = AtomicBool::new(true);
        assert!(wait(&receiver, Some(&cancelled), Duration::from_secs(1)).is_err());
    }

    #[test]
    fn missing_answerer_fails_closed() {
        let (sender, receiver) = mpsc::channel::<bool>();
        drop(sender);
        let error = wait(&receiver, None, Duration::from_secs(1)).unwrap_err();
        assert!(!error.is_user_denied());
    }
}
