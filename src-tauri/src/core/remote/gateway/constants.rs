use std::time::Duration;

pub(super) const PATH: &str = "/remote/v1";
pub(super) const PING_INTERVAL: Duration = Duration::from_secs(15);
pub(super) const INBOUND_DEADLINE: Duration = Duration::from_secs(45);
pub(super) const SEND_TIMEOUT: Duration = Duration::from_secs(10);
