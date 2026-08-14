//! Remote Gateway — WebSocket bridge for Anya Android Companion.
//!
//! Phone authenticates with a short-lived pairing token (QR / code) or a
//! previously registered device credential, then speaks the shared wire
//! protocol under `/remote/v1`.

mod bridge;
mod tunnel;
mod compose;
mod download;
mod gateway;
mod http_proxy;
mod pairing;
pub mod preview;
mod protocol;
mod state;
mod upload;

pub use compose::SessionCompose;
pub use bridge::{on_bus_event, push_interaction_resolved};
pub use gateway::start_gateway;
pub use pairing::create_pairing_session;
pub use state::{
    gateway_status, list_paired_devices, restore_gateway_if_enabled, revoke_device, stop_gateway,
    GatewayStatus, PairedDevice, PairingSessionInfo, TunnelPrefs, remote_state,
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
