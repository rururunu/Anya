//! Agent tools for Microsoft Office automation.

use serde_json::{json, Value};

use crate::core::chat::limits::TOOL_OUTPUT_MAX_CHARS;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;

use super::debug::{emit_tool_debug, sanitize_summary};
use super::excel::{
    get_selection_text as excel_get_selection, get_used_range_text, save_active_workbook,
    set_selection_value, ExcelError,
};
use super::powerpoint::{
    get_selection_text as ppt_get_selection, get_slide_text, insert_text_at_cursor as ppt_insert,
    replace_selection_text as ppt_replace, save_active_presentation, PowerPointError,
};
use super::word::{
    accept_all_revisions, add_comment, apply_font_to_selection_or_range, get_document_paragraphs,
    get_document_range, get_document_text, get_selection_text as word_get_selection,
    insert_table_at_selection, insert_text_at_cursor, list_comments, reject_all_revisions,
    replace_selection_or_range, save_active_document, WordError,
};

pub fn register_tools(registry: &mut ToolRegistry) {
    register_word_tools(registry);
    register_excel_tools(registry);
    register_ppt_tools(registry);
}

fn register_word_tools(registry: &mut ToolRegistry) {
    macro_rules! reg {
        ($tool:expr) => {
            registry.register(std::sync::Arc::new($tool));
        };
    }
    reg!(WordGetDocumentContentTool);
    reg!(WordGetDocumentRangeTool);
    reg!(WordGetDocumentParagraphsTool);
    reg!(WordGetSelectionTool);
    reg!(WordReplaceSelectionTool);
    reg!(WordInsertTextTool);
    reg!(WordInsertTableTool);
    reg!(WordApplyFontTool);
    reg!(WordListCommentsTool);
    reg!(WordAddCommentTool);
    reg!(WordAcceptRevisionsTool);
    reg!(WordRejectRevisionsTool);
    reg!(WordSaveDocumentTool);
}

fn register_excel_tools(registry: &mut ToolRegistry) {
    registry.register(std::sync::Arc::new(ExcelGetSelectionTool));
    registry.register(std::sync::Arc::new(ExcelGetUsedRangeTool));
    registry.register(std::sync::Arc::new(ExcelSetSelectionTool));
    registry.register(std::sync::Arc::new(ExcelSaveWorkbookTool));
}

fn register_ppt_tools(registry: &mut ToolRegistry) {
    registry.register(std::sync::Arc::new(PptGetSelectionTool));
    registry.register(std::sync::Arc::new(PptGetSlideTextTool));
    registry.register(std::sync::Arc::new(PptReplaceSelectionTool));
    registry.register(std::sync::Arc::new(PptInsertTextTool));
    registry.register(std::sync::Arc::new(PptSavePresentationTool));
}

struct WordGetDocumentContentTool;
impl Tool for WordGetDocumentContentTool {
    fn name(&self) -> &str {
        "word_get_document_content"
    }
    fn description(&self) -> &str {
        "Read the active Microsoft Word document. For large documents prefer start_char/end_char or paragraph_start/paragraph_count to read in chunks."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "max_chars": { "type": "integer" },
                "start_char": { "type": "integer" },
                "end_char": { "type": "integer" },
                "paragraph_start": { "type": "integer" },
                "paragraph_count": { "type": "integer" }
            }
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let max_chars = capped_chars(args["max_chars"].as_u64());
        let result = if let (Some(start), Some(count)) = (
            args["paragraph_start"].as_i64().map(|v| v as i32),
            args["paragraph_count"].as_i64().map(|v| v as i32),
        ) {
            get_document_paragraphs(start, count, max_chars)
        } else {
            let start = args["start_char"].as_i64().map(|v| v as i32);
            let end = args["end_char"].as_i64().map(|v| v as i32);
            if start.is_some() || end.is_some() {
                get_document_range(start, end, max_chars)
            } else {
                get_document_text(max_chars)
            }
        };
        run_word(ctx, self.name(), result)
    }
}

