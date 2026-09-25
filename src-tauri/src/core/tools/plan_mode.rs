use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::core::tools::error::ToolError;

/// Hard plan-mode gate: when active, reject non-readonly mutating tools.
pub const PLAN_GATE_BLOCKED: &str =
    "plan mode is active: writer tools (and Shell) are not allowed until the user approves the plan or switches back to Agent. Call save_plan + update_tasks and stop, or wait for Approve & execute — do not retry writers.";

/// User-visible stop text when a writer hits the gate. Also used to detect
/// that the turn ended because of the gate (even if no checklist exists).
pub const PLAN_GATE_STOP_HINT: &str = "计划尚未批准";

/// `update_tasks` in plan mode without a prior `save_plan` this cycle.
pub const PLAN_TASKS_WITHOUT_SAVED_PLAN: &str = "plan mode requires save_plan before update_tasks: call save_plan with the full implementation proposal first, then resubmit this checklist";

/// `save_plan` after the user already approved — execution must not open a new proposal.
pub const SAVE_PLAN_REQUIRES_PLAN_MODE: &str =
    "save_plan is only available while plan mode is active. Continue executing the approved plan; do not start a new proposal this turn";

pub struct PlanModeStore {
    active_sessions: Mutex<HashSet<String>>,
    /// Sessions that actually have something to approve: a checklist, or a
    /// writer that hit the gate. Plan mode alone is not enough.
    awaiting_approval: Mutex<HashSet<String>>,
    /// Sessions where `save_plan` has already run during the current planning
    /// cycle. Reset whenever plan mode (re)activates so a stale save from a
    /// previous cycle never excuses skipping `save_plan` this time.
    plans_saved: Mutex<HashSet<String>>,
    /// Plan file the current planning cycle's `save_plan` actually wrote, keyed
    /// by root session. The default name is derived host-side from a timestamp
    /// and a slug, so no other layer can reconstruct it.
    plan_paths: Mutex<HashMap<String, String>>,
}

