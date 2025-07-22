pub mod client_cache;
pub mod events;
pub mod http_server;

use prometheus_client::registry::Registry;
use std::sync::Arc;

pub struct MetricsCollector {
    registry: Arc<Registry>,
    enabled: bool,
}

impl MetricsCollector {
    pub fn new(enabled: bool) -> Self {
        Self {
            registry: Arc::new(Registry::default()),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
