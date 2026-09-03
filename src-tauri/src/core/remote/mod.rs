//! Remote Gateway — WebSocket bridge for Anya Android Companion.
//!
//! Phone authenticates with a short-lived pairing token (QR / code) or a
//! previously registered device credential, then speaks the shared wire
//! protocol under `/remote/v1`.

mod bridge;
mod compose;
mod download;
mod gateway;
mod http_proxy;
mod pairing;
pub mod preview;
mod protocol;
mod staged;
mod state;
mod tunnel;
mod upload;

pub use bridge::{
    on_bus_event, push_interaction_resolved, push_session_rewound,
    resume_run_state_after_interaction,
};
pub use compose::SessionCompose;
pub use gateway::start_gateway;
pub use pairing::create_pairing_session;
pub use state::{
    gateway_status, list_paired_devices, remote_state, restore_gateway_if_enabled, revoke_device,
    stop_gateway, GatewayStatus, PairedDevice, PairingSessionInfo, TunnelPrefs,
};
pub use upload::{cleanup_session_uploads, inbox_root_if_exists, MAX_UPLOAD_BYTES};

pub fn get_session_compose(session_id: &str) -> SessionCompose {
    compose::get(session_id)
}

pub fn set_session_compose(session_id: &str, compose: SessionCompose) -> SessionCompose {
    compose::set(session_id, compose)
}

pub fn broadcast_compose_event(session_id: &str, compose: &SessionCompose) {
    bridge::broadcast_server_message(&protocol::ServerMessage::Event {
        name: "session.compose".into(),
        data: compose::event_payload(session_id, compose),
    });
}

pub fn list_staged(session_id: &str) -> Vec<String> {
    staged::list(session_id)
}

fn broadcast_staged(session_id: &str, messages: &[String]) {
    bridge::broadcast_server_message(&protocol::ServerMessage::Event {
        name: "session.staged".into(),
        data: staged::event_payload(session_id, messages),
    });
}

pub fn push_staged(session_id: &str, content: &str) -> Vec<String> {
    let messages = staged::push(session_id, content);
    broadcast_staged(session_id, &messages);
    messages
}

pub fn remove_staged(session_id: &str, index: usize) -> Vec<String> {
    let messages = staged::remove(session_id, index);
    broadcast_staged(session_id, &messages);
    messages
}

pub fn clear_staged(session_id: &str) {
    staged::clear(session_id);
    broadcast_staged(session_id, &[]);
}

pub fn take_staged_at(session_id: &str, index: usize) -> Option<String> {
    let content = staged::take_at(session_id, index)?;
    broadcast_staged(session_id, &staged::list(session_id));
    Some(content)
}

pub fn pop_staged_front(session_id: &str) -> Option<String> {
    let content = staged::pop_front(session_id)?;
    broadcast_staged(session_id, &staged::list(session_id));
    Some(content)
}

pub fn insert_staged(session_id: &str, index: usize, content: &str) -> Vec<String> {
    let messages = staged::insert(session_id, index, content);
    broadcast_staged(session_id, &messages);
    messages
}
