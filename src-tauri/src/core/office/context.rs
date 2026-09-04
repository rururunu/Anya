//! Office Context Provider — best-effort Word/Excel/PowerPoint environment snapshot.

use serde::{Deserialize, Serialize};

use crate::core::context::platform::WindowDetector;
use crate::core::runtime::RequestContext;

use super::debug::{record_context_collection, sanitize_summary};
use super::excel::{collect_excel_snapshot, excel_is_available, ExcelError};
use super::powerpoint::{collect_powerpoint_snapshot, powerpoint_is_available, PowerPointError};
use super::word::{collect_word_snapshot, word_is_available, WordError};

pub const OFFICE_SELECTION_MAX_CHARS: usize = 8_000;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OfficeContext {
    pub app: String,
    pub is_foreground: bool,
    pub document_path: Option<String>,
    pub document_name: Option<String>,
    pub selected_text: Option<String>,
    pub selection_start: Option<i32>,
    pub selection_end: Option<i32>,
    pub document_title: Option<String>,
    pub page_count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_sheet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide_index: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide_count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_changes_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_revisions: Option<i32>,
}

pub fn office_app_available(app: &str) -> bool {
    match app {
        "word" => word_is_available(),
        "excel" => excel_is_available(),
        "powerpoint" => powerpoint_is_available(),
        _ => false,
    }
}

/// Best-effort Office context collection. Never panics; returns `None` when unavailable.
pub fn collect_office_context() -> Option<OfficeContext> {
    match collect_office_context_inner() {
        Ok(Some(context)) => {
            let summary = format!(
                "app={} foreground={} doc={} selection={}",
                context.app,
                context.is_foreground,
                context.document_name.as_deref().unwrap_or("<none>"),
                context
                    .selected_text
                    .as_ref()
                    .map(|text| sanitize_summary(text, 80))
                    .unwrap_or_else(|| "none".to_string())
            );
            record_context_collection(true, &summary, None);
            Some(context)
        }
        Ok(None) => {
            record_context_collection(false, "office unavailable", None);
            None
        }
        Err(error) => {
            record_context_collection(false, "office collection failed", Some(&error));
            tracing::warn!(provider = "office", error = %error, "context provider failed");
            None
        }
    }
}

fn collect_office_context_inner() -> Result<Option<OfficeContext>, String> {
    let foreground = foreground_office_app();
    let mut candidates = Vec::new();
    if let Some(app) = foreground {
        candidates.push(app);
    }
    for app in ["word", "excel", "powerpoint"] {
        if !candidates.iter().any(|value| *value == app) {
            candidates.push(app);
        }
    }
    for app in candidates {
        if let Some(context) = try_collect_app(app, foreground == Some(app))? {
            return Ok(Some(context));
        }
    }
    Ok(None)
}

fn try_collect_app(app: &str, is_foreground: bool) -> Result<Option<OfficeContext>, String> {
    match app {
        "word" => match collect_word_snapshot() {
            Ok(snapshot) => Ok(Some(OfficeContext {
                app: "word".to_string(),
                is_foreground,
                document_path: snapshot.document_path,
                document_name: snapshot.document_name,
                selected_text: snapshot
                    .selected_text
                    .as_deref()
                    .map(|text| truncate_chars(text, OFFICE_SELECTION_MAX_CHARS)),
                selection_start: snapshot.selection_start,
                selection_end: snapshot.selection_end,
                document_title: snapshot.document_title,
                page_count: snapshot.page_count,
                track_changes_enabled: snapshot.track_changes_enabled,
                pending_revisions: snapshot.pending_revisions,
                ..OfficeContext::default()
            })),
            Err(WordError::Com(error))
                if matches!(
                    error,
                    super::com::ComError::NotRunning(_, _) | super::com::ComError::ProgId(_, _)
                ) =>
            {
                Ok(None)
            }
            Err(WordError::NoActiveDocument) => Ok(None),
            Err(error) => Err(error.to_string()),
        },
        "excel" => match collect_excel_snapshot() {
            Ok(snapshot) => Ok(Some(OfficeContext {
                app: "excel".to_string(),
                is_foreground,
                document_path: snapshot.workbook_path,
                document_name: snapshot.workbook_name,
                selected_text: snapshot
                    .selected_text
                    .as_deref()
                    .map(|text| truncate_chars(text, OFFICE_SELECTION_MAX_CHARS)),
                active_sheet: snapshot.active_sheet,
                cell_address: snapshot.cell_address,
                ..OfficeContext::default()
            })),
            Err(ExcelError::Com(error))
                if matches!(
                    error,
                    super::com::ComError::NotRunning(_, _) | super::com::ComError::ProgId(_, _)
                ) =>
            {
                Ok(None)
            }
            Err(ExcelError::NoActiveWorkbook) => Ok(None),
            Err(error) => Err(error.to_string()),
        },
        "powerpoint" => match collect_powerpoint_snapshot() {
            Ok(snapshot) => Ok(Some(OfficeContext {
                app: "powerpoint".to_string(),
                is_foreground,
                document_path: snapshot.presentation_path,
                document_name: snapshot.presentation_name,
                selected_text: snapshot
                    .selected_text
                    .as_deref()
                    .map(|text| truncate_chars(text, OFFICE_SELECTION_MAX_CHARS)),
                slide_index: snapshot.slide_index,
                slide_count: snapshot.slide_count,
                ..OfficeContext::default()
            })),
            Err(PowerPointError::Com(error))
                if matches!(
                    error,
                    super::com::ComError::NotRunning(_, _) | super::com::ComError::ProgId(_, _)
                ) =>
            {
                Ok(None)
            }
            Err(PowerPointError::NoActivePresentation) => Ok(None),
            Err(error) => Err(error.to_string()),
        },
        _ => Ok(None),
    }
}