struct WordGetDocumentRangeTool;
impl Tool for WordGetDocumentRangeTool {
    fn name(&self) -> &str {
        "word_get_document_range"
    }
    fn description(&self) -> &str {
        "Read a character range from the active Word document via COM Range(start,end)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "start_char": { "type": "integer" },
                "end_char": { "type": "integer" },
                "max_chars": { "type": "integer" }
            },
            "required": ["start_char", "end_char"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let start = args["start_char"].as_i64().unwrap_or(0) as i32;
        let end = args["end_char"].as_i64().unwrap_or(0) as i32;
        run_word(
            ctx,
            self.name(),
            get_document_range(
                Some(start),
                Some(end),
                capped_chars(args["max_chars"].as_u64()),
            ),
        )
    }
}

struct WordGetDocumentParagraphsTool;
impl Tool for WordGetDocumentParagraphsTool {
    fn name(&self) -> &str {
        "word_get_document_paragraphs"
    }
    fn description(&self) -> &str {
        "Read a paragraph slice from the active Word document (1-based paragraph index)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "paragraph_start": { "type": "integer" },
                "paragraph_count": { "type": "integer" },
                "max_chars": { "type": "integer" }
            },
            "required": ["paragraph_start", "paragraph_count"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let start = args["paragraph_start"].as_i64().unwrap_or(1) as i32;
        let count = args["paragraph_count"].as_i64().unwrap_or(1) as i32;
        run_word(
            ctx,
            self.name(),
            get_document_paragraphs(start, count, capped_chars(args["max_chars"].as_u64())),
        )
    }
}

struct WordGetSelectionTool;
impl Tool for WordGetSelectionTool {
    fn name(&self) -> &str {
        "word_get_selection"
    }
    fn description(&self) -> &str {
        "Read the current selection in Microsoft Word."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_word(ctx, self.name(), word_get_selection())
    }
}

struct WordReplaceSelectionTool;
impl Tool for WordReplaceSelectionTool {
    fn name(&self) -> &str {
        "word_replace_selection"
    }
    fn description(&self) -> &str {
        "Replace the current Microsoft Word selection with new text. If Anya stole focus and the selection collapsed, pass start/end from the earlier Office Context (selectionStart/selectionEnd)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "start": { "type": "integer", "description": "Optional Range.Start when current selection is empty" },
                "end": { "type": "integer", "description": "Optional Range.End when current selection is empty" }
            },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let start = args["start"].as_i64().map(|v| v as i32);
        let end = args["end"].as_i64().map(|v| v as i32);
        run_word(
            ctx,
            self.name(),
            replace_selection_or_range(args["text"].as_str().unwrap_or(""), start, end),
        )
    }
}

struct WordInsertTextTool;
impl Tool for WordInsertTextTool {
    fn name(&self) -> &str {
        "word_insert_text"
    }
    fn description(&self) -> &str {
        "Insert text at the current Microsoft Word cursor position."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_word(
            ctx,
            self.name(),
            insert_text_at_cursor(args["text"].as_str().unwrap_or("")),
        )
    }
}

struct WordInsertTableTool;
impl Tool for WordInsertTableTool {
    fn name(&self) -> &str {
        "word_insert_table"
    }
    fn description(&self) -> &str {
        "Insert a real Word table at the current selection. Prefer python-docx (generate_word) for building a whole document; use this for small live edits. `cells` is a row-major flat array of length rows*cols."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "rows": { "type": "integer" },
                "cols": { "type": "integer" },
                "cells": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Row-major cell texts; length must equal rows*cols"
                }
            },
            "required": ["rows", "cols", "cells"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let rows = args["rows"].as_i64().unwrap_or(0) as i32;
        let cols = args["cols"].as_i64().unwrap_or(0) as i32;
        let cells: Vec<String> = args["cells"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();
        run_word(
            ctx,
            self.name(),
            insert_table_at_selection(rows, cols, &cells),
        )
    }
}

struct WordApplyFontTool;
impl Tool for WordApplyFontTool {
    fn name(&self) -> &str {
        "word_apply_font"
    }
    fn description(&self) -> &str {
        "Normalize font name and size on the current Word selection (or start/end range). Use after word_replace_selection when mixed run sizes appear."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "font_name": { "type": "string", "description": "e.g. 仿宋 / 宋体 / 黑体" },
                "size_pt": { "type": "number", "description": "Font size in points, e.g. 12" },
                "start": { "type": "integer" },
                "end": { "type": "integer" }
            },
            "required": ["font_name", "size_pt"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let start = args["start"].as_i64().map(|v| v as i32);
        let end = args["end"].as_i64().map(|v| v as i32);
        let size = args["size_pt"].as_f64().unwrap_or(12.0);
        run_word(
            ctx,
            self.name(),
            apply_font_to_selection_or_range(
                args["font_name"].as_str().unwrap_or("仿宋"),
                size,
                start,
                end,
            ),
        )
    }
}

