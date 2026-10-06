use crate::core::context::models::ChatContext;
use crate::core::context::platform::WindowDetector;
use crate::core::context::provider::{CaptureProvider, CaptureResult};
use crate::core::context::strategy::{ActiveProvider, StrategyResolver};

pub enum ContextCaptureOutcome {
    Success(ChatContext),
    Empty,
}

/// Context Engine 唯一入口 — 负责采集并生成 ChatContext。
pub struct ContextManager {
    strategy: StrategyResolver,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            strategy: StrategyResolver::new(),
        }
    }

    pub fn capture(&self) -> ContextCaptureOutcome {
        let window = match WindowDetector::detect() {
            Ok(window) => window,
            Err(error) => {
                tracing::warn!(provider = "foreground_window", error = %error, "context provider failed");
                return ContextCaptureOutcome::Empty;
            }
        };
        self.capture_window(window)
    }

    pub fn capture_window(
        &self,
        window: crate::core::context::models::WindowInfo,
    ) -> ContextCaptureOutcome {
        tracing::debug!(
            provider = "foreground_window",
            process = %window.process_name,
            pid = window.pid,
            title = %window.title,
            "context foreground detected"
        );

        if window.pid == std::process::id() {
            tracing::debug!(
                process = %window.process_name,
                strategy = "ignored_self",
                "context strategy selected"
            );
            return ContextCaptureOutcome::Empty;
        }

        if !WindowDetector::detect().is_ok_and(|current| {
            current.hwnd == window.hwnd
                && current.pid == window.pid
                && current.title == window.title
        }) {
            return ContextCaptureOutcome::Success(ChatContext {
                window: Some(window),
                ..ChatContext::empty()
            });
        }
        let provider = self.strategy.resolve(&window);
        if let Some(selection) = crate::services::browser_selection::selection_for(&window) {
            return ContextCaptureOutcome::Success(ChatContext {
                selected_text: Some(selection.text),
                selected_files: Vec::new(),
                selected_images: Vec::new(),
                source: Some(crate::core::context::models::CaptureSource::Browser),
                window: Some(window),
            });
        }
        let strategy_name = match &provider {
            ActiveProvider::Explorer(_) => "explorer",
            ActiveProvider::Clipboard(_) => "clipboard",
        };
        tracing::debug!(
            process = %window.process_name,
            strategy = strategy_name,
            "context strategy selected"
        );
        let capture_result = match provider {
            ActiveProvider::Explorer(provider) => {
                capture_safely("explorer", || provider.capture(&window))
            }
            ActiveProvider::Clipboard(provider) => {
                capture_safely("clipboard", || provider.capture(&window))
            }
        };

        let context = match capture_result {
            CaptureResult::Success(partial) => ChatContext {
                selected_text: partial.selected_text,
                selected_files: partial.selected_files,
                selected_images: partial.selected_images,
                source: Some(partial.source),
                window: Some(window),
            },
            CaptureResult::Empty => ChatContext {
                selected_text: None,
                selected_files: Vec::new(),
                selected_images: Vec::new(),
                source: None,
                window: Some(window),
            },
        };
        tracing::debug!(
            process = %context.window.as_ref().map(|item| item.process_name.as_str()).unwrap_or("unknown"),
            strategy = strategy_name,
            window_available = context.window.is_some(),
            has_selected_text = context.selected_text.as_ref().is_some_and(|text| !text.trim().is_empty()),
            selected_files = context.selected_files.len(),
            selected_images = context.selected_images.len(),
            "context captured"
        );
        ContextCaptureOutcome::Success(context)
    }
}

fn capture_safely<F>(provider: &'static str, capture: F) -> CaptureResult
where
    F: FnOnce() -> CaptureResult,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(capture)) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(provider, "context provider panicked; using empty context");
            CaptureResult::Empty
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_panic_degrades_to_empty() {
        let result = capture_safely("test", || panic!("provider failure"));
        assert_eq!(result, CaptureResult::Empty);
    }
}
