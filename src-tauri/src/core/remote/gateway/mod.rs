//! WebSocket remote gateway for the Anya Companion app.

mod auth;
mod chat_send;
mod companion;
mod compose;
mod constants;
mod interactions;
mod outbound;
mod payloads;
mod rpc;
mod send;
mod server;
mod workspace_files;

pub use companion::handle_companion_stream;
pub use server::start_gateway;
