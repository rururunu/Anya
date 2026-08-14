use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::core::ai::provider::ProviderError;
use crate::core::runtime::{StreamEvent, ToolCallPayload};

use super::types::merge_tool_call;

/// Everything collected from one provider stream turn once it finishes (or
/// the provider short-circuits with a `TurnComplete`/`Finish` event).
pub struct StreamTurnResult {
    pub content: String,
    pub reasoning: String,
    pub tool_calls: Vec<ToolCallPayload>,
    pub finish_reason: Option<String>,
}

/// Drain a single provider stream turn on `turn_rx`, forwarding
/// Delta/Reasoning/Status/Usage/UserContentPatch events to the outer `tx` as
/// they arrive, and folding everything else into a [`StreamTurnResult`].
///
/// Returns `Err` on cancellation or a provider-reported error (the error is
/// also forwarded to `tx` before returning).
pub async fn collect_stream_turn(
    mut turn_rx: mpsc::Receiver<StreamEvent>,
    tx: &mpsc::Sender<StreamEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<StreamTurnResult, ProviderError> {
    tracing::debug!("stream_turn start");
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tool_calls = Vec::new();
    let mut finish_reason = None;

    while let Some(event) = turn_rx.recv().await {
        if cancelled.load(Ordering::Relaxed) {
            return Err(ProviderError::cancelled());
        }
        match event {
            StreamEvent::Start => {}
            StreamEvent::Delta(delta) => {
                content.push_str(&delta);
                let _ = tx.send(StreamEvent::Delta(delta)).await;
            }
            StreamEvent::Reasoning(chunk) => {
                reasoning.push_str(&chunk);
                let _ = tx.send(StreamEvent::Reasoning(chunk)).await;
            }
            StreamEvent::Status { kind } => {
                let _ = tx.send(StreamEvent::Status { kind }).await;
            }
            StreamEvent::UserContentPatch {
                message_id,
                content,
            } => {
                let _ = tx
                    .send(StreamEvent::UserContentPatch {
                        message_id,
                        content,
                    })
                    .await;
            }
            StreamEvent::ToolCall(call) => {
                merge_tool_call(&mut tool_calls, call);
            }
            StreamEvent::Usage(usage) => {
                let _ = tx.send(StreamEvent::Usage(usage)).await;
            }
            StreamEvent::TurnComplete {
                content: turn_content,
                reasoning: turn_reasoning,
                tool_calls: turn_tool_calls,
                finish_reason: turn_finish,
            } => {
                content = turn_content;
                if let Some(value) = turn_reasoning {
                    reasoning = value;
                }
                tool_calls = turn_tool_calls;
                finish_reason = turn_finish;
            }
            StreamEvent::Finish => break,
            StreamEvent::Error(message) => {
                let _ = tx.send(StreamEvent::Error(message.clone())).await;
                return Err(ProviderError::message(message));
            }
        }
    }

    tracing::debug!(
        content_len = content.len(),
        reasoning_len = reasoning.len(),
        tool_calls = tool_calls.len(),
        "stream_turn done"
    );

    Ok(StreamTurnResult {
        content,
        reasoning,
        tool_calls,
        finish_reason,
    })
}