struct WordListCommentsTool;
impl Tool for WordListCommentsTool {
    fn name(&self) -> &str {
        "word_list_comments"
    }
    fn description(&self) -> &str {
        "List comments in the active Word document (author + preview, no full body dump)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "max_items": { "type": "integer" } }
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let max_items = args["max_items"].as_u64().unwrap_or(20) as usize;
        run_word(ctx, self.name(), list_comments(max_items))
    }
}

struct WordAddCommentTool;
impl Tool for WordAddCommentTool {
    fn name(&self) -> &str {
        "word_add_comment"
    }
    fn description(&self) -> &str {
        "Add a comment to the current Word selection (or whole document when use_selection=false)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "use_selection": { "type": "boolean" }
            },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let use_selection = args["use_selection"].as_bool().unwrap_or(true);
        run_word(
            ctx,
            self.name(),
            add_comment(args["text"].as_str().unwrap_or(""), use_selection),
        )
    }
}

struct WordAcceptRevisionsTool;
impl Tool for WordAcceptRevisionsTool {
    fn name(&self) -> &str {
        "word_accept_all_revisions"
    }
    fn description(&self) -> &str {
        "Accept all tracked revisions in the active Word document."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_word(ctx, self.name(), accept_all_revisions())
    }
}

struct WordRejectRevisionsTool;
impl Tool for WordRejectRevisionsTool {
    fn name(&self) -> &str {
        "word_reject_all_revisions"
    }
    fn description(&self) -> &str {
        "Reject all tracked revisions in the active Word document."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_word(ctx, self.name(), reject_all_revisions())
    }
}

struct WordSaveDocumentTool;
impl Tool for WordSaveDocumentTool {
    fn name(&self) -> &str {
        "word_save_document"
    }
    fn description(&self) -> &str {
        "Save the active Microsoft Word document."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_word(ctx, self.name(), save_active_document())
    }
}

struct ExcelGetSelectionTool;
impl Tool for ExcelGetSelectionTool {
    fn name(&self) -> &str {
        "excel_get_selection"
    }
    fn description(&self) -> &str {
        "Read the current Excel selection."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_excel(ctx, self.name(), excel_get_selection())
    }
}

struct ExcelGetUsedRangeTool;
impl Tool for ExcelGetUsedRangeTool {
    fn name(&self) -> &str {
        "excel_get_used_range"
    }
    fn description(&self) -> &str {
        "Read text from the active sheet's used range in Excel."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "max_chars": { "type": "integer" } }
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_excel(
            ctx,
            self.name(),
            get_used_range_text(capped_chars(args["max_chars"].as_u64())),
        )
    }
}

struct ExcelSetSelectionTool;
impl Tool for ExcelSetSelectionTool {
    fn name(&self) -> &str {
        "excel_set_selection"
    }
    fn description(&self) -> &str {
        "Write a value into the current Excel selection."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_excel(
            ctx,
            self.name(),
            set_selection_value(args["text"].as_str().unwrap_or("")),
        )
    }
}

struct ExcelSaveWorkbookTool;
impl Tool for ExcelSaveWorkbookTool {
    fn name(&self) -> &str {
        "excel_save_workbook"
    }
    fn description(&self) -> &str {
        "Save the active Excel workbook."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_excel(ctx, self.name(), save_active_workbook())
    }
}

struct PptGetSelectionTool;
impl Tool for PptGetSelectionTool {
    fn name(&self) -> &str {
        "ppt_get_selection"
    }
    fn description(&self) -> &str {
        "Read the current PowerPoint selection."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_ppt(ctx, self.name(), ppt_get_selection())
    }
}

