//! Shared in-session checklist so `update_tasks` and chat rewind the same list.

use std::sync::{Arc, Mutex, OnceLock};

use crate::core::runtime::ChatMessage;
use crate::core::tools::context::TaskItem;

const TASK_TOOLS: &[&str] = &["update_tasks", "todo_write"];

/// Process-wide checklist mutated by builtin task tools and restored on rewind.
pub fn shared_task_list() -> Arc<Mutex<Vec<TaskItem>>> {
    static STORE: OnceLock<Arc<Mutex<Vec<TaskItem>>>> = OnceLock::new();
    Arc::clone(STORE.get_or_init(|| Arc::new(Mutex::new(Vec::new()))))
}

/// Replace the shared checklist after history is truncated.
pub fn replace_shared_tasks(tasks: Vec<TaskItem>) {
    if let Ok(mut guard) = shared_task_list().lock() {
        *guard = tasks;
    }
}

/// Last `update_tasks` / `todo_write` payload still present in remaining history.
pub fn last_tasks_from_messages(messages: &[ChatMessage]) -> Vec<TaskItem> {
    for message in messages.iter().rev() {
        let Some(activities) = &message.tool_activities else {
            continue;
        };
        for activity in activities.iter().rev() {
            if !TASK_TOOLS.contains(&activity.tool_name.as_str()) {
                continue;
            }
            let Some(args) = &activity.arguments else {
                continue;
            };
            let Some(raw) = args.get("tasks") else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<Vec<TaskItem>>(raw.clone()) else {
                continue;
            };
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

/// True when remaining history still contains a `save_plan` call.
pub fn history_has_save_plan(messages: &[ChatMessage]) -> bool {
    messages.iter().any(|message| {
        message.tool_activities.as_ref().is_some_and(|activities| {
            activities
                .iter()
                .any(|activity| activity.tool_name == "save_plan")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{MessageStatus, Role, ToolActivity};

    fn activity(tool_name: &str, args: serde_json::Value) -> ToolActivity {
        ToolActivity {
            id: "a".into(),
            subagent_id: None,
            parent_activity_id: None,
            tool_name: tool_name.into(),
            title: tool_name.into(),
            kind: "other".into(),
            detail: None,
            arguments: Some(args),
            result: None,
            preview: None,
            success: true,
            status: "done".into(),
        }
    }

    fn message(activities: Vec<ToolActivity>) -> ChatMessage {
        ChatMessage {
            id: "m".into(),
            session_id: "s".into(),
            role: Role::Assistant,
            content: String::new(),
            reasoning: None,
            work_timeline: None,
            tool_activities: Some(activities),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        }
    }

    fn task(content: &str, status: &str) -> serde_json::Value {
        serde_json::json!({ "content": content, "status": status })
    }

    #[test]
    fn last_tasks_prefers_latest_remaining_update() {
        let earlier = message(vec![activity(
            "update_tasks",
            serde_json::json!({ "tasks": [task("one", "pending")] }),
        )]);
        let later = message(vec![activity(
            "update_tasks",
            serde_json::json!({ "tasks": [task("one", "completed")] }),
        )]);
        let from_full = last_tasks_from_messages(&[earlier.clone(), later]);
        assert_eq!(from_full[0].status, "completed");
        let from_rewound = last_tasks_from_messages(&[earlier]);
        assert_eq!(from_rewound[0].status, "pending");
    }

    #[test]
    fn last_tasks_empty_when_checklist_was_truncated() {
        let user = ChatMessage {
            id: "u".into(),
            session_id: "s".into(),
            role: Role::User,
            content: "hi".into(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        };
        assert!(last_tasks_from_messages(&[user]).is_empty());
    }
}
