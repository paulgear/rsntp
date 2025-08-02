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

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct MemoryLabels {
    memory_type: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct FdLabels {
    fd_type: String,
}

pub struct ProcessMetrics {
    // Gauges
    threads: Gauge,
    memory_bytes: prometheus_client::metrics::family::Family<MemoryLabels, Gauge>,
    start_time: Gauge,
    fds: prometheus_client::metrics::family::Family<FdLabels, Gauge>,

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
        let memory_bytes = prometheus_client::metrics::family::Family::default();
        let start_time = Gauge::default();
        let fds = prometheus_client::metrics::family::Family::default();
        let rust_info_metric = Info::new(rust_info_labels);
        let rsntp_info_metric = Info::new(rsntp_info_labels);

        registry.register("rsntp", "Information about the rsntp version", rsntp_info_metric);
        registry.register("rsntp_process_fds", "Process file descriptors by type", fds.clone());
        registry.register("rsntp_process_memory_bytes", "Process memory usage in bytes by type", memory_bytes.clone());
        registry.register("rsntp_process_start_time_seconds", "Start time of the process since unix epoch in seconds", start_time.clone());
        registry.register("rsntp_process_threads", "Number of OS threads in the process", threads.clone());
        registry.register("rsntp_rust", "Information about the Rust version", rust_info_metric);

        Self {
            threads,
            memory_bytes,
            start_time,
            fds,
            system,
            process_start_time,
        }
    }

    pub fn update(&mut self) {
        self.system.refresh_all();

        if let Some(_process) = self.system.process(Pid::from(std::process::id() as usize)) {
            // Process start time
            self.start_time.set(self.process_start_time as i64);
        }

        // File descriptor information (Linux-specific)
        if let Ok(open_fds) = std::fs::read_dir("/proc/self/fd") {
            let labels = FdLabels { fd_type: "open".to_string() };
            self.fds.get_or_create(&labels).set(open_fds.count() as i64);
        }

        if let Ok(limits) = std::fs::read_to_string("/proc/self/limits") {
            for line in limits.lines() {
                if line.starts_with("Max open files") {
                    if let Some(parts) = line.split_whitespace().nth(3) {
                        if let Ok(max_fds) = parts.parse::<i64>() {
                            let labels = FdLabels { fd_type: "max".to_string() };
                            self.fds.get_or_create(&labels).set(max_fds);
                            break;
                        }
                    }
                }
            }
        }

        // Parse thread count and all Vm* memory metrics from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("Threads:") {
                    if let Some(count_str) = line.split_whitespace().nth(1) {
                        if let Ok(count) = count_str.parse::<i64>() {
                            self.threads.set(count);
                        }
                    }
                } else if line.starts_with("Vm") {
                    if let Some(colon_pos) = line.find(':') {
                        let memory_type = line[2..colon_pos].to_lowercase();
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<i64>() {
                                let labels = MemoryLabels { memory_type };
                                self.memory_bytes.get_or_create(&labels).set(kb * 1024);
                            }
                        }
                    }
                }
            }
        }
    }
}
