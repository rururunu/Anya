use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::core::tools::error::ToolError;
use crate::models::settings::ChatMode;

/// Hard plan-mode gate: when active, reject non-readonly mutating tools.
pub const PLAN_GATE_BLOCKED: &str =
    "plan mode is active: writer tools are blocked until the user approves the plan";

/// User-visible stop text when a writer hits the gate. Also used to detect
/// that the turn ended because of the gate (even if no checklist exists).
pub const PLAN_GATE_STOP_HINT: &str = "计划尚未批准";

pub struct PlanModeStore {
    active_sessions: Mutex<HashSet<String>>,
    /// Sessions that actually have something to approve: a checklist, or a
    /// writer that hit the gate. Plan mode alone is not enough.
    awaiting_approval: Mutex<HashSet<String>>,
}

impl PlanModeStore {
    pub fn new() -> Self {
        Self {
            active_sessions: Mutex::new(HashSet::new()),
            awaiting_approval: Mutex::new(HashSet::new()),
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
        "update_tasks"
            | "ask_user"
            | "share_to_companion"
            | "share_preview_url"
            | "complete_plan_step"
            | "todo_write"
    )
}

pub fn shared_plan_mode_store() -> &'static PlanModeStore {
    static STORE: OnceLock<PlanModeStore> = OnceLock::new();
    STORE.get_or_init(PlanModeStore::new)
}

/// Decide whether a new Agent turn should automatically enter plan mode.
///
/// Ask mode never plans. Explicit skip/force phrases win. Otherwise a light
/// complexity score decides — users should not have to flip a mode switch for
/// multi-step work.
pub fn should_auto_plan(message: &str, chat_mode: ChatMode) -> bool {
    // Only Agent auto-enters plan. Ask never plans; Plan is already planning.
    if chat_mode != ChatMode::Agent {
        return false;
    }
    let text = message.trim();
    if text.is_empty() {
        return false;
    }
    let lower = text.to_lowercase();

    if has_skip_plan_intent(&lower) {
        return false;
    }
    if has_force_plan_intent(text, &lower) {
        return true;
    }

    let mut score = 0u32;
    let char_len = text.chars().count();
    if char_len >= 120 {
        score += 1;
    }
    if char_len >= 280 {
        score += 1;
    }

    let list_items = count_list_items(text);
    if list_items >= 3 {
        score += 2;
    } else if list_items >= 2 {
        score += 1;
    }

    let path_hits = count_path_like_mentions(text);
    if path_hits >= 3 {
        score += 2;
    } else if path_hits >= 2 {
        score += 1;
    }

    let keyword_hits = COMPLEXITY_KEYWORDS
        .iter()
        .filter(|keyword| text.contains(*keyword) || lower.contains(&keyword.to_lowercase()))
        .count();
    if keyword_hits >= 2 {
        score += 2;
    } else if keyword_hits == 1 {
        score += 1;
    }

    if count_action_connectors(text, &lower) >= 1 {
        score += 1;
    }

    score >= 2
}

const COMPLEXITY_KEYWORDS: &[&str] = &[
    "实现",
    "重构",
    "架构",
    "完整",
    "迁移",
    "接入",
    "系统设计",
    "端到端",
    "分步",
    "设计并",
    "implement",
    "refactor",
    "migrate",
    "architecture",
    "end-to-end",
    "from scratch",
    "multi-step",
    "roll out",
    "wire up",
];

fn has_skip_plan_intent(lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "直接做",
        "别规划",
        "不要规划",
        "不用规划",
        "跳过计划",
        "skip plan",
        "don't plan",
        "do not plan",
        "no plan",
        "just do it",
        "directly implement",
    ];
    PHRASES.iter().any(|phrase| lower.contains(phrase))
}

fn has_force_plan_intent(text: &str, lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "先做计划",
        "先规划",
        "先出方案",
        "先出计划",
        "plan first",
        "make a plan",
        "write a plan",
        "propose a plan",
    ];
    PHRASES
        .iter()
        .any(|phrase| text.contains(phrase) || lower.contains(phrase))
}

fn count_list_items(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            starts_with_ordered_marker(line)
                || line.starts_with("- ")
                || line.starts_with("* ")
                || line.starts_with("• ")
        })
        .count()
}

fn starts_with_ordered_marker(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_digit() {
        return false;
    }
    let mut saw_digit = true;
    for ch in chars {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        return saw_digit && matches!(ch, '.' | ')' | '、' | '．');
    }
    false
}

fn count_path_like_mentions(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| {
            let cleaned = token.trim_matches(|c: char| {
                matches!(c, '`' | '"' | '\'' | ',' | '.' | ';' | ':' | ')' | '(')
            });
            cleaned.contains('/')
                || cleaned.contains('\\')
                || cleaned.contains(".rs")
                || cleaned.contains(".ts")
                || cleaned.contains(".tsx")
                || cleaned.contains(".vue")
                || cleaned.contains(".py")
                || cleaned.contains(".md")
                || cleaned.contains(".json")
        })
        .count()
}

fn count_action_connectors(text: &str, lower: &str) -> usize {
    const CONNECTORS: &[&str] = &[
        "然后",
        "并且",
        "同时",
        "接着",
        "再",
        "以及",
        " and then ",
        " then ",
        " also ",
        " as well as ",
        " plus ",
    ];
    CONNECTORS
        .iter()
        .filter(|connector| text.contains(*connector) || lower.contains(*connector))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_mode_never_auto_plans() {
        assert!(!should_auto_plan(
            "实现一个完整的暗色模式开关并跑类型检查",
            ChatMode::Ask
        ));
    }

    #[test]
    fn short_trivial_requests_skip_plan() {
        assert!(!should_auto_plan("把 typo 改了", ChatMode::Agent));
    }

    #[test]
    fn multi_step_chinese_request_auto_plans() {
        assert!(should_auto_plan(
            "给设置页实现暗色模式：加状态、改 UI、接入主题，然后跑类型检查。",
            ChatMode::Agent
        ));
    }

    #[test]
    fn numbered_list_auto_plans() {
        let message = "Please do the following:\n1. add auth\n2. wire routes\n3. add tests";
        assert!(should_auto_plan(message, ChatMode::Agent));
    }

    #[test]
    fn skip_phrase_wins() {
        assert!(!should_auto_plan(
            "实现完整的暗色模式开关，直接做，别规划",
            ChatMode::Agent
        ));
    }

    #[test]
    fn force_phrase_wins_even_if_short() {
        assert!(should_auto_plan("先规划一下这个改动", ChatMode::Agent));
    }

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
    fn deactivating_plan_mode_clears_awaiting() {
        let store = PlanModeStore::new();
        store.set_active("s", true);
        store.mark_awaiting_approval("s");
        store.set_active("s", false);
        assert!(!store.is_awaiting_approval("s"));
    }
}
