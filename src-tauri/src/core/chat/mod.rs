//! Chat domain: send path, streaming, prompt assembly, and the **main** agent loop.
//!
//! Hot path: [`service::ChatService`] → [`stream::StreamManager`] →
//! [`agent::AgentRunner`] → [`agent_loop`]. Run lifecycle shell is
//! [`crate::core::agent::AgentRuntime`]. Overview: `docs/architecture-overview.md`.

pub mod agent;
mod agent_loop;
#[cfg(test)]
mod agent_loop_tests;
pub mod compact;
pub mod conversation_manager;
pub mod db;
pub mod error;
pub mod eval_harness;
pub mod journal;
pub mod limits;
pub mod model_context;
pub mod preferences;
pub mod prompt;
pub mod prompts;
mod selection;
pub mod service;
pub mod session_origin;
pub mod session_title;
pub mod stream;
pub mod telemetry;
pub mod trajectory;

pub use preferences::SendPreferences;
pub use service::ChatService;
