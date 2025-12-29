use crate::engine::WsStats;
use crate::types::{RunPhase, StatsSnapshot, WsMessageResult};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub struct WsAggregator {
    stats: WsStats,
    duration: Duration,
    result_rx: mpsc::Receiver<WsMessageResult>,
    snapshot_tx: watch::Sender<StatsSnapshot>,
    warmup_duration: Duration,
    phase_tx: watch::Sender<RunPhase>,
    start_time: Instant,
    warmup_complete: bool,
    cancel_token: CancellationToken,
    connections_active: u32,
}

impl WsAggregator {
    pub fn new(
        duration: Duration,
        result_rx: mpsc::Receiver<WsMessageResult>,
        snapshot_tx: watch::Sender<StatsSnapshot>,
        warmup_duration: Duration,
        phase_tx: watch::Sender<RunPhase>,
        cancel_token: CancellationToken,
        connections_active: u32,
    ) -> Self {
        let in_warmup = !warmup_duration.is_zero();
        if !in_warmup {
            let _ = phase_tx.send(RunPhase::Running);
        }

        Self {
            stats: WsStats::new(),
            duration,
            result_rx,
            snapshot_tx,
            warmup_duration,
            phase_tx,
            start_time: Instant::now(),
            warmup_complete: !in_warmup,
            cancel_token,
            connections_active,
        }
    }

    pub async fn run(mut self) -> WsStats {
        let mut snapshot_interval = tokio::time::interval(Duration::from_millis(100));
        snapshot_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;

                result = self.result_rx.recv() => {
                    match result {
                        Some(ws_result) => {
                            self.check_warmup_complete();
                            if self.warmup_complete {
                                self.stats.record_message(&ws_result);
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
        let snapshot = self.create_ws_snapshot();
        let _ = self.snapshot_tx.send(snapshot);
    }

    fn create_ws_snapshot(&self) -> StatsSnapshot {
        let elapsed = self.stats.elapsed();

        StatsSnapshot {
            elapsed,
            is_websocket: true,

            // HTTP fields (zeroed for WS tests)
            total_requests: 0,
            successful: 0,
            failed: 0,
            bytes_received: 0,
            rolling_rps: 0.0,
            requests_per_sec: 0.0,
            error_rate: 0.0,
            latency_min_us: 0,
            latency_max_us: 0,
            latency_mean_us: 0.0,
            latency_stddev_us: 0.0,
            latency_p50_us: 0,
            latency_p75_us: 0,
            latency_p90_us: 0,
            latency_p95_us: 0,
            latency_p99_us: 0,
            latency_p999_us: 0,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            timeline: Vec::new(),
            check_stats: HashMap::new(),
            overall_check_pass_rate: None,
            dropped_iterations: 0,
            vus_active: 0,
            vus_max: 0,
            target_rate: 0,

            // Latency correction fields (not used for WS)
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

            // WebSocket fields
            ws_messages_sent: self.stats.total_messages_sent,
            ws_messages_received: self.stats.total_messages_received,
            ws_bytes_sent: self.stats.total_bytes_sent,
            ws_bytes_received: self.stats.total_bytes_received,
            ws_connections_active: self.connections_active,
            ws_connections_established: self.stats.connections_established,
            ws_connection_errors: self.stats.connection_errors,
            ws_disconnects: self.stats.disconnects,
            ws_messages_per_sec: self.stats.messages_per_sec(),
            ws_rolling_mps: self.stats.rolling_messages_per_sec(),
            ws_error_rate: self.stats.error_rate(),
            ws_errors: self.stats.errors.clone(),
            ws_latency_min_us: self.stats.message_latency_min(),
            ws_latency_max_us: self.stats.message_latency_max(),
            ws_latency_mean_us: self.stats.message_latency_mean(),
            ws_latency_stddev_us: self.stats.message_latency_stddev(),
            ws_latency_p50_us: self.stats.message_latency_percentile(50.0),
            ws_latency_p95_us: self.stats.message_latency_percentile(95.0),
            ws_latency_p99_us: self.stats.message_latency_percentile(99.0),
            ws_connect_time_mean_us: self.stats.connect_time_mean(),
            ws_connect_time_p99_us: self.stats.connect_time_percentile(99.0),
        }
    }
}
