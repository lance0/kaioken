use crate::engine::{create_snapshot, Stats};
use crate::types::{RequestResult, RunPhase, StatsSnapshot};
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
        let snapshot = create_snapshot(&self.stats);
        let _ = self.snapshot_tx.send(snapshot);
    }
}
