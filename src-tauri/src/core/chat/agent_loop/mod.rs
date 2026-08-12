//! Building blocks for [`super::agent::AgentRunner::run`], split out so new
//! turn policies (extra challenges, alternate compaction strategies, etc.)
//! can be added without growing a single giant loop function.
//!
//! - [`types`]: shared value types (`StartedTool`, `ToolOutcome`) and small
//!   helpers (`now_millis`, `non_empty`, `merge_tool_call`,
//!   `estimate_request_tokens`).
//! - [`stream_turn`]: collects one provider stream turn into
//!   content/reasoning/tool_calls/finish_reason, forwarding UI-relevant
//!   events to the outer channel.
//! - [`mid_turn_compact`]: mid-turn auto-compact near the context window.
//! - [`soft_inject`]: drains queued follow-up instructions into the request.
//! - [`challenge`]: honest-completion / verification challenge policy.
//! - [`failure`]: consecutive-failure and repeated-identical-error circuit
//!   breaker.
//! - [`tools`]: serial/parallel tool dispatch and tool-activity UI events.

pub mod challenge;
pub mod failure;
pub mod mid_turn_compact;
pub mod post_edit_verify;
pub mod soft_inject;
pub mod stream_turn;
pub mod tools;
pub mod types;
