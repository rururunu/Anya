use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::core::ai::provider::AIProvider;
use crate::core::chat::compact::{self, is_compaction_summary, CompactPriorResult};
use crate::core::runtime::{ChatMessage, ChatRequest, StreamEvent};

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

pub struct MidTurnCompactOutcome {
    #[allow(dead_code)]
    pub folded_count: usize,
    pub summary: ChatMessage,
    /// When set, persist the summary into conversation before this message id
    /// (prior-turn compact). `None` for in-flight tool compact — those rows
    /// are not stored as separate conversation messages.
    pub persist_before_id: Option<String>,
    #[allow(dead_code)]
    pub estimated_tokens: usize,
}

/// Near the context window, auto-compact and continue: never hard-stop the
/// turn solely because tokens crossed a budget.
///
/// `last_compact_msg_len` skips a retry on the same message snapshot. Compact
/// cannot shrink tool schemas or the current user turn; without this guard the
/// loop re-folded (and persisted) prior history on every list/read step.
pub async fn maybe_compact(
    provider: &Arc<dyn AIProvider>,
    max_turn_tokens: usize,
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    used_tokens: &mut usize,
    last_compact_msg_len: &mut usize,
    tx: &mpsc::Sender<StreamEvent>,
) -> Option<MidTurnCompactOutcome> {
    if COMPACT_DISABLED.load(Ordering::Relaxed) {
        return None;
    }
    let compact_at = mid_turn_compact_threshold(max_turn_tokens);
    if compact_at == 0 || *used_tokens < compact_at {
        return None;
    }
    if request.messages.len() == *last_compact_msg_len {
        return None;
    }
    let outcome = force_compact(
        provider,
        max_turn_tokens,
        request,
        user_msg_index,
        used_tokens,
        tx,
    )
    .await;
    *last_compact_msg_len = request.messages.len();
    outcome
}

/// Compact unconditionally (ignoring the token threshold), used when the
/// provider already rejected the request for exceeding the context window.
pub async fn force_compact(
    provider: &Arc<dyn AIProvider>,
    max_turn_tokens: usize,
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    used_tokens: &mut usize,
    tx: &mpsc::Sender<StreamEvent>,
) -> Option<MidTurnCompactOutcome> {
    if COMPACT_DISABLED.load(Ordering::Relaxed) {
        return None;
    }
    let Some(user_idx) = *user_msg_index else {
        return None;
    };
    let _ = provider;

    let _ = tx
        .send(StreamEvent::Status {
            kind: "context_compacting".to_string(),
        })
        .await;

    let prior_already_summarized = user_idx > 0
        && request.messages[..user_idx]
            .iter()
            .any(is_compaction_summary);

    let applied = if user_idx > 0 && !prior_already_summarized {
        let prior = request.messages[..user_idx].to_vec();
        let current_turn = request.messages[user_idx..].to_vec();
        if let Some(folded) = compact::compact_prior(&prior, &request.session_id, None).await {
            summary_from(&folded).map(|summary| {
                let persist_before_id = first_kept_id(&folded);
                let folded_count = folded.folded_count;
                let mut new_messages = folded.messages;
                let new_user_idx = new_messages.len();
                new_messages.extend(current_turn);
                request.messages = new_messages;
                *user_msg_index = Some(new_user_idx);
                (folded_count, summary, persist_before_id)
            })
        } else {
            apply_after_user_fold(request, user_idx).await
        }
    } else {
        apply_after_user_fold(request, user_idx).await
    };

    let Some((folded_count, summary, persist_before_id)) = applied else {
        let _ = tx
            .send(StreamEvent::Status {
                kind: String::new(),
            })
            .await;
        return None;
    };
    *used_tokens = estimate_request_tokens(request);
    let window = max_turn_tokens.max(1);
    let ratio = *used_tokens as f32 / window as f32;
    let _ = tx
        .send(StreamEvent::Status {
            kind: format!(
                "context_compacted:{folded_count}:{ratio:.4}:{}:{window}",
                *used_tokens
            ),
        })
        .await;
    Some(MidTurnCompactOutcome {
        folded_count,
        summary,
        persist_before_id,
        estimated_tokens: *used_tokens,
    })
}

