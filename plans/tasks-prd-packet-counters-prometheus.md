# Tasks: Prometheus Packet Counters for NTP Server

## Relevant Files

- `plans/prd-packet-counters-prometheus.md` - PRD containing high level description and implementation guidelines
- `src/metrics/mod.rs` - Main metrics module containing all Prometheus functionality
- `src/metrics/events.rs` - Enums for counter, gauge, and histogram events
- `src/metrics/client_cache.rs` - TTL cache implementation for unique client tracking
- `src/metrics/http_server.rs` - HTTP server for `/metrics` endpoint using hyper
- `src/main.rs` - Command line argument parsing and metrics initialization
- `Cargo.toml` - Dependencies for prometheus_client, moka, and hyper crates
- `tests/metrics_test.rs` - Integration tests for metrics functionality
- `tests/client_cache_test.rs` - Unit tests for client cache functionality

### Notes

- Use `cargo test` to run all tests
- Use `cargo test metrics` to run metrics-specific tests
- Metrics should have zero performance impact when disabled

## Tasks

- [x] 1. Set up Dependencies and Project Structure
  - [x] 1.1 Add prometheus_client, moka, and hyper dependencies to Cargo.toml
  - [x] 1.2 Create src/metrics/ directory structure
  - [x] 1.3 Create src/metrics/mod.rs with public module declarations
  - [x] 1.4 Create placeholder files for events.rs, client_cache.rs, and http_server.rs

- [ ] 2. Implement Core Metrics Module
  - [ ] 2.1 Define PacketEvent, GaugeEvent, and HistogramEvent enums in events.rs
  - [ ] 2.2 Implement MetricsCollector struct with Prometheus registry and metrics
  - [ ] 2.3 Add thread-safe counter increment functions with thread_id parameter
  - [ ] 2.4 Add gauge update functions for first_seen_time and last_seen_time
  - [ ] 2.5 Add histogram recording function for packet sizes
  - [ ] 2.6 Implement TTL cache for unique client tracking in client_cache.rs
  - [ ] 2.7 Add client IP tracking function with automatic gauge updates
  - [ ] 2.8 Ensure all metric operations ignore errors and have zero impact when disabled

- [ ] 3. Add Command Line Interface Support
  - [ ] 3.1 Add --metrics-port parameter to clap configuration in main.rs
  - [ ] 3.2 Add --client-cache-limits parameter with default "64K,1M,16M"
  - [ ] 3.3 Parse client cache limits into separate values for minute/hour/day
  - [ ] 3.4 Pass metrics configuration to MetricsCollector constructor

- [ ] 4. Implement HTTP Metrics Endpoint
  - [ ] 4.1 Create HTTP server using hyper in http_server.rs
  - [ ] 4.2 Implement /metrics endpoint that returns Prometheus format
  - [ ] 4.3 Start HTTP server in separate lower-priority thread
  - [ ] 4.4 Handle server startup and shutdown gracefully

- [ ] 5. Integrate Metrics into Main Application
  - [ ] 5.1 Initialize MetricsCollector in main.rs when --metrics-port is provided
  - [ ] 5.2 Add metric recording calls to NTP server packet handling code
  - [ ] 5.3 Record packet events for all server operations (receive, send, errors)
  - [ ] 5.4 Record client IP addresses for unique client tracking
  - [ ] 5.5 Record packet sizes for histogram metrics
  - [ ] 5.6 Ensure metrics calls are conditional and have no impact when disabled