pub fn enrich_request_context(context: &mut RequestContext, office: OfficeContext) {
    if let Some(selection) = office
        .selected_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if context
            .selection
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            context.selection = Some(selection.to_string());
        }
    }
    context.office_context = Some(office);
}

pub fn format_office_context_block(office: &OfficeContext) -> String {
    let app_label = match office.app.as_str() {
        "excel" => "Microsoft Excel",
        "powerpoint" => "Microsoft PowerPoint",
        _ => "Microsoft Word",
    };
    let mut lines = vec![
        format!("Application: {app_label}"),
        format!("Foreground: {}", office.is_foreground),
    ];
    if let Some(name) = non_empty(&office.document_name) {
        lines.push(format!("Document: {name}"));
    }
    if let Some(path) = non_empty(&office.document_path) {
        lines.push(format!("Path: {path}"));
    }
    if let Some(title) = non_empty(&office.document_title) {
        lines.push(format!("Title: {title}"));
    }
    if let Some(pages) = office.page_count {
        lines.push(format!("Pages: {pages}"));
    }
    if let Some(sheet) = non_empty(&office.active_sheet) {
        lines.push(format!("Active Sheet: {sheet}"));
    }
    if let Some(cell) = non_empty(&office.cell_address) {
        lines.push(format!("Cell: {cell}"));
    }
    if let (Some(index), Some(count)) = (office.slide_index, office.slide_count) {
        lines.push(format!("Slide: {index} / {count}"));
    }
    if let Some(enabled) = office.track_changes_enabled {
        lines.push(format!("Track Changes: {enabled}"));
    }
    if let Some(count) = office.pending_revisions {
        lines.push(format!("Pending Revisions: {count}"));
    }
    if let (Some(start), Some(end)) = (office.selection_start, office.selection_end) {
        lines.push(format!("Selection Range: {start}..{end}"));
    }
    if let Some(selection) = non_empty(&office.selected_text) {
        lines.push(format!(
            "Selected Text:\n{}",
            truncate_chars(selection, OFFICE_SELECTION_MAX_CHARS)
        ));
    }
    lines.push(tool_hint_for_app(&office.app));
    format!("[{app_label} Context]\n{}", lines.join("\n\n"))
}

fn tool_hint_for_app(app: &str) -> String {
    match app {
        "excel" => "Prefer excel_* tools (excel_get_selection, excel_get_used_range, excel_set_selection, excel_save_workbook) for Excel tasks.".to_string(),
        "powerpoint" => "Prefer ppt_* tools (ppt_get_selection, ppt_get_slide_text, ppt_replace_selection, ppt_insert_text, ppt_save_presentation) for PowerPoint tasks.".to_string(),
        _ => "Prefer word_* tools (word_get_document_content, word_get_selection, word_replace_selection, word_insert_text, word_save_document) for live Word editing. For new/edit .docx files use #skill:docx; for md↔docx conversion use #skill:pandoc; for simple python-docx use generate_word.".to_string(),
    }
}

fn foreground_office_app() -> Option<&'static str> {
    let window = WindowDetector::detect().ok()?;
    if window.process_name.eq_ignore_ascii_case("WINWORD.EXE") {
        Some("word")
    } else if window.process_name.eq_ignore_ascii_case("EXCEL.EXE") {
        Some("excel")
    } else if window.process_name.eq_ignore_ascii_case("POWERPNT.EXE") {
        Some("powerpoint")
    } else {
        None
    }
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_none_without_office() {
        if word_is_available() || excel_is_available() || powerpoint_is_available() {
            return;
        }
        assert!(collect_office_context().is_none());
    }

    #[test]
    fn merge_into_request_context() {
        let office = OfficeContext {
            app: "word".to_string(),
            is_foreground: true,
            document_name: Some("Draft.docx".to_string()),
            selected_text: Some("hello".to_string()),
            ..OfficeContext::default()
        };
        let mut request = RequestContext::default();
        enrich_request_context(&mut request, office.clone());
        assert_eq!(request.office_context.as_ref(), Some(&office));
        assert_eq!(request.selection.as_deref(), Some("hello"));
    }

    #[test]
    fn merge_does_not_override_existing_selection() {
        let office = OfficeContext {
            selected_text: Some("from word".to_string()),
            ..OfficeContext::default()
        };
        let mut request = RequestContext {
            selection: Some("from ide".to_string()),
            ..RequestContext::default()
        };
        enrich_request_context(&mut request, office);
        assert_eq!(request.selection.as_deref(), Some("from ide"));
    }

    #[test]
    fn format_block_includes_tool_hint() {
        let block = format_office_context_block(&OfficeContext {
            app: "word".to_string(),
            is_foreground: true,
            document_name: Some("Notes.docx".to_string()),
            ..OfficeContext::default()
        });
        assert!(block.contains("[Microsoft Word Context]"));
        assert!(block.contains("word_get_document_content"));
    }

    #[test]
    fn excel_hint_mentions_excel_tools() {
        let block = format_office_context_block(&OfficeContext {
            app: "excel".to_string(),
            ..OfficeContext::default()
        });
        assert!(block.contains("excel_get_selection"));
    }
}
