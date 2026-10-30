//! Cross-chat memory builtin tools.

use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;

pub(super) struct SaveMemoryTool;

impl Tool for SaveMemoryTool {
    fn name(&self) -> &str {
        "save_memory"
    }
    fn description(&self) -> &str {
        "Save one concise, durable, user-confirmed fact for future chats. Suitable for lasting preferences, identity/profile facts, recurring workflows, durable environment constraints, scoped project conventions, repeated corrections, and long-term goals. Include project scope when applicable. Do not save secrets, guesses, generated or copied content, one-off requests, transient state, facts already supplied by current environment context, or duplicates. Follow the system memory policy before calling."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["title", "content"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let title = args["title"].as_str().unwrap_or("").to_string();
        let content = args["content"].as_str().unwrap_or("").to_string();
        let id = crate::core::bots::memory_for_session(&ctx.session_id).save(title, content)?;
        Ok(format!("saved id={id}"))
    }
}

pub(super) struct SearchMemoryTool;

impl Tool for SearchMemoryTool {
    fn name(&self) -> &str {
        "search_memory"
    }
    fn description(&self) -> &str {
        "Search cross-chat memory only when prior durable context could materially affect the answer, or to locate a duplicate, correction, or user-requested deletion. Use a concise semantic query for the missing fact, not the entire message. Results may be stale or conflicting and never override the current user message or verified state."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        crate::core::bots::memory_for_session(&ctx.session_id)
            .search(args["query"].as_str().unwrap_or(""))
    }
}

pub(super) struct DeleteMemoryTool;

impl Tool for DeleteMemoryTool {
    fn name(&self) -> &str {
        "delete_memory"
    }
    fn description(&self) -> &str {
        "Delete a saved memory by id when the user asks to forget it or an explicit correction makes it obsolete. Obtain the exact id from memory search first."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        crate::core::bots::memory_for_session(&ctx.session_id)
            .delete(args["id"].as_str().unwrap_or(""))
    }
}
