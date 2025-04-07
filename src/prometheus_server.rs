use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use prometheus::{Encoder, TextEncoder};
use tokio::runtime::Runtime;

use crate::metrics::Metrics;

async fn metrics_handler(metrics: Arc<Metrics>, _req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry().gather();
    let mut buffer = vec![];

    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap();

    Ok(response)
}

pub fn start_prometheus_server(addr_str: String, metrics: Arc<Metrics>) {
    // Parse the address
    let addr: SocketAddr = match addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse Prometheus server address: {}", e);
            return;
        }
    };

    // Create a Tokio runtime
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create Tokio runtime: {}", e);
            return;
        }
    };

    // Start the server
    rt.block_on(async {
        let metrics_clone = metrics.clone();

        let make_svc = make_service_fn(move |_conn| {
            let metrics = metrics_clone.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    metrics_handler(metrics.clone(), req)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        println!("Prometheus metrics server listening on http://{}", addr);

        if let Err(e) = server.await {
            eprintln!("Prometheus server error: {}", e);
        }
    });
}
