use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::future::join_all;

use crate::core::ai::provider::ProviderError;
use crate::core::chat::limits::truncate_tool_output;
use crate::core::runtime::{ToolActivity, ToolCallPayload};
use crate::core::tools::context::ToolContext;
use crate::core::tools::display::build_activity_view;
use crate::core::tools::error::ToolError;
use crate::runtime::ToolManager;
use tracing::Instrument;

use super::types::{now_millis, StartedTool, ToolOutcome};

/// Runs tool calls for one step, either serially (default, needed when any
/// call is non-read-only) or in parallel (only when every call in the batch
/// is read-only), and turns each dispatch into a [`ToolOutcome`].
pub struct ToolExecutor {
    tools: Arc<ToolManager>,
    tool_output_max_chars: usize,
}

impl ToolExecutor {
    pub fn new(tools: Arc<ToolManager>, tool_output_max_chars: usize) -> Self {
        Self {
            tools,
            tool_output_max_chars,
        }
    }

    /// Parallel execution is only safe when every call in the batch is
    /// read-only; a single mutating call forces the serial path so ordering
    /// and side effects stay predictable.
    pub fn should_run_parallel(&self, tool_calls: &[ToolCallPayload]) -> bool {
        tool_calls.len() > 1
            && tool_calls
                .iter()
                .all(|call| self.tools.is_read_only(&call.name))
    }

    pub async fn execute_tools_serial(
        &self,
        tool_calls: &[ToolCallPayload],
        tool_ctx: &ToolContext,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<Vec<ToolOutcome>, ProviderError> {
        let mut outcomes = Vec::with_capacity(tool_calls.len());
        for call in tool_calls {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            outcomes.push(self.execute_one_tool(call, tool_ctx).await?);
        }
        Ok(outcomes)
    }

    pub async fn execute_tools_parallel(
        &self,
        tool_calls: &[ToolCallPayload],
        tool_ctx: &ToolContext,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<Vec<ToolOutcome>, ProviderError> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(ProviderError::cancelled());
        }

        // Emit "running" UI events on the agent task before dispatching workers.
        let prepared: Vec<_> = tool_calls
            .iter()
            .map(|call| self.begin_tool_activity(call, tool_ctx))
            .collect();

        let jobs = prepared.into_iter().map(|started| {
            let tools = Arc::clone(&self.tools);
            let mut execution_context = tool_ctx.clone();
            execution_context.parent_activity_id = Some(started.activity_id.clone());
            let tool_name = started.tool_name.clone();
            let tool_args = started.args.clone();
            let max_chars = self.tool_output_max_chars;
            async move {
                let span = tracing::info_span!(
                    target: "peek.tool",
                    "tool_dispatch",
                    tool = %tool_name,
                    call_id = %started.call_id,
                    session_id = %execution_context.session_id,
                );
                tracing::debug!(parent: &span, "tool start");
                let execution = tools
                    .dispatch_async(&execution_context, &tool_name, tool_args)
                    .instrument(span.clone())
                    .await;
                tracing::debug!(parent: &span, success = execution.is_ok(), "tool done");
                (started, execution, max_chars)
            }
        });

        let finished = join_all(jobs).await;
        if cancelled.load(Ordering::Relaxed) {
            return Err(ProviderError::cancelled());
        }
        let mut outcomes = Vec::with_capacity(finished.len());
        for (started, execution, max_chars) in finished {
            outcomes.push(self.finish_tool_activity(started, execution, tool_ctx, max_chars));
        }
        Ok(outcomes)
    }

