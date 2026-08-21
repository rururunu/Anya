/// `reqwest::blocking::Client` owns a nested Tokio runtime. Creating or dropping
/// that runtime on a Tokio worker (including some `spawn_blocking` threads)
/// panics with: "Cannot drop a runtime in a context where blocking is not allowed."
///
/// Run the closure on a fresh OS thread that has no current runtime handle.
pub fn run_isolated<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    join_isolated(f).unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

/// Like [`run_isolated`], but a panic in the worker becomes `on_panic` instead
/// of unwinding the caller (which is often a Tokio `spawn_blocking` thread).
pub fn run_isolated_or<T, F>(f: F, on_panic: impl FnOnce(String) -> T) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    join_isolated(f).unwrap_or_else(|payload| on_panic(panic_message(payload)))
}

fn join_isolated<T, F>(f: F) -> Result<T, Box<dyn std::any::Any + Send + 'static>>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    std::thread::Builder::new()
        .name("peek-isolated-blocking".into())
        .spawn(f)
        .expect("spawn peek-isolated-blocking thread")
        .join()
}

fn panic_message(payload: Box<dyn std::any::Any + Send + 'static>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|value| (*value).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

/// Drop a nested-runtime value off the current Tokio context.
pub fn drop_isolated<T: Send + 'static>(value: T) {
    let _ = std::thread::Builder::new()
        .name("peek-drop-blocking-runtime".into())
        .spawn(move || drop(value));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_isolated_or_converts_worker_panic() {
        let result = run_isolated_or(
            || -> Result<(), String> { panic!("boom") },
            |message| Err(message),
        );
        assert!(
            result.unwrap_err().contains("boom"),
            "panic payload should surface as an error string"
        );
    }
}
