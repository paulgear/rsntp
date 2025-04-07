use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Mutex;
use prometheus::{IntCounterVec, Registry, register_int_counter_vec};

pub struct Metrics {
    // Packets received, grouped by IP protocol version (4 or 6)
    packets_received: IntCounterVec,

    // Packets sent, grouped by IP protocol version (4 or 6)
    packets_sent: IntCounterVec,

    // Packets dropped, grouped by reason
    packets_dropped: IntCounterVec,

    // Unique remote addresses seen, tracked separately
    unique_addresses_v4: Mutex<HashSet<String>>,
    unique_addresses_v6: Mutex<HashSet<String>>,

    // Counters for unique addresses
    unique_addresses: IntCounterVec,

    // Registry for all metrics
    registry: Registry,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        // Create metrics
        let packets_received = register_int_counter_vec!(
            "ntp_packets_received_total",
            "Total number of NTP packets received",
            &["ip_version"],
        ).unwrap();

        let packets_sent = register_int_counter_vec!(
            "ntp_packets_sent_total",
            "Total number of NTP packets sent",
            &["ip_version"],
        ).unwrap();

        let packets_dropped = register_int_counter_vec!(
            "ntp_packets_dropped_total",
            "Total number of NTP packets dropped",
            &["reason"],
        ).unwrap();

        let unique_addresses = register_int_counter_vec!(
            "ntp_unique_addresses_total",
            "Total number of unique remote addresses seen",
            &["ip_version"],
        ).unwrap();

        // Register metrics with the registry
        registry.register(Box::new(packets_received.clone())).unwrap();
        registry.register(Box::new(packets_sent.clone())).unwrap();
        registry.register(Box::new(packets_dropped.clone())).unwrap();
        registry.register(Box::new(unique_addresses.clone())).unwrap();

        Metrics {
            packets_received,
            packets_sent,
            packets_dropped,
            unique_addresses_v4: Mutex::new(HashSet::new()),
            unique_addresses_v6: Mutex::new(HashSet::new()),
            unique_addresses,
            registry,
        }
    }

    pub fn record_packet_received(&self, addr: &SocketAddr) {
        match addr {
            SocketAddr::V4(_) => {
                self.packets_received.with_label_values(&["ipv4"]).inc();

                // Track unique IPv4 addresses
                let addr_str = addr.to_string();
                let mut unique_addrs = self.unique_addresses_v4.lock().unwrap();
                if !unique_addrs.contains(&addr_str) {
                    unique_addrs.insert(addr_str);
                    self.unique_addresses.with_label_values(&["ipv4"]).inc();
                }
            },
            SocketAddr::V6(_) => {
                self.packets_received.with_label_values(&["ipv6"]).inc();

                // Track unique IPv6 addresses
                let addr_str = addr.to_string();
                let mut unique_addrs = self.unique_addresses_v6.lock().unwrap();
                if !unique_addrs.contains(&addr_str) {
                    unique_addrs.insert(addr_str);
                    self.unique_addresses.with_label_values(&["ipv6"]).inc();
                }
            },
        }
    }

    pub fn record_packet_sent(&self, addr: &SocketAddr) {
        match addr {
            SocketAddr::V4(_) => {
                self.packets_sent.with_label_values(&["ipv4"]).inc();
            },
            SocketAddr::V6(_) => {
                self.packets_sent.with_label_values(&["ipv6"]).inc();
            },
        }
    }

    pub fn record_packet_dropped(&self, reason: &str) {
        self.packets_dropped.with_label_values(&[reason]).inc();
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}
