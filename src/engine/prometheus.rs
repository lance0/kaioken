//! Prometheus metrics export for kaioken
//!
//! Supports two modes:
//! - Push to Pushgateway: POST metrics to a Prometheus Pushgateway
//! - Serve endpoint: Expose /metrics HTTP endpoint for scraping

use crate::types::StatsSnapshot;
use prometheus::{Counter, Encoder, Gauge, Opts, Registry, TextEncoder};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    registry: Registry,
    #[allow(dead_code)]
    target_url: String,

    // Counters (monotonically increasing)
    requests_total: Counter,
    requests_success: Counter,
    requests_failed: Counter,
    bytes_received: Counter,
    dropped_iterations: Counter,

    // Gauges (point-in-time values)
    rps: Gauge,
    error_rate: Gauge,
    latency_p50: Gauge,
    latency_p95: Gauge,
    latency_p99: Gauge,
    latency_p999: Gauge,
    vus_active: Gauge,
    vus_max: Gauge,

    // Track previous values for counter deltas
    prev_total: RwLock<u64>,
    prev_success: RwLock<u64>,
    prev_failed: RwLock<u64>,
    prev_bytes: RwLock<u64>,
    prev_dropped: RwLock<u64>,
}

impl PrometheusExporter {
    /// Create a new PrometheusExporter with metrics registered
    pub fn new(target_url: &str) -> Self {
        let registry = Registry::new();

        // Create counters
        let requests_total = Counter::with_opts(
            Opts::new("kaioken_requests_total", "Total HTTP requests made")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let requests_success = Counter::with_opts(
            Opts::new("kaioken_requests_success_total", "Successful HTTP requests")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let requests_failed = Counter::with_opts(
            Opts::new("kaioken_requests_failed_total", "Failed HTTP requests")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let bytes_received = Counter::with_opts(
            Opts::new("kaioken_bytes_received_total", "Total bytes received")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let dropped_iterations = Counter::with_opts(
            Opts::new(
                "kaioken_dropped_iterations_total",
                "Dropped iterations (arrival rate mode)",
            )
            .const_label("job", "kaioken")
            .const_label("instance", target_url),
        )
        .unwrap();

        // Create gauges
        let rps = Gauge::with_opts(
            Opts::new("kaioken_rps", "Current requests per second")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let error_rate = Gauge::with_opts(
            Opts::new("kaioken_error_rate", "Current error rate (0.0-1.0)")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let latency_p50 = Gauge::with_opts(
            Opts::new("kaioken_latency_p50_ms", "50th percentile latency in ms")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let latency_p95 = Gauge::with_opts(
            Opts::new("kaioken_latency_p95_ms", "95th percentile latency in ms")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let latency_p99 = Gauge::with_opts(
            Opts::new("kaioken_latency_p99_ms", "99th percentile latency in ms")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let latency_p999 = Gauge::with_opts(
            Opts::new("kaioken_latency_p999_ms", "99.9th percentile latency in ms")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let vus_active = Gauge::with_opts(
            Opts::new("kaioken_vus_active", "Currently active virtual users")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        let vus_max = Gauge::with_opts(
            Opts::new("kaioken_vus_max", "Maximum virtual users configured")
                .const_label("job", "kaioken")
                .const_label("instance", target_url),
        )
        .unwrap();

        // Register all metrics
        registry.register(Box::new(requests_total.clone())).unwrap();
        registry
            .register(Box::new(requests_success.clone()))
            .unwrap();
        registry
            .register(Box::new(requests_failed.clone()))
            .unwrap();
        registry.register(Box::new(bytes_received.clone())).unwrap();
        registry
            .register(Box::new(dropped_iterations.clone()))
            .unwrap();
        registry.register(Box::new(rps.clone())).unwrap();
        registry.register(Box::new(error_rate.clone())).unwrap();
        registry.register(Box::new(latency_p50.clone())).unwrap();
        registry.register(Box::new(latency_p95.clone())).unwrap();
        registry.register(Box::new(latency_p99.clone())).unwrap();
        registry.register(Box::new(latency_p999.clone())).unwrap();
        registry.register(Box::new(vus_active.clone())).unwrap();
        registry.register(Box::new(vus_max.clone())).unwrap();

        Self {
            registry,
            target_url: target_url.to_string(),
            requests_total,
            requests_success,
            requests_failed,
            bytes_received,
            dropped_iterations,
            rps,
            error_rate,
            latency_p50,
            latency_p95,
            latency_p99,
            latency_p999,
            vus_active,
            vus_max,
            prev_total: RwLock::new(0),
            prev_success: RwLock::new(0),
            prev_failed: RwLock::new(0),
            prev_bytes: RwLock::new(0),
            prev_dropped: RwLock::new(0),
        }
    }

    /// Update all metrics from a snapshot
    pub async fn update(&self, snapshot: &StatsSnapshot) {
        // Update counters with deltas (counters can only increase)
        let mut prev_total = self.prev_total.write().await;
        if snapshot.total_requests > *prev_total {
            self.requests_total
                .inc_by((snapshot.total_requests - *prev_total) as f64);
            *prev_total = snapshot.total_requests;
        }

        let mut prev_success = self.prev_success.write().await;
        if snapshot.successful > *prev_success {
            self.requests_success
                .inc_by((snapshot.successful - *prev_success) as f64);
            *prev_success = snapshot.successful;
        }

        let mut prev_failed = self.prev_failed.write().await;
        if snapshot.failed > *prev_failed {
            self.requests_failed
                .inc_by((snapshot.failed - *prev_failed) as f64);
            *prev_failed = snapshot.failed;
        }

        let mut prev_bytes = self.prev_bytes.write().await;
        if snapshot.bytes_received > *prev_bytes {
            self.bytes_received
                .inc_by((snapshot.bytes_received - *prev_bytes) as f64);
            *prev_bytes = snapshot.bytes_received;
        }

        let mut prev_dropped = self.prev_dropped.write().await;
        if snapshot.dropped_iterations > *prev_dropped {
            self.dropped_iterations
                .inc_by((snapshot.dropped_iterations - *prev_dropped) as f64);
            *prev_dropped = snapshot.dropped_iterations;
        }

        // Update gauges (point-in-time values)
        self.rps.set(snapshot.requests_per_sec);
        self.error_rate.set(snapshot.error_rate);
        self.latency_p50
            .set(snapshot.latency_p50_us as f64 / 1000.0);
        self.latency_p95
            .set(snapshot.latency_p95_us as f64 / 1000.0);
        self.latency_p99
            .set(snapshot.latency_p99_us as f64 / 1000.0);
        self.latency_p999
            .set(snapshot.latency_p999_us as f64 / 1000.0);
        self.vus_active.set(snapshot.vus_active as f64);
        self.vus_max.set(snapshot.vus_max as f64);
    }

    /// Encode all metrics in Prometheus text format
    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    /// Get the target URL this exporter is tracking
    #[allow(dead_code)]
    pub fn target_url(&self) -> &str {
        &self.target_url
    }
}

/// Push metrics to a Prometheus Pushgateway
pub async fn push_to_gateway(url: &str, metrics: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let push_url = format!("{}/metrics/job/kaioken", url.trim_end_matches('/'));

    let response = client
        .post(&push_url)
        .body(metrics.to_string())
        .header("Content-Type", "text/plain; charset=utf-8")
        .send()
        .await
        .map_err(|e| format!("Failed to push to Pushgateway: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Pushgateway returned error status: {}",
            response.status()
        ));
    }

    Ok(())
}

/// Serve a /metrics endpoint for Prometheus scraping
pub async fn serve_metrics_endpoint(
    port: u16,
    exporter: Arc<PrometheusExporter>,
    cancel_token: tokio_util::sync::CancellationToken,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(
                "Failed to bind Prometheus metrics endpoint on {}: {}",
                addr,
                e
            );
            return;
        }
    };

    tracing::info!(
        "Prometheus metrics endpoint listening on http://{}/metrics",
        addr
    );

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::debug!("Prometheus metrics endpoint shutting down");
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut socket, _)) => {
                        let exporter = exporter.clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; 1024];
                            if socket.read(&mut buf).await.is_ok() {
                                let request = String::from_utf8_lossy(&buf);

                                // Simple HTTP parsing - check if it's a GET /metrics request
                                if request.starts_with("GET /metrics") || request.starts_with("GET / ") {
                                    let body = exporter.encode();
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                        body.len(),
                                        body
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                } else if request.starts_with("GET /health") {
                                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                                    let _ = socket.write_all(response.as_bytes()).await;
                                } else {
                                    let response = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found";
                                    let _ = socket.write_all(response.as_bytes()).await;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to accept connection: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_creation() {
        let exporter = PrometheusExporter::new("https://example.com/api");
        assert_eq!(exporter.target_url(), "https://example.com/api");
    }

    #[test]
    fn test_metrics_encoding() {
        let exporter = PrometheusExporter::new("https://example.com");
        let encoded = exporter.encode();

        // Should contain all our metric names
        assert!(encoded.contains("kaioken_requests_total"));
        assert!(encoded.contains("kaioken_rps"));
        assert!(encoded.contains("kaioken_latency_p99_ms"));
        assert!(encoded.contains("kaioken_vus_active"));
    }

    #[tokio::test]
    async fn test_metrics_update() {
        use std::collections::HashMap;
        use std::time::Duration;

        let exporter = PrometheusExporter::new("https://example.com");

        let snapshot = StatsSnapshot {
            elapsed: Duration::from_secs(10),
            total_requests: 1000,
            successful: 990,
            failed: 10,
            bytes_received: 500000,
            rolling_rps: 100.0,
            requests_per_sec: 100.0,
            error_rate: 0.01,
            latency_min_us: 1000,
            latency_max_us: 100000,
            latency_mean_us: 5000.0,
            latency_stddev_us: 2000.0,
            latency_p50_us: 4000,
            latency_p75_us: 6000,
            latency_p90_us: 8000,
            latency_p95_us: 10000,
            latency_p99_us: 20000,
            latency_p999_us: 50000,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            timeline: vec![],
            vus_active: 50,
            vus_max: 100,
            target_rate: 0,
            dropped_iterations: 5,
            latency_correction_enabled: false,
            corrected_latency_min_us: None,
            corrected_latency_max_us: None,
            corrected_latency_mean_us: None,
            corrected_latency_p50_us: None,
            corrected_latency_p75_us: None,
            corrected_latency_p90_us: None,
            corrected_latency_p95_us: None,
            corrected_latency_p99_us: None,
            corrected_latency_p999_us: None,
            queue_time_mean_us: None,
            queue_time_p99_us: None,
            total_queue_time_us: 0,
            is_websocket: false,
            ws_messages_sent: 0,
            ws_messages_received: 0,
            ws_bytes_sent: 0,
            ws_bytes_received: 0,
            ws_connections_active: 0,
            ws_connections_established: 0,
            ws_connection_errors: 0,
            ws_disconnects: 0,
            ws_messages_per_sec: 0.0,
            ws_rolling_mps: 0.0,
            ws_error_rate: 0.0,
            ws_errors: HashMap::new(),
            ws_latency_min_us: 0,
            ws_latency_max_us: 0,
            ws_latency_mean_us: 0.0,
            ws_latency_stddev_us: 0.0,
            ws_latency_p50_us: 0,
            ws_latency_p95_us: 0,
            ws_latency_p99_us: 0,
            ws_connect_time_mean_us: 0.0,
            ws_connect_time_p99_us: 0,
            check_stats: HashMap::new(),
            overall_check_pass_rate: None,
        };

        exporter.update(&snapshot).await;

        let encoded = exporter.encode();
        // After update, metrics should have values
        // Prometheus format: metric{labels} value
        assert!(encoded.contains("kaioken_rps{"));
        assert!(encoded.contains("} 100")); // rps should be 100
        assert!(encoded.contains("kaioken_latency_p99_ms{"));
        assert!(encoded.contains("} 20")); // 20000us = 20ms
    }
}
