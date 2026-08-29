mod events;
mod lifecycle;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub(crate) use lifecycle::StreamSpawnInput;

pub struct StreamManager {
    pub(super) active_tasks: Arc<Mutex<HashMap<String, lifecycle::ActiveTask>>>,
    pub(super) epoch_counter: Arc<AtomicU64>,
}

impl StreamManager {
    /// Creates an empty stream manager with no active assistant tasks.
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            epoch_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Returns the assistant message id currently streaming for a session, if any.
    pub fn active_assistant_for_session(&self, session_id: &str) -> Option<String> {
        let active = self.active_tasks.lock().ok()?;
        active
            .iter()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, _)| id.clone())
    }

    /// Queues a soft-inject user message into an active stream for the session.
    pub fn soft_inject(&self, session_id: &str, content: String) -> Result<String, crate::core::chat::error::ChatError> {
        use crate::core::chat::error::ChatError;

        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }
        let mut active = self
            .active_tasks
            .lock()
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        let (message_id, task) = active
            .iter_mut()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, task)| (id.clone(), task))
            .ok_or(ChatError::MessageNotFound)?;
        if let Ok(mut queue) = task.soft_queue.lock() {
            queue.push_back(content);
        }
        Ok(message_id)
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifecycle::{ActiveTask, epoch_still_active};
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn epoch_mismatch_is_inactive() {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        tasks.lock().unwrap().insert(
            "m1".into(),
            ActiveTask {
                session_id: "s1".into(),
                epoch: 2,
                cancelled: Arc::new(AtomicBool::new(false)),
                soft_queue: Arc::new(Mutex::new(VecDeque::new())),
                content: Arc::new(Mutex::new(String::new())),
                reasoning: Arc::new(Mutex::new(String::new())),
            },
        );
        assert!(!epoch_still_active(&tasks, "m1", 1));
        assert!(epoch_still_active(&tasks, "m1", 2));
    }
}
