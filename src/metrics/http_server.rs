use crate::metrics::MetricsCollector;
use prometheus_client::encoding::text::encode;
use std::sync::Arc;
use std::thread;
use tiny_http::{Server, Response, Header};

#[derive(Clone)]
pub struct MetricsServer {
    metrics: Arc<MetricsCollector>,
    port: u16,
}

impl MetricsServer {
    pub fn new(metrics: Arc<MetricsCollector>, port: u16) -> Self {
        Self { metrics, port }
    }

    fn handle_clients(&self) -> Response<std::io::Cursor<Vec<u8>>> {
        Response::from_string("")
            .with_header("Content-Type: text/plain; version=0.0.4; charset=utf-8".parse::<Header>().unwrap())
    }

    fn handle_metrics(&self) -> Response<std::io::Cursor<Vec<u8>>> {
        self.metrics.update_unique_clients();
        let mut buffer = String::new();
        match encode(&mut buffer, &self.metrics.registry()) {
            Ok(_) => Response::from_string(buffer)
                .with_header("Content-Type: text/plain; version=0.0.4; charset=utf-8".parse::<Header>().unwrap()),
            Err(_) => Response::from_string("Failed to encode metrics").with_status_code(500),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server = Server::http(format!("0.0.0.0:{}", self.port))?;
        println!("Metrics server listening on http://0.0.0.0:{}", self.port);

        for request in server.incoming_requests() {
            let metrics_server = self.clone();
            thread::spawn(move || {
                let response = match (request.method(), request.url()) {
                    (&tiny_http::Method::Get, "/clients") => metrics_server.handle_clients(),
                    (&tiny_http::Method::Get, "/metrics") => metrics_server.handle_metrics(),
                    _ => Response::from_string("Not Found").with_status_code(404),
                };
                let _ = request.respond(response);
            });
        }

        Ok(())
    }
}