impl PlanModeStore {
    pub fn new() -> Self {
        Self {
            active_sessions: Mutex::new(HashSet::new()),
            awaiting_approval: Mutex::new(HashSet::new()),
            plans_saved: Mutex::new(HashSet::new()),
            plan_paths: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_active(&self, session_id: &str, active: bool) {
        if let Ok(mut guard) = self.active_sessions.lock() {
            if active {
                guard.insert(session_id.to_string());
            } else {
                guard.remove(session_id);
            }
        }
        if !active {
            self.clear_awaiting_approval(session_id);
        }
        // Either a fresh planning cycle starts or the previous one just
        // ended — both cases should forget any earlier `save_plan` call.
        if let Ok(mut guard) = self.plans_saved.lock() {
            guard.remove(session_id);
        }
        self.set_plan_path(session_id, None);
    }

    /// Record that `save_plan` wrote a proposal for this session's current
    /// planning cycle.
    pub fn mark_plan_saved(&self, session_id: &str) {
        self.sync_plan_saved(session_id, true);
    }

    /// Align `save_plan` memory with remaining history after a rewind.
    pub fn sync_plan_saved(&self, session_id: &str, saved: bool) {
        if let Ok(mut guard) = self.plans_saved.lock() {
            if saved {
                guard.insert(session_id.to_string());
            } else {
                guard.remove(session_id);
            }
        }
        if !saved {
            self.set_plan_path(session_id, None);
        }
    }

    /// Remember which file the current cycle's `save_plan` wrote.
    pub fn set_plan_path(&self, session_id: &str, path: Option<&str>) {
        if session_id.is_empty() {
            return;
        }
        if let Ok(mut guard) = self.plan_paths.lock() {
            match path {
                Some(path) => guard.insert(session_id.to_string(), path.to_string()),
                None => guard.remove(session_id),
            };
        }
    }

    /// Plan file written by the current cycle's `save_plan`, if any.
    pub fn plan_path(&self, session_id: &str) -> Option<String> {
        self.plan_paths
            .lock()
            .ok()
            .and_then(|guard| guard.get(session_id).cloned())
    }

    /// True once `save_plan` has run since plan mode last (re)activated.
    pub fn has_saved_plan(&self, session_id: &str) -> bool {
        self.plans_saved
            .lock()
            .ok()
            .is_some_and(|g| g.contains(session_id))
    }

    pub fn is_active(&self, session_id: &str) -> bool {
        self.active_sessions
            .lock()
            .ok()
            .is_some_and(|g| g.contains(session_id))
    }

    pub fn active_session_ids(&self) -> Vec<String> {
        self.active_sessions
            .lock()
            .ok()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn mark_awaiting_approval(&self, session_id: &str) {
        if !self.is_active(session_id) {
            return;
        }
        if let Ok(mut guard) = self.awaiting_approval.lock() {
            guard.insert(session_id.to_string());
        }
    }

    pub fn clear_awaiting_approval(&self, session_id: &str) {
        if let Ok(mut guard) = self.awaiting_approval.lock() {
            guard.remove(session_id);
        }
    }

    /// True only when plan mode is on *and* a plan (or gate stop) exists.
    pub fn is_awaiting_approval(&self, session_id: &str) -> bool {
        self.is_active(session_id)
            && self
                .awaiting_approval
                .lock()
                .ok()
                .is_some_and(|g| g.contains(session_id))
    }

    pub fn authorize(
        &self,
        session_id: &str,
        tool_name: &str,
        read_only: bool,
    ) -> Result<(), ToolError> {
        if !self.is_active(session_id) {
            return Ok(());
        }
        if plan_mode_allowed(tool_name, read_only) {
            return Ok(());
        }
        self.mark_awaiting_approval(session_id);
        Err(ToolError::new(PLAN_GATE_BLOCKED))
    }
}

pub fn content_requests_plan_approval(content: &str) -> bool {
    content.contains(PLAN_GATE_STOP_HINT)
}

pub fn task_status_is_open(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "pending" | "in_progress" | "active" | "running" | ""
    )
}

fn plan_mode_allowed(tool_name: &str, read_only: bool) -> bool {
    if read_only {
        return true;
    }
    matches!(
        tool_name,
        "save_plan"
            | "update_tasks"
            | "ask_user"
            | "share_to_companion"
            | "share_preview_url"
            | "todo_write"
            | "manage_plugin"
            | "request_plan_mode"
    )
}

pub fn shared_plan_mode_store() -> &'static PlanModeStore {
    static STORE: OnceLock<PlanModeStore> = OnceLock::new();
    STORE.get_or_init(PlanModeStore::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_mode_alone_does_not_await_approval() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        assert!(store.is_active("s"));
        assert!(!store.is_awaiting_approval("s"));
    }

    #[test]
    fn blocking_a_writer_marks_awaiting_approval() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        assert!(store.authorize("s", "write_file", false).is_err());
        assert!(store.is_awaiting_approval("s"));
    }

    #[test]
    fn readonly_tools_do_not_mark_awaiting() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        assert!(store.authorize("s", "read_file", true).is_ok());
        assert!(!store.is_awaiting_approval("s"));
    }

    #[test]
    fn manage_plugin_is_allowed_while_planning() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        assert!(store.authorize("s", "manage_plugin", false).is_ok());
        assert!(!store.is_awaiting_approval("s"));
    }

    #[test]
    fn deactivating_plan_mode_clears_awaiting() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        store.mark_awaiting_approval("s");
        store.set_active("s", false);
        assert!(!store.is_awaiting_approval("s"));
    }

    #[test]
    fn remembers_the_plan_path_per_session() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        store.set_plan_path("s", Some(".anya/plans/20260921-101500-fix.md"));
        assert_eq!(
            store.plan_path("s").as_deref(),
            Some(".anya/plans/20260921-101500-fix.md")
        );
        assert!(store.plan_path("other").is_none());
    }

    #[test]
    fn rewind_and_deactivation_forget_the_plan_path() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        store.set_plan_path("s", Some(".anya/plans/a.md"));
        store.sync_plan_saved("s", false);
        assert!(store.plan_path("s").is_none());

        store.set_plan_path("s", Some(".anya/plans/b.md"));
        store.set_active("s", false);
        assert!(store.plan_path("s").is_none());
    }
}