struct PptGetSlideTextTool;
impl Tool for PptGetSlideTextTool {
    fn name(&self) -> &str {
        "ppt_get_slide_text"
    }
    fn description(&self) -> &str {
        "Read text from the current PowerPoint slide."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "max_chars": { "type": "integer" } }
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_ppt(
            ctx,
            self.name(),
            get_slide_text(capped_chars(args["max_chars"].as_u64())),
        )
    }
}

struct PptReplaceSelectionTool;
impl Tool for PptReplaceSelectionTool {
    fn name(&self) -> &str {
        "ppt_replace_selection"
    }
    fn description(&self) -> &str {
        "Replace the current PowerPoint selection."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_ppt(
            ctx,
            self.name(),
            ppt_replace(args["text"].as_str().unwrap_or("")),
        )
    }
}

struct PptInsertTextTool;
impl Tool for PptInsertTextTool {
    fn name(&self) -> &str {
        "ppt_insert_text"
    }
    fn description(&self) -> &str {
        "Insert text into the current PowerPoint selection."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_ppt(
            ctx,
            self.name(),
            ppt_insert(args["text"].as_str().unwrap_or("")),
        )
    }
}

struct PptSavePresentationTool;
impl Tool for PptSavePresentationTool {
    fn name(&self) -> &str {
        "ppt_save_presentation"
    }
    fn description(&self) -> &str {
        "Save the active PowerPoint presentation."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let _ = args;
        run_ppt(ctx, self.name(), save_active_presentation())
    }
}

fn capped_chars(value: Option<u64>) -> usize {
    value
        .map(|v| v as usize)
        .unwrap_or(50_000)
        .min(TOOL_OUTPUT_MAX_CHARS)
}

fn run_word(
    ctx: &ToolContext,
    tool: &str,
    result: Result<String, WordError>,
) -> Result<String, ToolError> {
    match result {
        Ok(text) => {
            let summary = sanitize_summary(&text, 120);
            emit_tool_debug(ctx, tool, true, &summary, None);
            Ok(text)
        }
        Err(error) => {
            emit_tool_debug(ctx, tool, false, "failed", Some(&error.to_string()));
            Err(ToolError::new(error.to_string()))
        }
    }
}

fn run_excel(
    ctx: &ToolContext,
    tool: &str,
    result: Result<String, ExcelError>,
) -> Result<String, ToolError> {
    match result {
        Ok(text) => {
            let summary = sanitize_summary(&text, 120);
            emit_tool_debug(ctx, tool, true, &summary, None);
            Ok(text)
        }
        Err(error) => {
            emit_tool_debug(ctx, tool, false, "failed", Some(&error.to_string()));
            Err(ToolError::new(error.to_string()))
        }
    }
}

fn run_ppt(
    ctx: &ToolContext,
    tool: &str,
    result: Result<String, PowerPointError>,
) -> Result<String, ToolError> {
    match result {
        Ok(text) => {
            let summary = sanitize_summary(&text, 120);
            emit_tool_debug(ctx, tool, true, &summary, None);
            Ok(text)
        }
        Err(error) => {
            emit_tool_debug(ctx, tool, false, "failed", Some(&error.to_string()));
            Err(ToolError::new(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::BusEvent;
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore};

    struct RecordingBus {
        events: Mutex<Vec<BusEvent>>,
    }

    impl crate::core::event::EventBus for RecordingBus {
        fn emit(&self, event: BusEvent) {
            if let Ok(mut guard) = self.events.lock() {
                guard.push(event);
            }
        }
    }

    fn test_context(bus: Arc<RecordingBus>) -> ToolContext {
        ToolContext {
            workspace_root: PathBuf::from("."),
            request_context: RequestContext::default(),
            session_id: "session-test".to_string(),
            assistant_message_id: "assistant-test".to_string(),
            conversation: Arc::new(ConversationManager::new(
                std::env::temp_dir()
                    .join(format!("anya-office-tool-test-{}.db", uuid::Uuid::new_v4())),
            )),
            event_bus: bus,
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 2,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn word_tool_fails_gracefully_when_word_not_running() {
        if super::super::word::word_is_available() {
            return;
        }
        let bus = Arc::new(RecordingBus {
            events: Mutex::new(Vec::new()),
        });
        let ctx = test_context(Arc::clone(&bus));
        let tool = WordGetSelectionTool;
        let result = tool.execute(&ctx, json!({}));
        assert!(result.is_err());
    }
}
