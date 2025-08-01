use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;
use rustc_version_runtime::version;
use sysinfo::{System, Pid};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RustInfoLabels {
    version: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RsntpInfoLabels {
    version: String,
}

pub struct ProcessMetrics {
    // Gauges
    threads: Gauge,
    memory_vss: Gauge,
    memory_rss: Gauge,
    memory_swap: Gauge,
    start_time: Gauge,
    open_fds: Gauge,
    max_fds: Gauge,

    // System instance for collecting metrics
    system: System,
    process_start_time: u64,
}

impl ProcessMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let process_start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let rust_info_labels = RustInfoLabels {
            version: version().to_string(),
        };

        let rsntp_info_labels = RsntpInfoLabels {
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let threads = Gauge::default();
        let memory_vss = Gauge::default();
        let memory_rss = Gauge::default();
        let memory_swap = Gauge::default();
        let start_time = Gauge::default();
        let open_fds = Gauge::default();
        let max_fds = Gauge::default();
        let rust_info_metric = Info::new(rust_info_labels);
        let rsntp_info_metric = Info::new(rsntp_info_labels);

        registry.register("rsntp_process_threads_total", "Number of OS threads in the process", threads.clone());
        registry.register("rsntp_process_virtual_memory_bytes", "Virtual memory size in bytes", memory_vss.clone());
        registry.register("rsntp_process_resident_memory_bytes", "Resident memory size in bytes", memory_rss.clone());
        registry.register("rsntp_process_swap_memory_bytes", "Swap memory used by process in bytes", memory_swap.clone());
        registry.register("rsntp_process_start_time_seconds", "Start time of the process since unix epoch in seconds", start_time.clone());
        registry.register("rsntp_process_open_fds_total", "Number of open file descriptors", open_fds.clone());
        registry.register("rsntp_process_max_fds", "Maximum number of open file descriptors", max_fds.clone());
        registry.register("rsntp_rust", "Information about the Rust version", rust_info_metric);
        registry.register("rsntp", "Information about the rsntp version", rsntp_info_metric);

        Self {
            threads,
            memory_vss,
            memory_rss,
            memory_swap,
            start_time,
            open_fds,
            max_fds,
            system,
            process_start_time,
        }
    }

    pub fn update(&mut self) {
        self.system.refresh_all();

        if let Some(process) = self.system.process(Pid::from(std::process::id() as usize)) {
            // Memory metrics (convert from KB to bytes)
            self.memory_rss.set((process.memory() * 1024) as i64);
            self.memory_vss.set((process.virtual_memory() * 1024) as i64);

            // Process start time
            self.start_time.set(self.process_start_time as i64);
        }

        // File descriptor information (Linux-specific)
        if let Ok(open_fds) = std::fs::read_dir("/proc/self/fd") {
            self.open_fds.set(open_fds.count() as i64);
        }

        if let Ok(limits) = std::fs::read_to_string("/proc/self/limits") {
            for line in limits.lines() {
                if line.starts_with("Max open files") {
                    if let Some(parts) = line.split_whitespace().nth(3) {
                        if let Ok(max_fds) = parts.parse::<i64>() {
                            self.max_fds.set(max_fds);
                            break;
                        }
                    }
                }
            }
        }

        // Parse thread count and swap usage from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("Threads:") {
                    if let Some(count_str) = line.split_whitespace().nth(1) {
                        if let Ok(count) = count_str.parse::<i64>() {
                            self.threads.set(count);
                        }
                    }
                } else if line.starts_with("VmSwap:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<i64>() {
                            self.memory_swap.set(kb * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
}
