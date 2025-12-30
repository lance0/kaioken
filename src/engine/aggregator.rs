use crate::engine::prometheus::{PrometheusExporter, push_to_gateway, serve_metrics_endpoint};
use crate::engine::{Stats, create_snapshot, create_snapshot_with_arrival_rate};
use crate::types::{PrometheusConfig, RequestResult, RunPhase, StatsSnapshot};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

pub struct Aggregator {
    stats: Stats,
    result_rx: mpsc::Receiver<RequestResult>,
    snapshot_tx: watch::Sender<StatsSnapshot>,
    warmup_duration: Duration,
    phase_tx: watch::Sender<RunPhase>,
    start_time: Instant,
    warmup_complete: bool,
    max_requests: u64,
    cancel_token: CancellationToken,
    // Arrival rate metrics (optional)
    dropped_iterations: Option<Arc<AtomicU64>>,
    vus_active: Option<Arc<AtomicU32>>,
    vus_max: u32,
    target_rate: u32,
    // SQLite logging (optional)
    sqlite_conn: Option<Connection>,
    // Prometheus metrics export (optional)
    prometheus_exporter: Option<Arc<PrometheusExporter>>,
    prometheus_config: Option<PrometheusConfig>,
}

impl Aggregator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        duration: Duration,
        result_rx: mpsc::Receiver<RequestResult>,
        snapshot_tx: watch::Sender<StatsSnapshot>,
        warmup_duration: Duration,
        phase_tx: watch::Sender<RunPhase>,
        max_requests: u64,
        cancel_token: CancellationToken,
        db_url: Option<PathBuf>,
        prometheus: Option<PrometheusConfig>,
        target_url: &str,
    ) -> Self {
        Self::with_arrival_rate_metrics(
            duration,
            result_rx,
            snapshot_tx,
            warmup_duration,
            phase_tx,
            max_requests,
            cancel_token,
            None,
            None,
            0,
            0,
            db_url,
            prometheus,
            target_url,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_arrival_rate_metrics(
        duration: Duration,
        result_rx: mpsc::Receiver<RequestResult>,
        snapshot_tx: watch::Sender<StatsSnapshot>,
        warmup_duration: Duration,
        phase_tx: watch::Sender<RunPhase>,
        max_requests: u64,
        cancel_token: CancellationToken,
        dropped_iterations: Option<Arc<AtomicU64>>,
        vus_active: Option<Arc<AtomicU32>>,
        vus_max: u32,
        target_rate: u32,
        db_url: Option<PathBuf>,
        prometheus: Option<PrometheusConfig>,
        target_url: &str,
    ) -> Self {
        let in_warmup = !warmup_duration.is_zero();
        if !in_warmup {
            let _ = phase_tx.send(RunPhase::Running);
        }

        // Initialize SQLite connection if db_url is provided
        let sqlite_conn = db_url.and_then(|path| match init_sqlite_db(&path) {
            Ok(conn) => Some(conn),
            Err(e) => {
                tracing::warn!("Failed to initialize SQLite database: {}", e);
                None
            }
        });

        // Initialize Prometheus exporter if configured
        let prometheus_exporter = prometheus
            .as_ref()
            .map(|_| Arc::new(PrometheusExporter::new(target_url)));

        // Spawn metrics endpoint server if configured
        if let Some(PrometheusConfig::Endpoint { port }) = &prometheus
            && let Some(ref exporter) = prometheus_exporter
        {
            let exporter_clone = exporter.clone();
            let cancel_clone = cancel_token.clone();
            let port = *port;
            tokio::spawn(async move {
                serve_metrics_endpoint(port, exporter_clone, cancel_clone).await;
            });
        }

        Self {
            stats: Stats::new(duration),
            result_rx,
            snapshot_tx,
            warmup_duration,
            phase_tx,
            start_time: Instant::now(),
            warmup_complete: !in_warmup,
            max_requests,
            cancel_token,
            dropped_iterations,
            vus_active,
            vus_max,
            target_rate,
            sqlite_conn,
            prometheus_exporter,
            prometheus_config: prometheus,
        }
    }

    pub async fn run(mut self) -> Stats {
        let mut snapshot_interval = tokio::time::interval(Duration::from_millis(100));
        snapshot_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;

                result = self.result_rx.recv() => {
                    match result {
                        Some(req_result) => {
                            self.check_warmup_complete();
                            if self.warmup_complete {
                                self.stats.record(&req_result);

                                // Check max_requests limit
                                if self.max_requests > 0
                                    && self.stats.total_requests() >= self.max_requests
                                {
                                    tracing::info!(
                                        "Max requests ({}) reached, stopping",
                                        self.max_requests
                                    );
                                    self.cancel_token.cancel();
                                }
                            }
                        }
                        None => {
                            self.send_snapshot();
                            break;
                        }
                    }
                }

                _ = snapshot_interval.tick() => {
                    self.check_warmup_complete();
                    self.send_snapshot();
                }
            }
        }

        self.stats
    }

    fn check_warmup_complete(&mut self) {
        if !self.warmup_complete && self.start_time.elapsed() >= self.warmup_duration {
            self.warmup_complete = true;
            self.stats.reset();
            let _ = self.phase_tx.send(RunPhase::Running);
            tracing::info!("Warmup complete, starting measurement");
        }
    }

    fn send_snapshot(&self) {
        let snapshot = if self.dropped_iterations.is_some() || self.vus_active.is_some() {
            let dropped = self
                .dropped_iterations
                .as_ref()
                .map(|d| d.load(Ordering::Relaxed))
                .unwrap_or(0);
            let active = self
                .vus_active
                .as_ref()
                .map(|v| v.load(Ordering::Relaxed))
                .unwrap_or(0);
            create_snapshot_with_arrival_rate(
                &self.stats,
                dropped,
                active,
                self.vus_max,
                self.target_rate,
            )
        } else {
            create_snapshot(&self.stats)
        };

        // Log snapshot to SQLite if configured
        if let Some(ref conn) = self.sqlite_conn
            && let Err(e) = log_snapshot_to_sqlite(conn, &snapshot)
        {
            tracing::warn!("Failed to log snapshot to SQLite: {}", e);
        }

        // Update and export Prometheus metrics if configured
        if let Some(ref exporter) = self.prometheus_exporter {
            // Update metrics synchronously (uses internal locks)
            let exporter_clone = exporter.clone();
            let snapshot_clone = snapshot.clone();
            tokio::spawn(async move {
                exporter_clone.update(&snapshot_clone).await;
            });

            // Push to Pushgateway if in push mode (fire-and-forget)
            if let Some(PrometheusConfig::Pushgateway { ref url }) = self.prometheus_config {
                let metrics = exporter.encode();
                let url = url.clone();
                tokio::spawn(async move {
                    if let Err(e) = push_to_gateway(&url, &metrics).await {
                        tracing::warn!("Failed to push to Prometheus Pushgateway: {}", e);
                    }
                });
            }
        }

        let _ = self.snapshot_tx.send(snapshot);
    }
}

