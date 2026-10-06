use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::core::context::manager::{ContextCaptureOutcome, ContextManager};
use crate::core::context::models::{ChatContext, WindowInfo};
use crate::core::context::platform::WindowDetector;
use crate::core::runtime::RequestContext;

#[derive(Clone)]
struct StoredContext {
    id: u64,
    captured_at_ms: u64,
    context: ChatContext,
}

static CONTEXT_STORE: OnceLock<Mutex<Option<StoredContext>>> = OnceLock::new();
static CAPTURE_GATE: OnceLock<CaptureGate> = OnceLock::new();
static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct CaptureGate {
    state: Mutex<CaptureState>,
    ready: Condvar,
}

#[derive(Default)]
struct CaptureState {
    active_captures: usize,
    completed_captures: u64,
}

impl CaptureGate {
    fn begin(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.active_captures = state.active_captures.saturating_add(1);
        }
    }

    fn finish(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.active_captures = state.active_captures.saturating_sub(1);
            state.completed_captures = state.completed_captures.saturating_add(1);
            if state.active_captures == 0 {
                self.ready.notify_all();
            }
        }
    }

    fn wait_until_ready(&self) {
        self.wait_while(Duration::from_millis(1500), |state| {
            state.active_captures > 0
        });
    }

    fn wait_for_completed_capture(&self) {
        // Do not wait for a capture that never started (workbench `/context`).
        // Only block while an overlay capture is currently in flight.
        self.wait_until_ready();
    }

    fn wait_while(&self, timeout: Duration, mut pending: impl FnMut(&CaptureState) -> bool) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let deadline = Instant::now() + timeout;
        while pending(&state) {
            let now = Instant::now();
            if now >= deadline {
                tracing::warn!("context capture wait timed out");
                return;
            }
            match self
                .ready
                .wait_timeout(state, deadline.saturating_duration_since(now))
            {
                Ok((next, waited)) => {
                    state = next;
                    if waited.timed_out() && pending(&state) {
                        tracing::warn!("context capture wait timed out");
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    }
}

struct CaptureGuard {
    gate: &'static CaptureGate,
    started_at_ms: u64,
}

impl CaptureGuard {
    fn begin() -> Self {
        let gate = CAPTURE_GATE.get_or_init(CaptureGate::default);
        gate.begin();
        let started_at_ms = now_millis();
        tracing::debug!(started_at_ms, "capture_now start");
        Self {
            gate,
            started_at_ms,
        }
    }
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        self.gate.finish();
        tracing::debug!(
            started_at_ms = self.started_at_ms,
            finished_at_ms = now_millis(),
            "capture_now end"
        );
    }
}

fn store() -> &'static Mutex<Option<StoredContext>> {
    CONTEXT_STORE.get_or_init(|| Mutex::new(None))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Snapshot the current foreground window without clipboard / UIA / Explorer COM.
///
/// Alt+Alt show must not wait on those providers; they run after the window is visible.
pub fn snapshot_foreground() -> (RequestContext, Option<WindowInfo>) {
    let window = WindowDetector::detect()
        .ok()
        .filter(|window| window.pid != std::process::id());
    let captured = ChatContext {
        window: window.clone(),
        ..ChatContext::empty()
    };
    if let Ok(mut guard) = store().lock() {
        let stored = StoredContext {
            id: NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
            captured_at_ms: now_millis(),
            context: captured.clone(),
        };
        log_snapshot("snapshot_foreground stored context", &stored);
        *guard = Some(stored);
    }
    (map_to_request_context(Some(&captured)), window)
}

/// 在 overlay 显示前采集前台上下文并缓存。
pub fn capture_now() -> RequestContext {
    let context_store = store();
    tracing::debug!(
        context_store_address = %format_args!("{:p}", context_store),
        "capture_now ContextStore instance"
    );
    let _capture_guard = CaptureGuard::begin();
    let manager = ContextManager::new();
    let captured = match manager.capture() {
        ContextCaptureOutcome::Success(context) => context,
        ContextCaptureOutcome::Empty => ChatContext::empty(),
    };

    if let Ok(mut guard) = store().lock() {
        let stored = StoredContext {
            id: NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
            captured_at_ms: now_millis(),
            context: captured.clone(),
        };
        log_snapshot("capture_now stored context", &stored);
        *guard = Some(stored);
    }

    map_to_request_context(Some(&captured))
}

/// Capture a frozen source before the overlay takes focus. Never reuse old selections.
pub fn capture_for_window(window: WindowInfo) -> RequestContext {
    let _capture_guard = CaptureGuard::begin();
    let id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut guard) = store().lock() {
        *guard = Some(StoredContext {
            id,
            captured_at_ms: now_millis(),
            context: ChatContext {
                window: Some(window.clone()),
                ..ChatContext::empty()
            },
        });
    }
    let captured = match ContextManager::new().capture_window(window) {
        ContextCaptureOutcome::Success(context) => context,
        ContextCaptureOutcome::Empty => ChatContext::empty(),
    };
    tracing::debug!(has_selection = captured.has_content(), source = ?captured.source, "frozen-source capture completed");
    if let Ok(mut guard) = store().lock() {
        finish_snapshot(&mut guard, id, captured.clone());
    }
    map_to_request_context(Some(&captured))
}

fn finish_snapshot(stored: &mut Option<StoredContext>, id: u64, context: ChatContext) {
    if stored.as_ref().is_some_and(|latest| latest.id == id) {
        *stored = Some(StoredContext {
            id,
            captured_at_ms: now_millis(),
            context,
        });
    }
}

