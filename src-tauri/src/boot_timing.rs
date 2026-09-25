//! Boot-phase timing.
//!
//! `setup()` runs on the main thread, so anything slow there delays both the
//! window and the webview's first IPC — which is why every part of startup feels
//! late at once rather than one feature lagging. These marks go to the normal
//! tracing log so the phases can be read from the terminal, without webview
//! devtools.
//!
//! The first logged phase lands after `telemetry::init_logging`, because a mark
//! emitted before the subscriber exists is dropped.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static LAST_MARK_MS: AtomicU64 = AtomicU64::new(0);
static SEEN_PHASES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
static LOGGING_READY: AtomicBool = AtomicBool::new(false);
/// Marks taken before the subscriber exists, replayed by [`flush_pending`].
static PENDING: Mutex<Vec<(&'static str, u64, u64)>> = Mutex::new(Vec::new());

/// Baseline for every phase. Call first thing in `main()`.
pub fn mark_process_start() {
    let _ = PROCESS_START.get_or_init(Instant::now);
}

/// Log a phase with ms since [`mark_process_start`] and ms since the previous one.
pub fn phase(name: &'static str) {
    let Some(start) = PROCESS_START.get() else {
        return;
    };
    let elapsed = start.elapsed().as_millis() as u64;
    let previous = LAST_MARK_MS.swap(elapsed, Ordering::Relaxed);
    if LOGGING_READY.load(Ordering::Relaxed) {
        emit(name, elapsed, elapsed.saturating_sub(previous));
    } else if let Ok(mut queued) = PENDING.lock() {
        queued.push((name, elapsed, elapsed.saturating_sub(previous)));
    }
}

/// Replay the marks taken before `init_logging` installed the subscriber.
/// Call once, right after logging is initialised.
pub fn flush_pending() {
    LOGGING_READY.store(true, Ordering::Relaxed);
    let queued = PENDING
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default();
    for (name, since_start, since_prev) in queued {
        emit(name, since_start, since_prev);
    }
}

fn emit(name: &'static str, since_start_ms: u64, since_prev_ms: u64) {
    tracing::info!(
        target: "peek_lib::boot",
        phase = name,
        since_start_ms,
        since_prev_ms,
        "boot"
    );
}

/// Like [`phase`], but logs at most once per process for `name`.
pub fn phase_once(name: &'static str) {
    let seen = SEEN_PHASES.get_or_init(|| Mutex::new(HashSet::new()));
    let first = match seen.lock() {
        Ok(mut guard) => guard.insert(name),
        Err(poisoned) => poisoned.into_inner().insert(name),
    };
    if first {
        phase(name);
    }
}
