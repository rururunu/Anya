use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent};

use super::types::now_millis;

/// Fold any queued soft-inject messages (follow-up instructions sent by the
/// user while the agent was mid-turn) into `request` as user messages, and
/// notify `tx` with a `soft_injected` status if anything was drained.
///
/// Called both at the top of the loop and right after a tool boundary, so a
/// follow-up lands before the next provider call as soon as possible.
pub async fn drain_soft_injects(
    soft_queue: &Arc<Mutex<VecDeque<String>>>,
    request: &mut ChatRequest,
    tx: &mpsc::Sender<StreamEvent>,
    user_msg_index: &mut Option<usize>,
) -> bool {
    let injected: Vec<String> = {
        let Ok(mut queue) = soft_queue.lock() else {
            return false;
        };
        queue.drain(..).collect()
    };
    if injected.is_empty() {
        return false;
    }

    for content in injected {
        let message = ChatMessage {
            id: format!("agent-followup-{}", uuid::Uuid::new_v4()),
            session_id: request.session_id.clone(),
            role: Role::User,
            content: format!("[Follow-up instruction while you were working]\n{content}"),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: now_millis(),
            estimated_tokens: None,
        };
        if user_msg_index.is_none() {
            *user_msg_index = Some(request.messages.len());
        }
        request.messages.push(message);
    }

    let _ = tx
        .send(StreamEvent::Status {
            kind: "soft_injected".to_string(),
        })
        .await;
    true
}