pub fn latest_request_context() -> RequestContext {
    let context_store = store();
    tracing::debug!(
        context_store_address = %format_args!("{:p}", context_store),
        "latest_request_context ContextStore instance"
    );
    CAPTURE_GATE
        .get_or_init(CaptureGate::default)
        .wait_until_ready();
    let guard = store().lock().ok();
    let context = guard.and_then(|value| value.clone());

    match context.as_ref() {
        Some(stored) => log_snapshot("latest_request_context found", stored),
        None => tracing::debug!(found = false, "latest_request_context snapshot lookup"),
    }

    map_to_request_context(context.as_ref().map(|stored| &stored.context))
}

/// Wait for an in-flight overlay capture, then return.
///
/// Workbench `/context` must not block until overlay capture has ever run —
/// that wait never completes and freezes the workbench IPC thread.
pub fn wait_for_completed_capture() {
    CAPTURE_GATE
        .get_or_init(CaptureGate::default)
        .wait_for_completed_capture();
}

fn log_snapshot(message: &'static str, stored: &StoredContext) {
    tracing::debug!(
        context_id = stored.id,
        captured_at_ms = stored.captured_at_ms,
        found = true,
        window_process = ?stored
            .context
            .window
            .as_ref()
            .map(|window| window.process_name.as_str()),
        window_title = ?stored
            .context
            .window
            .as_ref()
            .map(|window| window.title.as_str()),
        message
    );
}

fn map_to_request_context(context: Option<&ChatContext>) -> RequestContext {
    let Some(context) = context else {
        return RequestContext::default();
    };

    let selected_files = context
        .selected_files
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    RequestContext {
        selection: context.selected_text.clone(),
        selected_files,
        selected_images: context.selected_images.clone(),
        active_window: context.window.as_ref().map(|window| {
            let title = window.title.trim();
            let mut label = if title.is_empty() {
                format!("{} (pid {})", window.process_name, window.pid)
            } else {
                format!("{} - {} (pid {})", window.process_name, title, window.pid)
            };
            if let Some(selection) = crate::services::browser_selection::selection_for(window) {
                if context.selected_text.as_deref() == Some(selection.text.as_str()) {
                    label.push_str(&format!("\nPage URL: {}", selection.url));
                }
            }
            label
        }),
        active_file: None,
        workspace: None,
        clipboard: None,
        git_status: None,
        last_shell_execution: None,
        ide_context: None,
    }
}

#[cfg(test)]
mod capture_timing_tests {
    use super::*;
    #[test]
    fn late_capture_cannot_overwrite_a_newer_shortcut_snapshot() {
        let mut stored = Some(StoredContext {
            id: 2,
            captured_at_ms: 0,
            context: ChatContext {
                selected_text: Some("newer window".into()),
                ..ChatContext::empty()
            },
        });
        finish_snapshot(
            &mut stored,
            1,
            ChatContext {
                selected_text: Some("late old window".into()),
                ..ChatContext::empty()
            },
        );
        assert_eq!(
            stored.as_ref().unwrap().context.selected_text.as_deref(),
            Some("newer window")
        );
        finish_snapshot(
            &mut stored,
            2,
            ChatContext {
                selected_text: Some("current capture".into()),
                ..ChatContext::empty()
            },
        );
        assert_eq!(
            stored.as_ref().unwrap().context.selected_text.as_deref(),
            Some("current capture")
        );
    }
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn context_query_waits_until_capture_is_complete() {
        let gate = Arc::new(CaptureGate::default());
        gate.begin();
        let query_completed = Arc::new(AtomicBool::new(false));
        let waiter_gate = Arc::clone(&gate);
        let waiter_completed = Arc::clone(&query_completed);
        let waiter = std::thread::spawn(move || {
            waiter_gate.wait_until_ready();
            waiter_completed.store(true, Ordering::Release);
        });

        std::thread::sleep(Duration::from_millis(20));
        assert!(!query_completed.load(Ordering::Acquire));

        gate.finish();
        waiter.join().expect("context query waiter");
        assert!(query_completed.load(Ordering::Acquire));
    }

    #[test]
    fn capture_and_query_share_the_same_context_store_instance() {
        let capture_store = store() as *const Mutex<Option<StoredContext>>;
        let query_store = store() as *const Mutex<Option<StoredContext>>;
        assert_eq!(capture_store, query_store);
    }

    #[test]
    fn overlay_context_waits_only_while_capture_is_in_flight() {
        let gate = Arc::new(CaptureGate::default());
        let snapshot = Arc::new(Mutex::new(None::<RequestContext>));
        gate.begin();

        let waiter_gate = Arc::clone(&gate);
        let waiter_snapshot = Arc::clone(&snapshot);
        let waiter = std::thread::spawn(move || {
            waiter_gate.wait_for_completed_capture();
            waiter_snapshot.lock().ok().and_then(|value| value.clone())
        });

        std::thread::sleep(Duration::from_millis(20));
        assert!(
            !waiter.is_finished(),
            "/context must wait while overlay capture is running"
        );

        *snapshot.lock().expect("snapshot lock") = Some(RequestContext {
            active_window: Some("Code.exe - Anya".to_string()),
            ..RequestContext::default()
        });
        gate.finish();

        let resolved = waiter
            .join()
            .expect("context waiter")
            .expect("non-empty snapshot");
        assert_eq!(resolved.active_window.as_deref(), Some("Code.exe - Anya"));
    }

    #[test]
    fn workbench_context_does_not_wait_for_overlay_capture() {
        let gate = CaptureGate::default();
        let started = Instant::now();
        gate.wait_for_completed_capture();
        assert!(
            started.elapsed() < Duration::from_millis(200),
            "/context in the workbench must not block until overlay capture has run"
        );
    }
}
