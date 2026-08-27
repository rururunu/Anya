use std::collections::HashSet;

use crate::core::runtime::{ChatMessage, ToolActivity, WorkTimelineItem};

use super::helpers::refresh_message_token_cache;
use super::ConversationManager;

/// Which run of text a streamed chunk belongs to, so it can be merged into the
/// trailing `WorkTimelineItem` of the same kind instead of starting a new one
/// for every delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineTextKind {
    Reasoning,
    Content,
}

impl ConversationManager {
    /// Append a streamed chunk to the message's chronological work timeline,
    /// merging into the trailing entry when it is the same kind so reasoning
    /// and reply text each collapse into one run instead of one item per
    /// delta. This only updates the in-memory copy — like the raw
    /// `content`/`reasoning` accumulators, the timeline is flushed to SQLite
    /// by the terminal `update_message` call once the turn settles, and
    /// recovered from the journal on crash in the meantime.
    pub fn append_work_timeline_text(
        &self,
        session_id: &str,
        message_id: &str,
        kind: TimelineTextKind,
        chunk: &str,
    ) {
        if chunk.is_empty() {
            return;
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(message) = sessions
            .get_mut(session_id)
            .and_then(|messages| messages.iter_mut().find(|item| item.id == message_id))
        else {
            return;
        };
        let timeline = message.work_timeline.get_or_insert_with(Vec::new);
        let next_index = timeline.len();
        if let Some(last) = timeline.last_mut() {
            let merged = match (last, kind) {
                (WorkTimelineItem::Reasoning { content, .. }, TimelineTextKind::Reasoning) => {
                    content.push_str(chunk);
                    true
                }
                (WorkTimelineItem::Content { content, .. }, TimelineTextKind::Content) => {
                    content.push_str(chunk);
                    true
                }
                _ => false,
            };
            if merged {
                return;
            }
        }
        let item = match kind {
            TimelineTextKind::Reasoning => WorkTimelineItem::Reasoning {
                id: format!("{message_id}-reasoning-{next_index}"),
                content: chunk.to_string(),
            },
            TimelineTextKind::Content => WorkTimelineItem::Content {
                id: format!("{message_id}-content-{next_index}"),
                content: chunk.to_string(),
            },
        };
        timeline.push(item);
    }

    pub fn work_timeline_len(&self, session_id: &str, message_id: &str) -> usize {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(session_id)?
                    .iter()
                    .find(|item| item.id == message_id)?
                    .work_timeline
                    .as_ref()
                    .map(|timeline| timeline.len())
            })
            .unwrap_or(0)
    }

    /// Keep timeline items produced before `keep_len` (completed prior rounds)
    /// and drop anything streamed during a failed/retried provider attempt.
    pub fn truncate_work_timeline(&self, session_id: &str, message_id: &str, keep_len: usize) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(message) = sessions
                .get_mut(session_id)
                .and_then(|messages| messages.iter_mut().find(|item| item.id == message_id))
            {
                if keep_len == 0 {
                    message.work_timeline = None;
                } else if let Some(timeline) = message.work_timeline.as_mut() {
                    if timeline.len() > keep_len {
                        timeline.truncate(keep_len);
                    }
                }
                let kept_tool_ids = message
                    .work_timeline
                    .as_ref()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| match item {
                                WorkTimelineItem::Tool {
                                    tool_activity_id, ..
                                } => Some(tool_activity_id.as_str()),
                                _ => None,
                            })
                            .collect::<HashSet<_>>()
                    })
                    .unwrap_or_default();
                if let Some(activities) = message.tool_activities.as_mut() {
                    activities.retain(|activity| {
                        kept_tool_ids.contains(activity.id.as_str())
                            || activity
                                .parent_activity_id
                                .as_deref()
                                .is_some_and(|parent| kept_tool_ids.contains(parent))
                    });
                    if activities.is_empty() {
                        message.tool_activities = None;
                    }
                }
            }
        }
    }

    pub fn upsert_tool_activity(
        &self,
        session_id: &str,
        message_id: &str,
        activity: ToolActivity,
    ) -> Option<ChatMessage> {
        self.ensure_session_loaded(session_id);
        let should_persist = activity.status != "running";
        let mut sessions = self.sessions.lock().ok()?;
        let message = sessions
            .get_mut(session_id)?
            .iter_mut()
            .find(|item| item.id == message_id)?;
        let activities = message.tool_activities.get_or_insert_with(Vec::new);
        if let Some(existing) = activities.iter_mut().find(|item| item.id == activity.id) {
            *existing = activity;
        } else {
            // Anchor the tool card at the point it actually started, right
            // after whatever narration preceded it, instead of grouping all
            // tool activity separately from the text that led up to it.
            let already_on_timeline = message.work_timeline.as_ref().is_some_and(|items| {
                items.iter().any(|item| {
                    matches!(
                        item,
                        WorkTimelineItem::Tool { tool_activity_id, .. }
                            if tool_activity_id == &activity.id
                    )
                })
            });
            let timeline_entry = WorkTimelineItem::Tool {
                id: format!("{}-tool-{}", message_id, activity.id),
                tool_activity_id: activity.id.clone(),
            };
            activities.push(activity);
            if !already_on_timeline {
                message
                    .work_timeline
                    .get_or_insert_with(Vec::new)
                    .push(timeline_entry);
            }
        }
        if !should_persist {
            message.estimated_tokens = None;
            return Some(message.clone());
        }
        refresh_message_token_cache(message);
        let updated = message.clone();
        let pool = self.db_pool.clone();
        let msg_to_save = updated.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::super::db::save_message_tool_fields(&pool, &msg_to_save).await {
                eprintln!("Failed to save tool activity to SQLite: {}", e);
            }
        });
        Some(updated)
    }
}
