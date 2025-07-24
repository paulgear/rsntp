pub mod client_cache;
pub mod events;
pub mod http_server;

pub use self::http_server::MetricsServer;

use crate::metrics::client_cache::ClientCache;
use crate::metrics::events::PacketEvent;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct PacketLabels {
    thread_id: String,
    packet_event: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct ClientLabels {
    period: String,
    ip_version: String,
}

pub struct MetricsCollector {
    registry: Arc<Registry>,
    packet_counter: Family<PacketLabels, Counter>,
    first_seen_gauge: Family<PacketLabels, Gauge>,
    last_seen_gauge: Family<PacketLabels, Gauge>,
    packet_size_histogram: Histogram,
    unique_clients_gauge: Family<ClientLabels, Gauge>,
    client_cache: Option<ClientCache>,
}

impl MetricsCollector {
    pub fn new(minute_limit: u64, hour_limit: u64, day_limit: u64) -> Self {
        let mut registry = Registry::default();

        let packet_counter = Family::<PacketLabels, Counter>::default();
        let first_seen_gauge = Family::<PacketLabels, Gauge>::default();
        let last_seen_gauge = Family::<PacketLabels, Gauge>::default();

        // Histogram buckets: <48, 48-56, 56-128, 128+ bytes
        let packet_size_histogram = Histogram::new(vec![48.0, 56.0, 128.0].into_iter());

        let unique_clients_gauge = Family::<ClientLabels, Gauge>::default();

        let client_cache = if minute_limit == 0 || hour_limit == 0 || day_limit == 0 {
            None
        } else {
            Some(ClientCache::new(minute_limit, hour_limit, day_limit))
        };

        registry.register(
            "rsntp_packet_count",
            "NTP packet event counters",
            packet_counter.clone(),
        );
        registry.register(
            "rsntp_first_seen_time",
            "First time each packet event was seen (Unix nanoseconds)",
            first_seen_gauge.clone(),
        );
        registry.register(
            "rsntp_last_seen_time",
            "Last time each packet event was seen (Unix nanoseconds)",
            last_seen_gauge.clone(),
        );
        registry.register(
            "rsntp_packet_size_bytes",
            "Histogram of packet sizes in bytes",
            packet_size_histogram.clone(),
        );
        registry.register(
            "rsntp_unique_clients",
            "Number of unique client IP addresses by time period",
            unique_clients_gauge.clone(),
        );

        Self {
            registry: Arc::new(registry),
            packet_counter,
            first_seen_gauge,
            last_seen_gauge,
            packet_size_histogram,
            unique_clients_gauge,
            client_cache,
        }
    }

    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    fn current_time_nanos() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64
    }

    pub fn increment_packet_counter(&self, event: PacketEvent, thread_id: u32) {
        let labels = PacketLabels {
            thread_id: thread_id.to_string(),
            packet_event: event.as_str().to_string(),
        };
        self.packet_counter.get_or_create(&labels).inc();

        // Also increment global counter (thread_id = 0)
        if thread_id != 0 {
            self.increment_packet_counter(event, 0);
        }
    }

    pub fn update_first_seen_time(&self, event: PacketEvent, thread_id: u32) {
        let current_time = Self::current_time_nanos();
        let labels = PacketLabels {
            thread_id: thread_id.to_string(),
            packet_event: event.as_str().to_string(),
        };

        let gauge = self.first_seen_gauge.get_or_create(&labels);
        // Only set if not already set (first time)
        if gauge.get() == 0 {
            gauge.set(current_time);
        }

        // Also update global gauge (thread_id = 0)
        if thread_id != 0 {
            self.update_first_seen_time(event, 0);
        }
    }

    pub fn update_last_seen_time(&self, event: PacketEvent, thread_id: u32) {
        let current_time = Self::current_time_nanos();
        let labels = PacketLabels {
            thread_id: thread_id.to_string(),
            packet_event: event.as_str().to_string(),
        };

        self.last_seen_gauge.get_or_create(&labels).set(current_time);

        // Also update global gauge (thread_id = 0)
        if thread_id != 0 {
            self.update_last_seen_time(event, 0);
        }
    }

    pub fn record_packet_size(&self, size_bytes: usize) {
        self.packet_size_histogram.observe(size_bytes as f64);
    }

    pub fn add_client_ip(&self, ip: IpAddr) {
        if let Some(ref cache) = self.client_cache {
            let (minute_new, hour_new, day_new) = cache.add_client(ip);
            let ip_version = match ip {
                IpAddr::V4(_) => "4",
                IpAddr::V6(_) => "6",
            };

            if minute_new {
                let labels = ClientLabels {
                    period: "minute".to_string(),
                    ip_version: ip_version.to_string(),
                };
                self.unique_clients_gauge.get_or_create(&labels).inc();
            }

            if hour_new {
                let labels = ClientLabels {
                    period: "hour".to_string(),
                    ip_version: ip_version.to_string(),
                };
                self.unique_clients_gauge.get_or_create(&labels).inc();
            }

            if day_new {
                let labels = ClientLabels {
                    period: "day".to_string(),
                    ip_version: ip_version.to_string(),
                };
                self.unique_clients_gauge.get_or_create(&labels).inc();
            }
        }
    }

    pub fn update_packet_counter(&self, event: PacketEvent, thread_id: u32) {
        self.increment_packet_counter(event, thread_id);
        self.update_first_seen_time(event, thread_id);
        self.update_last_seen_time(event, thread_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_new_metrics_collector() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        assert!(collector.client_cache.is_some());
    }

    #[test]
    fn test_new_metrics_collector_disabled_cache() {
        let collector = MetricsCollector::new(0, 1000, 10000);
        assert!(collector.client_cache.is_none());
    }

    #[test]
    fn test_increment_packet_counter() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        collector.increment_packet_counter(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_first_seen_time() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        collector.update_first_seen_time(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_last_seen_time() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        collector.update_last_seen_time(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_record_packet_size() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        collector.record_packet_size(48);
        collector.record_packet_size(128);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_v4() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        collector.add_client_ip(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_v6() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        collector.add_client_ip(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_disabled_cache() {
        let collector = MetricsCollector::new(0, 1000, 10000);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        collector.add_client_ip(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_packet_counter() {
        let collector = MetricsCollector::new(100, 1000, 10000);
        collector.update_packet_counter(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_registry_access() {
        let collector = MetricsCollector::new(0, 0, 0);
        let _registry = collector.registry();
    }

    #[test]
    fn test_gauge_operations_simple() {
        let collector = MetricsCollector::new(0, 0, 0);
        // Test just the gauge creation without calling methods
        let labels = PacketLabels {
            thread_id: "1".to_string(),
            packet_event: "test".to_string(),
        };
        let _gauge = collector.first_seen_gauge.get_or_create(&labels);
    }

    #[test]
    fn test_direct_gauge() {
        use prometheus_client::metrics::gauge::Gauge;
        let gauge: Gauge = Gauge::default();
        assert_eq!(0, gauge.set(42));
        assert_eq!(42, gauge.get());
    }

    #[test]
    fn test_family_gauge_set() {
        use prometheus_client::metrics::family::Family;
        use prometheus_client::metrics::gauge::Gauge;
        let family: Family<PacketLabels, Gauge> = Family::default();
        let labels = PacketLabels {
            thread_id: "1".to_string(),
            packet_event: "test".to_string(),
        };
        let gauge = family.get_or_create(&labels);
        gauge.set(42);
        assert_eq!(42, gauge.get());
    }
}
