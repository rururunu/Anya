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
}

impl PlanModeStore {
    pub fn new() -> Self {
        Self {
            active_sessions: Mutex::new(HashSet::new()),
            awaiting_approval: Mutex::new(HashSet::new()),
            plans_saved: Mutex::new(HashSet::new()),
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

/// Hint whether an Agent turn should *offer* Plan mode via `request_plan_mode`.
/// Never flips the writer gate by itself. Ask mode never hints. Explicit
/// skip/force phrases win. Otherwise a light complexity score decides.
pub fn should_auto_plan(message: &str, chat_mode: ChatMode) -> bool {
    // Only Agent is hinted to request Plan. Ask never plans; Plan is already on.
    if chat_mode != ChatMode::Agent {
        return false;
    }
    let owned = strip_attached_file_bodies(message);
    let text = owned.trim();
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

    let list_items = count_list_items(text);
    if list_items >= 3 {
        score += 3;
    }

    let enum_actions = count_enumeration_actions(text);
    if enum_actions >= 3 {
        score += 3;
    }

    if has_sequential_phase_markers(text, &lower) {
        score += 3;
    }

    let keyword_hits = COMPLEXITY_KEYWORDS
        .iter()
        .filter(|keyword| text.contains(*keyword) || lower.contains(&keyword.to_lowercase()))
        .count();
    if keyword_hits >= 2 {
        score += 3;
    } else if keyword_hits == 1 {
        score += 2;
    }

    let path_hits = count_path_like_mentions(text);
    if path_hits >= 5 {
        score += 2;
    } else if path_hits >= 3 {
        score += 1;
    }

    if text.chars().count() >= 350 {
        score += 1;
    }

    score >= 3
}

/// Attached paste/file bodies must not trip auto-plan: a 40-line code dump
/// has lists and path tokens even when the user only asked a short question.
fn strip_attached_file_bodies(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find("<peek-attached-file") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(tag_end) = after.find('>') else {
            out.push_str(after);
            return out;
        };
        let opening = &after[..=tag_end];
        if opening.ends_with("/>") {
            rest = &after[tag_end + 1..];
            continue;
        }
        if let Some(close) = after.find("</peek-attached-file>") {
            rest = &after[close + "</peek-attached-file>".len()..];
            continue;
        }
        out.push_str(after);
        return out;
    }
    out.push_str(rest);
    out
}

const COMPLEXITY_KEYWORDS: &[&str] = &[
    "系统设计",
    "架构重构",
    "架构设计",
    "大规模重构",
    "端到端架构",
    "从零搭建",
    "分阶段实施",
    "分步执行",
    "全流程设计",
    "system design",
    "architecture redesign",
    "architecture refactor",
    "large-scale refactor",
    "end-to-end architecture",
    "from scratch architecture",
    "multi-phase rollout",
];

fn has_skip_plan_intent(lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "直接做",
        "别规划",
        "不要规划",
        "不用规划",
        "跳过计划",
        "不用出方案",
        "不用计划",
        "别出方案",
        "直接改",
        "直接修复",
        "直接写",
        "直接实现",
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
        "先给出计划",
        "请给出计划",
        "请出计划",
        "做个计划",
        "出个计划",
        "做个方案",
        "出个方案",
        "制定计划",
        "制定方案",
        "列出计划再",
        "先给计划",
        "先给方案",
        "先设计方案",
        "列出步骤再做",
        "先列出步骤",
        "先列出计划",
        "给我出个计划",
        "给我做个计划",
        "plan first",
        "make a plan",
        "write a plan",
        "propose a plan",
        "create a plan",
        "generate a plan",
        "give me a plan",
        "draft a plan",
    ];
    PHRASES
        .iter()
        .any(|phrase| text.contains(phrase) || lower.contains(phrase))
}

fn count_enumeration_actions(text: &str) -> usize {
    if !text.contains('、') {
        return 0;
    }
    const ACTION_PREFIXES: &[&str] = &[
        "加", "改", "接入", "建", "删", "查", "跑", "写", "设", "调", "迁", "配", "验", "测",
        "修复", "实现", "重构", "添加", "优化", "更新",
    ];
    let count = text
        .split('、')
        .filter(|part| {
            let trimmed = part.trim();
            ACTION_PREFIXES
                .iter()
                .any(|prefix| trimmed.contains(prefix))
        })
        .count();
    if count >= 3 {
        count
    } else {
        0
    }
}

fn has_sequential_phase_markers(text: &str, lower: &str) -> bool {
    const MARKERS: &[&str] = &[
        "第一步",
        "第二步",
        "第1步",
        "第2步",
        "步骤一",
        "步骤二",
        "阶段一",
        "阶段二",
        "step 1",
        "step 2",
        "phase 1",
        "phase 2",
    ];
    MARKERS
        .iter()
        .any(|marker| text.contains(marker) || lower.contains(marker))
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
    fn normal_tasks_do_not_auto_plan() {
        assert!(!should_auto_plan(
            "优化一个问题，就是它经常会莫名的给出计划，有时候甚至计划是空的。这很严重",
            ChatMode::Agent
        ));
        assert!(!should_auto_plan(
            "帮我实现一下登录接口，然后再在页面上测试一下效果。",
            ChatMode::Agent
        ));
        assert!(!should_auto_plan(
            "把这个接口接入，在 main.ts 里面测试一下，并且看一下完整的报错信息。",
            ChatMode::Agent
        ));
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
    fn pasted_attachment_body_does_not_auto_plan() {
        let body: String = (1..=45)
            .map(|i| format!("{i}. src/components/foo.ts implements bar.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            should_auto_plan(&body, ChatMode::Agent),
            "numbered code dump itself is multi-step"
        );
        let message = format!(
            "看看是怎么回事\n\n<peek-attached-file name=\"pasted-45lines.txt\" path=\"tmp\">\n{body}\n</peek-attached-file>"
        );
        assert!(!should_auto_plan(&message, ChatMode::Agent));
    }

    #[test]
    fn force_plan_still_wins_with_an_attachment() {
        let message = concat!(
            "先规划一下这个改动\n\n",
            "<peek-attached-file name=\"notes.txt\" path=\"tmp\">\n",
            "1. add auth\n2. wire routes\n3. add tests\n",
            "</peek-attached-file>"
        );
        assert!(should_auto_plan(message, ChatMode::Agent));
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
}
