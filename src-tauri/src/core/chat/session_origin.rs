//! Tracks which side (desktop workbench or paired phone) sent the most
//! recent turn for a session, so the prompt can tell the agent when it must
//! treat file requests as needing a real `share_to_companion` delivery.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOrigin {
    Desktop,
    Companion,
}

pub struct SessionOriginStore {
    origins: Mutex<HashMap<String, RequestOrigin>>,
}

impl SessionOriginStore {
    pub fn new() -> Self {
        Self {
            origins: Mutex::new(HashMap::new()),
        }
    }

    pub fn mark(&self, session_id: &str, origin: RequestOrigin) {
        if let Ok(mut guard) = self.origins.lock() {
            guard.insert(session_id.to_string(), origin);
        }
    }

    /// Defaults to `Desktop` for unknown sessions (e.g. tests, eval harness).
    pub fn get(&self, session_id: &str) -> RequestOrigin {
        self.origins
            .lock()
            .ok()
            .and_then(|guard| guard.get(session_id).copied())
            .unwrap_or(RequestOrigin::Desktop)
    }

    pub fn is_companion(&self, session_id: &str) -> bool {
        self.get(session_id) == RequestOrigin::Companion
    }
}

pub fn shared_session_origin_store() -> &'static SessionOriginStore {
    static STORE: OnceLock<SessionOriginStore> = OnceLock::new();
    STORE.get_or_init(SessionOriginStore::new)
}
