use std::sync::Arc;
use std::time::Duration;

pub type Settings = Arc<ServerSettings>;

pub struct ServerSettings {
    pub backlog: usize,
    pub keep_alive: Duration,
}
