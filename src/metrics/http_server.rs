use crate::metrics::MetricsCollector;
use prometheus_client::encoding::text::encode;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

pub struct MetricsServer {
    metrics: Arc<MetricsCollector>,
    port: u16,
}

impl MetricsServer {
    pub fn new(metrics: Arc<MetricsCollector>, port: u16) -> Self {
        Self { metrics, port }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        println!("Metrics server listening on http://0.0.0.0:{}", self.port);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let metrics = self.metrics.clone();
                    thread::spawn(move || {
                        let _ = handle_request(stream, metrics);
                    });
                }
                Err(_) => continue,
            }
        }

        Ok(())
    }
}

fn handle_request(
    mut stream: TcpStream,
    metrics: Arc<MetricsCollector>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_response(&mut stream, 400, "Bad Request", "")?;
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    match (method, path) {
        ("GET", "/metrics") => {
            if !metrics.is_enabled() {
                send_response(&mut stream, 503, "Service Unavailable", "Metrics disabled")?;
                return Ok(());
            }

            let mut buffer = String::new();
            if encode(&mut buffer, &metrics.registry()).is_err() {
                send_response(&mut stream, 500, "Internal Server Error", "Failed to encode metrics")?;
                return Ok(());
            }

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                buffer.len(),
                buffer
            );
            stream.write_all(response.as_bytes())?;
        }
        _ => {
            send_response(&mut stream, 404, "Not Found", "Not Found")?;
        }
    }

    Ok(())
}

fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
        status_code,
        status_text,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}
