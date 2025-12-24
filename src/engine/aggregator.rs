use crate::engine::{create_snapshot, create_snapshot_with_arrival_rate, Stats};
use crate::types::{RequestResult, RunPhase, StatsSnapshot};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
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
}

impl Aggregator {
    pub fn new(
        duration: Duration,
        result_rx: mpsc::Receiver<RequestResult>,
        snapshot_tx: watch::Sender<StatsSnapshot>,
        warmup_duration: Duration,
        phase_tx: watch::Sender<RunPhase>,
        max_requests: u64,
        cancel_token: CancellationToken,
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
    ) -> Self {
        let in_warmup = !warmup_duration.is_zero();
        if !in_warmup {
            let _ = phase_tx.send(RunPhase::Running);
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
            let dropped = self.dropped_iterations
                .as_ref()
                .map(|d| d.load(Ordering::Relaxed))
                .unwrap_or(0);
            let active = self.vus_active
                .as_ref()
                .map(|v| v.load(Ordering::Relaxed))
                .unwrap_or(0);
            create_snapshot_with_arrival_rate(&self.stats, dropped, active, self.vus_max, self.target_rate)
        } else {
            create_snapshot(&self.stats)
        };
        let _ = self.snapshot_tx.send(snapshot);
    }
}
