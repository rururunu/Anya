use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::runtime::RequestContext;

use super::{AgentEventRecord, AgentState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type",
    content = "data"
)]
pub enum AgentDebugEvent {
    RunCreated {
        run_id: String,
        state: AgentState,
    },
    ContextSnapshot {
        run_id: String,
        context: Box<RequestContext>,
    },
    TokenUsage {
        run_id: String,
        model: String,
        usage: crate::core::token::TokenUsage,
    },
    RuntimeEvent {
        record: AgentEventRecord,
    },
    ToolCall {
        run_id: String,
        call_id: String,
        tool: String,
        description: String,
        arguments: Value,
    },
    SubagentStarted {
        run_id: String,
        subagent_id: String,
        parent_subagent_id: Option<String>,
        description: String,
        read_only: bool,
        depth: u32,
        timestamp_ms: u64,
    },
    SubagentProgress {
        run_id: String,
        subagent_id: String,
        kind: String,
        content: String,
        timestamp_ms: u64,
    },
    SubagentToolCall {
        run_id: String,
        subagent_id: String,
        call_id: String,
        tool: String,
        description: String,
        arguments: Value,
        timestamp_ms: u64,
    },
    SubagentToolResult {
        run_id: String,
        subagent_id: String,
        call_id: String,
        tool: String,
        success: bool,
        result: String,
        timestamp_ms: u64,
    },
    SubagentFinished {
        run_id: String,
        subagent_id: String,
        success: bool,
        summary: String,
        timestamp_ms: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_event_serializes_for_frontend_transport() {
        let value = serde_json::to_value(AgentDebugEvent::ToolCall {
            run_id: "run-1".to_string(),
            call_id: "call-1".to_string(),
            tool: "shell".to_string(),
            description: "Check project".to_string(),
            arguments: serde_json::json!({ "command": "cargo check" }),
        })
        .unwrap();

        assert_eq!(value["type"], "toolCall");
        assert_eq!(value["data"]["runId"], "run-1");
        assert_eq!(value["data"]["arguments"]["command"], "cargo check");
    }

    #[test]
    fn boxed_context_snapshot_preserves_frontend_payload() {
        let context = RequestContext::default();
        let expected = serde_json::to_value(&context).unwrap();
        let value = serde_json::to_value(AgentDebugEvent::ContextSnapshot {
            run_id: "run-1".to_string(),
            context: Box::new(context),
        })
        .unwrap();

        assert_eq!(value["type"], "contextSnapshot");
        assert_eq!(value["data"]["runId"], "run-1");
        assert_eq!(value["data"]["context"], expected);
    }

    #[test]
    fn subagent_event_serializes_with_parent_run_identity() {
        let value = serde_json::to_value(AgentDebugEvent::SubagentStarted {
            run_id: "run-1".to_string(),
            subagent_id: "child-1".to_string(),
            parent_subagent_id: None,
            description: "Inspect context".to_string(),
            read_only: true,
            depth: 1,
            timestamp_ms: 42,
        })
        .unwrap();

        assert_eq!(value["type"], "subagentStarted");
        assert_eq!(value["data"]["runId"], "run-1");
        assert_eq!(value["data"]["subagentId"], "child-1");
        assert_eq!(value["data"]["readOnly"], true);
    }
}