    async fn execute_one_tool(
        &self,
        call: &ToolCallPayload,
        tool_ctx: &ToolContext,
    ) -> Result<ToolOutcome, ProviderError> {
        let span = tracing::info_span!(
            target: "peek.tool",
            "tool_dispatch",
            tool = %call.name,
            call_id = %call.id,
            session_id = %tool_ctx.session_id,
        );
        tracing::debug!(parent: &span, "tool start");

        let started = self.begin_tool_activity(call, tool_ctx);
        let tools = Arc::clone(&self.tools);
        let mut execution_context = tool_ctx.clone();
        execution_context.parent_activity_id = Some(started.activity_id.clone());
        let tool_name = started.tool_name.clone();
        let tool_args = started.args.clone();
        let execution = tools
            .dispatch_async(&execution_context, &tool_name, tool_args)
            .instrument(span.clone())
            .await;
        tracing::debug!(parent: &span, success = execution.is_ok(), "tool done");
        if tool_ctx.is_cancelled()
            || execution
                .as_ref()
                .err()
                .is_some_and(ToolError::is_cancelled)
        {
            return Err(ProviderError::cancelled());
        }
        Ok(self.finish_tool_activity(started, execution, tool_ctx, self.tool_output_max_chars))
    }

    fn begin_tool_activity(&self, call: &ToolCallPayload, tool_ctx: &ToolContext) -> StartedTool {
        let args: serde_json::Value =
            serde_json::from_str(&call.arguments).unwrap_or_else(|_| serde_json::json!({}));
        let activity_id = format!("tool-{}-{}", call.id, now_millis());
        let activity_view = build_activity_view(&call.name, &args, None);
        let preview_detail = activity_view.detail.clone();
        let tool_preview = self.tools.preview(tool_ctx, &call.name, &args);
        let display_sid = tool_ctx.root_session_id().to_string();
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: activity_id.clone(),
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                tool_name: call.name.clone(),
                title: activity_view.title.clone(),
                kind: activity_view.kind.clone(),
                detail: activity_view.detail.clone(),
                arguments: Some(args.clone()),
                result: None,
                preview: tool_preview.clone(),
                success: true,
                status: "running".to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolStarted {
                session_id: display_sid,
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: activity_id.clone(),
                tool_name: call.name.clone(),
                title: activity_view.title.clone(),
                kind: activity_view.kind.clone(),
                detail: activity_view.detail,
                arguments: args.clone(),
                preview: tool_preview.clone(),
            });
        StartedTool {
            call_id: call.id.clone(),
            tool_name: call.name.clone(),
            activity_id,
            args,
            preview_detail,
            tool_preview,
        }
    }

    fn finish_tool_activity(
        &self,
        started: StartedTool,
        execution: Result<String, ToolError>,
        tool_ctx: &ToolContext,
        max_chars: usize,
    ) -> ToolOutcome {
        let user_denied = execution.as_ref().err().is_some_and(ToolError::is_terminal);
        let (raw_result, success) = match execution {
            Ok(value) => (value, true),
            Err(error) => (format!("tool error: {error}"), false),
        };
        let result = truncate_tool_output(&raw_result, max_chars);
        let finished = build_activity_view(&started.tool_name, &started.args, Some(&result));
        let detail = finished.detail.or(started.preview_detail);
        let display_sid = tool_ctx.root_session_id().to_string();
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: started.activity_id.clone(),
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                tool_name: started.tool_name.clone(),
                title: finished.title.clone(),
                kind: finished.kind.clone(),
                detail: detail.clone(),
                arguments: Some(started.args.clone()),
                result: Some(result.clone()),
                preview: started.tool_preview.clone(),
                success,
                status: if success { "done" } else { "error" }.to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolFinished {
                session_id: display_sid,
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: started.activity_id,
                tool_name: started.tool_name.clone(),
                title: finished.title,
                kind: finished.kind,
                detail,
                arguments: started.args.clone(),
                preview: started.tool_preview,
                result: result.clone(),
                success,
            });
        ToolOutcome {
            call_id: started.call_id,
            tool_name: started.tool_name,
            arguments: serde_json::to_string(&started.args).unwrap_or_default(),
            result,
            success,
            user_denied,
        }
    }
}
