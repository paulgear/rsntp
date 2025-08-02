use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::gauge::Gauge;

use prometheus_client::registry::Registry;
use rustc_version_runtime::version;
use sysinfo::{System, Pid};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct InfoLabels {
    rsntp_version: String,
    rust_version: String,
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
    fds: prometheus_client::metrics::family::Family<FdLabels, Gauge>,
    info: prometheus_client::metrics::family::Family<InfoLabels, Gauge>,
    memory_bytes: prometheus_client::metrics::family::Family<MemoryLabels, Gauge>,
    process_start_time: u64,
    runtime_seconds: Gauge,
    system: System,
    threads: Gauge,
}

impl ProcessMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let process_start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let info_labels = InfoLabels {
            rsntp_version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: version().to_string(),
        };

        let threads = Gauge::default();
        let memory_bytes = prometheus_client::metrics::family::Family::default();
        let runtime_seconds = Gauge::default();
        let fds = prometheus_client::metrics::family::Family::default();
        let info = prometheus_client::metrics::family::Family::default();

        info.get_or_create(&info_labels).set(1);

        registry.register("rsntp_info", "Information about rsntp and rust versions", info.clone());
        registry.register("rsntp_process_fds", "Process file descriptors by type", fds.clone());
        registry.register("rsntp_process_memory_bytes", "Process memory usage in bytes by type", memory_bytes.clone());
        registry.register("rsntp_process_runtime_seconds", "Process runtime in seconds", runtime_seconds.clone());
        registry.register("rsntp_process_threads", "Number of OS threads in the process", threads.clone());

        Self {
            threads,
            memory_bytes,
            runtime_seconds,
            fds,
            info,
            system,
            process_start_time,
        }
    }

    pub fn update(&mut self) {
        self.system.refresh_all();

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let runtime = current_time - self.process_start_time;
        self.runtime_seconds.set(runtime as i64);

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
