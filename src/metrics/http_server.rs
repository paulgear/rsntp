use crate::metrics::MetricsCollector;
use prometheus_client::encoding::text::encode;
use std::sync::Arc;
use std::thread;
use tiny_http::{Server, Response, Header};

pub struct MetricsServer {
    metrics: Arc<MetricsCollector>,
    port: u16,
}

impl MetricsServer {
    pub fn new(metrics: Arc<MetricsCollector>, port: u16) -> Self {
        Self { metrics, port }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server = Server::http(format!("0.0.0.0:{}", self.port))?;
        println!("Metrics server listening on http://0.0.0.0:{}", self.port);

        for request in server.incoming_requests() {
            let metrics = self.metrics.clone();
            thread::spawn(move || {
                let response = match (request.method(), request.url()) {
                    (&tiny_http::Method::Get, "/metrics") => {
                        let mut buffer = String::new();
                        match encode(&mut buffer, &metrics.registry()) {
                            Ok(_) => Response::from_string(buffer)
                                .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4; charset=utf-8"[..]).unwrap()),
                            Err(_) => Response::from_string("Failed to encode metrics").with_status_code(500),
                        }
                    }
                    _ => Response::from_string("Not Found").with_status_code(404),
                };
                let _ = request.respond(response);
            });
        }

        Ok(())
    }
}
