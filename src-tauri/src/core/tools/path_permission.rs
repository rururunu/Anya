use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::error::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAccess {
    Read,
    Write,
}

impl PathAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

struct PendingPermission {
    sequence: u64,
    sender: mpsc::Sender<PermissionDecision>,
    session_id: String,
    path: String,
    operation: String,
    tool_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPathPermissionSnapshot {
    pub sequence: u64,
    pub request_id: String,
    pub session_id: String,
    pub path: String,
    pub operation: String,
    pub tool_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionDecision {
    AllowOnce,
    Deny,
}

impl PermissionDecision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "allow_once" => Some(Self::AllowOnce),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

pub struct PathPermissionStore {
    pending: Mutex<HashMap<String, PendingPermission>>,
}

impl PathPermissionStore {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Completes a pending request. Returns the session id when a waiter was notified.
    pub fn complete(&self, request_id: &str, decision: &str) -> Option<String> {
        let Some(decision) = PermissionDecision::parse(decision) else {
            return None;
        };
        let pending = self
            .pending
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(request_id));
        let Some(pending) = pending else {
            return None;
        };
        let session_id = pending.session_id.clone();
        let _ = pending.sender.send(decision);
        Some(session_id)
    }

    pub fn pending_items(&self) -> Vec<PendingPathPermissionSnapshot> {
        self.pending
            .lock()
            .ok()
            .map(|guard| {
                guard
                    .iter()
                    .map(|(request_id, pending)| PendingPathPermissionSnapshot {
                        sequence: pending.sequence,
                        request_id: request_id.clone(),
                        session_id: pending.session_id.clone(),
                        path: pending.path.clone(),
                        operation: pending.operation.clone(),
                        tool_name: pending.tool_name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn request_and_grant(
        &self,
        session_id: &str,
        event_bus: &Arc<dyn EventBus>,
        path: PathBuf,
        access: PathAccess,
        tool_name: &str,
    ) -> Result<PathBuf, ToolError> {
        self.request_and_grant_with_cancel(
            session_id, event_bus, path, access, tool_name, None, None,
        )
    }

    pub(crate) fn request_and_grant_with_cancel(
        &self,
        session_id: &str,
        event_bus: &Arc<dyn EventBus>,
        path: PathBuf,
        access: PathAccess,
        tool_name: &str,
        cancelled: Option<&std::sync::atomic::AtomicBool>,
        context: Option<&super::context::ToolContext>,
    ) -> Result<PathBuf, ToolError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        if let Some(ctx) = context {
            ctx.conversation.record_permission_event(
                session_id,
                &ctx.assistant_message_id,
                &request_id,
                "approval/asked",
                serde_json::json!({"toolName":tool_name,"path":path,"access":access.as_str()}),
            )?;
        }
        let (tx, rx) = mpsc::channel();
        if let Ok(mut guard) = self.pending.lock() {
            guard.insert(
                request_id.clone(),
                PendingPermission {
                    sequence: super::context::next_interaction_sequence(),
                    sender: tx,
                    session_id: session_id.to_string(),
                    path: path.display().to_string(),
                    operation: access.as_str().to_string(),
                    tool_name: tool_name.to_string(),
                },
            );
        }

        event_bus.emit(BusEvent::PathPermissionRequest {
            session_id: session_id.to_string(),
            request_id: request_id.clone(),
            path: path.display().to_string(),
            operation: access.as_str().to_string(),
            tool_name: tool_name.to_string(),
        });

        let decision = super::approval_wait::wait(&rx, cancelled, Duration::from_secs(600));
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&request_id);
        }
        event_bus.emit(BusEvent::InteractionResolved {
            session_id: session_id.into(),
            request_id: request_id.clone(),
            kind: "path_permission".into(),
        });
        if let Some(ctx) = context {
            let outcome = match &decision {
                Ok(PermissionDecision::Deny) => "rejected",
                Ok(_) => "allowed-once",
                Err(_) if ctx.is_cancelled() => "cancelled",
                Err(_) => "unavailable",
            };
            ctx.conversation.record_permission_event(
                session_id,
                &ctx.assistant_message_id,
                &request_id,
                "approval/decided",
                serde_json::json!({"outcome":outcome}),
            )?;
        }
        let decision = decision?;

        if decision == PermissionDecision::Deny {
            return Err(ToolError::user_denied("path access denied by user"));
        }

        Ok(path)
    }
}

impl Default for PathPermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_path_access_requires_a_fresh_decision() {
        struct Bus;
        impl EventBus for Bus {
            fn emit(&self, _: BusEvent) {}
        }
        let store = Arc::new(PathPermissionStore::new());
        for _ in 0..2 {
            let waiter = store.clone();
            let handle = std::thread::spawn(move || {
                let bus: Arc<dyn EventBus> = Arc::new(Bus);
                waiter.request_and_grant(
                    "same-session",
                    &bus,
                    PathBuf::from("outside.txt"),
                    PathAccess::Read,
                    "read",
                )
            });
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            let id = loop {
                if let Some(item) = store.pending_items().first() {
                    break item.request_id.clone();
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(store.complete(&id, "allow_once").is_some());
            assert!(handle.join().unwrap().is_ok());
            assert!(store.pending_items().is_empty());
        }
    }

    #[test]
    fn pending_items_surface_path_requests() {
        use crate::core::event::{BusEvent, EventBus};
        use std::sync::Arc;
        use std::time::Duration;

        struct NullBus;
        impl EventBus for NullBus {
            fn emit(&self, _event: BusEvent) {}
        }

        let store = Arc::new(PathPermissionStore::new());
        let bus: Arc<dyn EventBus> = Arc::new(NullBus);
        let waiter = Arc::clone(&store);
        let bus_clone = Arc::clone(&bus);
        let handle = std::thread::spawn(move || {
            waiter.request_and_grant(
                "s1",
                &bus_clone,
                PathBuf::from("/tmp/a.txt"),
                PathAccess::Read,
                "read_file",
            )
        });
        let started = std::time::Instant::now();
        let items = loop {
            let items = store.pending_items();
            if !items.is_empty() {
                break items;
            }
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "timed out waiting for pending path permission"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].session_id, "s1");
        assert_eq!(items[0].tool_name, "read_file");
        assert!(store.complete(&items[0].request_id, "deny").is_some());
        let result = handle.join().expect("thread");
        assert!(result.is_err());
        assert!(store.pending_items().is_empty());
    }

    #[test]
    fn permission_decisions_are_explicit() {
        assert_eq!(
            PermissionDecision::parse("allow_once"),
            Some(PermissionDecision::AllowOnce)
        );
        assert_eq!(PermissionDecision::parse("allow_always"), None);
        assert_eq!(
            PermissionDecision::parse("deny"),
            Some(PermissionDecision::Deny)
        );
        assert_eq!(PermissionDecision::parse("yes"), None);
    }
}
