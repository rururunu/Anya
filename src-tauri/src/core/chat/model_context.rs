//! Per-model context window caps.
//!
//! The settings toggle chooses 64k vs 1M as a *ceiling*. Actual budgets must
//! never exceed what the selected model can accept, otherwise compaction
//! stays quiet while the provider already rejects oversized prompts.

use super::limits::{DEFAULT_MAX_TURN_TOKENS, LARGE_MAX_TURN_TOKENS};

/// Best-effort native context window for a model id.
///
/// Returns `None` when unknown — callers should fall back to a conservative cap
/// instead of assuming 1M.
pub fn model_native_context_window(model_id: &str) -> Option<usize> {
    let id = model_id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }

    // DeepSeek V4 (Pro / Flash): official 1M context. Must run before the
    // generic `deepseek` 128k cap and before the Gemini `v4-flash` heuristic.
    if id.contains("deepseek-v4") || id.contains("deepseek_v4") {
        return Some(LARGE_MAX_TURN_TOKENS);
    }

    // DeepSeek V3 chat / reasoner: 128k.
    if id.contains("deepseek") {
        return Some(128_000);
    }

    // Modern Gemini / long-context flash (incl. Antigravity "v4-flash" style ids).
    if id.contains("gemini")
        || id.contains("antigravity")
        || ((id.contains("v4-flash") || id.contains("v3-flash")) && !id.contains("lite"))
    {
        if id.contains("1.0") || id.contains("pro-vision") {
            return Some(32_768);
        }
        return Some(LARGE_MAX_TURN_TOKENS);
    }

    if id.contains("claude") {
        return Some(200_000);
    }

    if id.contains("gpt-4.1")
        || id.contains("gpt-4o")
        || id.starts_with("o3")
        || id.starts_with("o4")
    {
        return Some(200_000);
    }

    if id.contains("gpt-4") || id.contains("gpt-3.5") {
        return Some(128_000);
    }

    None
}

/// Resolve the effective context window for compaction / usage meters.
///
/// - Large-context **off** → 64k (or lower if the model is smaller).
/// - Large-context **on** → min(1M, model native). Unknown models cap at 256k
///   so a small chat model never inherits a fake 1M budget.
pub fn effective_context_window(large_context_enabled: bool, model_id: &str) -> usize {
    let configured = if large_context_enabled {
        LARGE_MAX_TURN_TOKENS
    } else {
        DEFAULT_MAX_TURN_TOKENS
    };
    let native = model_native_context_window(model_id).unwrap_or(if large_context_enabled {
        256_000
    } else {
        DEFAULT_MAX_TURN_TOKENS
    });
    configured.min(native)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_context_respects_deepseek_v3_cap() {
        assert_eq!(effective_context_window(true, "deepseek-chat"), 128_000);
        assert_eq!(effective_context_window(true, "deepseek-reasoner"), 128_000);
    }

    #[test]
    fn large_context_allows_deepseek_v4_1m() {
        assert_eq!(
            effective_context_window(true, "deepseek-v4-pro"),
            LARGE_MAX_TURN_TOKENS
        );
        assert_eq!(
            effective_context_window(true, "deepseek-v4-flash"),
            LARGE_MAX_TURN_TOKENS
        );
        assert_eq!(
            effective_context_window(true, "deepseek-ai/DeepSeek-V4-Pro"),
            LARGE_MAX_TURN_TOKENS
        );
    }

    #[test]
    fn large_context_allows_gemini_1m() {
        assert_eq!(
            effective_context_window(true, "gemini-2.5-flash"),
            LARGE_MAX_TURN_TOKENS
        );
    }

    #[test]
    fn unknown_model_does_not_inherit_1m() {
        assert_eq!(effective_context_window(true, "mystery-model-9b"), 256_000);
    }

    #[test]
    fn large_context_off_stays_at_64k() {
        assert_eq!(
            effective_context_window(false, "gemini-2.5-pro"),
            DEFAULT_MAX_TURN_TOKENS
        );
        assert_eq!(
            effective_context_window(false, "deepseek-v4-pro"),
            DEFAULT_MAX_TURN_TOKENS
        );
    }
}
