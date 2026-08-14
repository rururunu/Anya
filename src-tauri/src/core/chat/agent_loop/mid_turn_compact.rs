use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::core::ai::provider::AIProvider;
use crate::core::runtime::{ChatRequest, StreamEvent};

use super::types::estimate_request_tokens;

static COMPACT_DISABLED: AtomicBool = AtomicBool::new(false);

/// Eval ablation: skip mid-turn auto-compact entirely.
pub fn set_compact_enabled(enabled: bool) {
    COMPACT_DISABLED.store(!enabled, Ordering::Relaxed);
}

/// Token count at which mid-turn auto-compact is attempted (near ~80-90% of
/// the context window). Crossing this budget never hard-stops the turn by
/// itself; it only triggers an attempt to fold prior history and continue.
pub fn mid_turn_compact_threshold(context_window: usize) -> usize {
    if context_window == 0 {
        return 0;
    }
    ((context_window as f32) * crate::core::chat::compact::COMPACT_TRIGGER_RATIO).ceil() as usize
}

/// Near the context window, auto-compact and continue: never hard-stop the
/// turn solely because tokens crossed a budget.
///
/// If `used_tokens` has crossed the mid-turn compact threshold and there is
/// prior history (messages before the current user turn) to fold, attempt to
/// summarize it. On success this mutates `request.messages`, `user_msg_index`,
/// and `used_tokens` in place and emits a `context_compacted` status on `tx`.
pub async fn maybe_compact(
    provider: &Arc<dyn AIProvider>,
    max_turn_tokens: usize,
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    used_tokens: &mut usize,
    tx: &mpsc::Sender<StreamEvent>,
) {
    if COMPACT_DISABLED.load(Ordering::Relaxed) {
        return;
    }
    let compact_at = mid_turn_compact_threshold(max_turn_tokens);
    if compact_at == 0 || *used_tokens < compact_at {
        return;
    }
    force_compact(provider, request, user_msg_index, used_tokens, tx).await;
}

/// Compact prior history unconditionally (ignoring the token threshold), used
/// when the provider already rejected the request for exceeding the context
/// window. Returns true when history was actually folded.
pub async fn force_compact(
    provider: &Arc<dyn AIProvider>,
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    used_tokens: &mut usize,
    tx: &mpsc::Sender<StreamEvent>,
) -> bool {
    if COMPACT_DISABLED.load(Ordering::Relaxed) {
        return false;
    }
    let Some(user_idx) = *user_msg_index else {
        return false;
    };
    if user_idx == 0 {
        return false;
    }

    let prior = &request.messages[..user_idx];
    let current_turn = request.messages[user_idx..].to_vec();
    let summarizer = crate::core::chat::compact::ProviderSummarizer::new(Arc::clone(provider));
    let Some(outcome) =
        crate::core::chat::compact::compact_prior(prior, &request.session_id, Some(&summarizer))
            .await
    else {
        return false;
    };

    let mut new_messages = outcome.messages;
    let new_user_idx = new_messages.len();
    new_messages.extend(current_turn);
    request.messages = new_messages;
    *user_msg_index = Some(new_user_idx);
    *used_tokens = estimate_request_tokens(request);
    let _ = tx
        .send(StreamEvent::Status {
            kind: "context_compacted".to_string(),
        })
        .await;
    true
}