async fn apply_after_user_fold(
    request: &mut ChatRequest,
    user_idx: usize,
) -> Option<(usize, ChatMessage, Option<String>)> {
    // Mechanical only: a nested LLM summarize mid-turn would steal the
    // in-flight provider stream and add several seconds of latency.
    let folded =
        compact::compact_after_user(&request.messages, user_idx, &request.session_id, None).await?;
    let summary = summary_from(&folded)?;
    let folded_count = folded.folded_count;
    request.messages = folded.messages;
    Some((folded_count, summary, None))
}

fn first_kept_id(folded: &CompactPriorResult) -> Option<String> {
    folded
        .messages
        .iter()
        .find(|message| !is_compaction_summary(message))
        .map(|message| message.id.clone())
}

fn summary_from(folded: &CompactPriorResult) -> Option<ChatMessage> {
    folded
        .messages
        .iter()
        .find(|message| is_compaction_summary(message))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai::provider::{AIProvider, ProviderError};
    use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role};
    use async_trait::async_trait;

    struct NoopProvider;

    #[async_trait]
    impl AIProvider for NoopProvider {
        fn id(&self) -> &'static str {
            "noop"
        }

        async fn stream(
            &self,
            _request: ChatRequest,
            _tx: mpsc::Sender<StreamEvent>,
        ) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    fn msg(id: &str, role: Role, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            session_id: "s1".into(),
            role,
            content: content.into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        }
    }

    fn request_from(messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest {
            request_id: "r1".into(),
            session_id: "s1".into(),
            messages,
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        }
    }

    #[tokio::test]
    async fn skips_retry_on_unchanged_snapshot() {
        let provider: Arc<dyn AIProvider> = Arc::new(NoopProvider);
        let (tx, mut rx) = mpsc::channel(8);
        let big = "word ".repeat(8_000);
        let mut request = request_from(vec![
            msg("u0", Role::User, &big),
            msg("a0", Role::Assistant, &big),
            msg("u1", Role::User, "now list the folder"),
        ]);
        let mut user_idx = Some(2);
        let mut used = estimate_request_tokens(&request);
        let mut last_len = 0usize;

        let first = maybe_compact(
            &provider,
            8_000,
            &mut request,
            &mut user_idx,
            &mut used,
            &mut last_len,
            &tx,
        )
        .await;
        assert!(first.is_some(), "first pass should fold oversized prior");
        let after_first = request.messages.clone();

        used = 100_000;
        let second = maybe_compact(
            &provider,
            8_000,
            &mut request,
            &mut user_idx,
            &mut used,
            &mut last_len,
            &tx,
        )
        .await;
        assert!(second.is_none(), "same snapshot must not compact again");
        assert_eq!(request.messages.len(), after_first.len());
        while rx.try_recv().is_ok() {}
    }

    #[tokio::test]
    async fn does_not_refold_already_summarized_prior() {
        let provider: Arc<dyn AIProvider> = Arc::new(NoopProvider);
        let (tx, mut rx) = mpsc::channel(8);
        let kept = "kept tail ".repeat(200);
        let mut request = request_from(vec![
            msg(
                "compact-1",
                Role::System,
                "<compaction-summary>already folded</compaction-summary>",
            ),
            msg("u0", Role::User, &kept),
            msg("a0", Role::Assistant, &kept),
            msg("u1", Role::User, "read the next file"),
        ]);
        let ids_before: Vec<String> = request.messages.iter().map(|m| m.id.clone()).collect();
        let mut user_idx = Some(3);
        let mut used = 100_000;
        let mut last_len = 0usize;

        let outcome = maybe_compact(
            &provider,
            8_000,
            &mut request,
            &mut user_idx,
            &mut used,
            &mut last_len,
            &tx,
        )
        .await;
        assert!(outcome.is_none());
        let ids_after: Vec<String> = request.messages.iter().map(|m| m.id.clone()).collect();
        assert_eq!(ids_before, ids_after);
        while rx.try_recv().is_ok() {}
    }
}