/// Initialize SQLite database with the required schema
fn init_sqlite_db(path: &std::path::Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ms INTEGER NOT NULL,
            elapsed_secs REAL NOT NULL,
            total_requests INTEGER NOT NULL,
            successful INTEGER NOT NULL,
            failed INTEGER NOT NULL,
            rps REAL NOT NULL,
            latency_p50_us INTEGER NOT NULL,
            latency_p95_us INTEGER NOT NULL,
            latency_p99_us INTEGER NOT NULL,
            latency_p999_us INTEGER NOT NULL,
            error_rate REAL NOT NULL,
            bytes_received INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_elapsed ON snapshots(elapsed_secs);",
    )?;

    Ok(conn)
}

/// Log a snapshot to SQLite database
fn log_snapshot_to_sqlite(
    conn: &Connection,
    snapshot: &StatsSnapshot,
) -> Result<(), rusqlite::Error> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO snapshots (
            timestamp_ms, elapsed_secs, total_requests, successful, failed,
            rps, latency_p50_us, latency_p95_us, latency_p99_us, latency_p999_us,
            error_rate, bytes_received
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            timestamp_ms,
            snapshot.elapsed.as_secs_f64(),
            snapshot.total_requests as i64,
            snapshot.successful as i64,
            snapshot.failed as i64,
            snapshot.requests_per_sec,
            snapshot.latency_p50_us as i64,
            snapshot.latency_p95_us as i64,
            snapshot.latency_p99_us as i64,
            snapshot.latency_p999_us as i64,
            snapshot.error_rate,
            snapshot.bytes_received as i64,
        ],
    )?;

    Ok(())
}
