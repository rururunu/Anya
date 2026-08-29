use crate::core::runtime::stream::ToolCallPayload;
use crate::core::runtime::{ToolActivity, WorkTimelineItem};

/// Serialize tool activities to JSON, truncating oversized fields for storage.
pub(crate) fn serialize_tool_activities(activities: Option<&Vec<ToolActivity>>) -> Option<String> {
    let activities = activities?;
    let capped: Vec<ToolActivity> = activities
        .iter()
        .map(|activity| {
            let mut activity = activity.clone();
            if let Some(result) = activity.result.as_mut() {
                *result = crate::core::chat::limits::truncate_tool_output(
                    result,
                    crate::core::chat::limits::STORED_TOOL_RESULT_MAX_CHARS,
                );
            }
            if let Some(preview) = activity.preview.as_mut() {
                if let Some(old) = preview.old_text.as_mut() {
                    *old = crate::core::chat::limits::truncate_chars(
                        old,
                        crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                    );
                }
                if let Some(new) = preview.new_text.as_mut() {
                    *new = crate::core::chat::limits::truncate_chars(
                        new,
                        crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                    );
                }
                preview.unified_diff = crate::core::chat::limits::truncate_chars(
                    &preview.unified_diff,
                    crate::core::chat::limits::STORED_PREVIEW_TEXT_MAX_CHARS,
                );
            }
            activity
        })
        .collect();
    serde_json::to_string(&capped).ok()
}

/// Serialize work timeline items to JSON, truncating oversized text for storage.
pub(crate) fn serialize_work_timeline(timeline: Option<&Vec<WorkTimelineItem>>) -> Option<String> {
    let timeline = timeline?;
    let capped: Vec<WorkTimelineItem> = timeline
        .iter()
        .map(|item| match item {
            WorkTimelineItem::Reasoning { id, content } => WorkTimelineItem::Reasoning {
                id: id.clone(),
                content: crate::core::chat::limits::truncate_chars(
                    content,
                    crate::core::chat::limits::STORED_TIMELINE_ITEM_MAX_CHARS,
                ),
            },
            WorkTimelineItem::Content { id, content } => WorkTimelineItem::Content {
                id: id.clone(),
                content: crate::core::chat::limits::truncate_chars(
                    content,
                    crate::core::chat::limits::STORED_TIMELINE_ITEM_MAX_CHARS,
                ),
            },
            other => other.clone(),
        })
        .collect();
    serde_json::to_string(&capped).ok()
}

/// Serialize tool call payloads to JSON, truncating oversized arguments for storage.
pub(crate) fn serialize_tool_calls(calls: Option<&Vec<ToolCallPayload>>) -> Option<String> {
    let calls = calls?;
    let capped: Vec<ToolCallPayload> = calls
        .iter()
        .map(|call| {
            let mut call = call.clone();
            call.arguments = crate::core::chat::limits::truncate_chars(
                &call.arguments,
                crate::core::chat::limits::STORED_TOOL_CALL_ARGS_MAX_CHARS,
            );
            call
        })
        .collect();
    serde_json::to_string(&capped).ok()
}
