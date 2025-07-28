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
}

pub struct MetricsCollector {
    pub client_cache: ClientCache,
    first_seen_gauge: Family<PacketLabels, Gauge>,
    last_seen_gauge: Family<PacketLabels, Gauge>,
    packet_counter: Family<PacketLabels, Counter>,
    packet_size_histogram: Histogram,
    registry: Arc<Registry>,
    unique_clients_gauge: Family<ClientLabels, Gauge>,
}

impl MetricsCollector {
    pub fn new(cache_configs: &[(u64, u64)]) -> Self {
        let mut registry = Registry::default();

        let client_cache = ClientCache::new(cache_configs);
        let first_seen_gauge = Family::<PacketLabels, Gauge>::default();
        let last_seen_gauge = Family::<PacketLabels, Gauge>::default();
        let packet_counter = Family::<PacketLabels, Counter>::default();
        let packet_size_histogram = Histogram::new(vec![48.0, 56.0, 128.0].into_iter());
        let unique_clients_gauge = Family::<ClientLabels, Gauge>::default();

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
            "rsntp_packet_count",
            "NTP packet event counters",
            packet_counter.clone(),
        );
        registry.register(
            "rsntp_packet_size_bytes",
            "NTP packet size in bytes",
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

        // Because we are reading the gauge here, we need to scope the gauge
        // access to ensure the read lock for the current thread's gauge is
        // dropped before we attempt to write the one for the global thread.
        {
            let gauge = self.first_seen_gauge.get_or_create(&labels);
            // Only set if not already set (first time)
            if gauge.get() == 0 {
                gauge.set(current_time);
            }
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

    pub fn inc_client(&self, ip: IpAddr) {
        self.client_cache.inc_client(ip);
    }

    pub fn update_packet_counter(&self, event: PacketEvent, thread_id: u32) {
        self.increment_packet_counter(event, thread_id);
        self.update_first_seen_time(event, thread_id);
        self.update_last_seen_time(event, thread_id);
    }

    // set unique_clients_gauge for each period to the count of elements in that period's cache
    fn update_unique_clients(&self) {
        for (count, period) in self.client_cache.iter_counts_ttls() {
            let labels = ClientLabels {
                period: period.to_string(),
            };
            let gauge = self.unique_clients_gauge.get_or_create(&labels);
            gauge.set(count as i64);
        }
    }

}

#[cfg(test)]
mod tests {
    extern crate regex;
    use prometheus_client::encoding::text::encode;
    use self::regex::Regex;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use super::*;

    #[test]
    fn test_increment_packet_counter() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        collector.increment_packet_counter(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_first_seen_time() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        collector.update_first_seen_time(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_last_seen_time() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        collector.update_last_seen_time(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_record_packet_size() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        collector.record_packet_size(48);
        collector.record_packet_size(128);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_v4() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        collector.inc_client(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_v6() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        collector.inc_client(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_add_client_ip_disabled_cache() {
        let collector = MetricsCollector::new(&[]);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        collector.inc_client(ip);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_update_packet_counter() {
        let collector = MetricsCollector::new(&[(100, 60), (1000, 3600), (10000, 86400)]);
        collector.update_packet_counter(PacketEvent::ServerRequestReceived, 1);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_registry_access() {
        let collector = MetricsCollector::new(&[]);
        let registry = collector.registry();
        collector.record_packet_size(48);
        collector.record_packet_size(128);

        // test basic registry output
        let mut buffer = String::new();
        let _ = encode(&mut buffer, &registry);
        assert!(buffer.contains("rsntp_packet_count"));
        assert!(buffer.contains("# TYPE rsntp_packet_size_bytes histogram"));

        // we should have 48 + 128 bytes total, in 2 packets
        let expected_sum = (48 + 128) as f64;
        let re = Regex::new(&format!(r"(?m)^rsntp_packet_size_bytes_sum {:.1}$", expected_sum)).unwrap();
        assert!(re.is_match(&buffer));
        let re = Regex::new(r"(?m)^rsntp_packet_size_bytes_count 2$").unwrap();
        assert!(re.is_match(&buffer));
    }

}
