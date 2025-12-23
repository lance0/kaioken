use crate::engine::{create_snapshot, Stats};
use crate::types::{RequestResult, StatsSnapshot};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

pub struct Aggregator {
    stats: Stats,
    result_rx: mpsc::Receiver<RequestResult>,
    snapshot_tx: watch::Sender<StatsSnapshot>,
}

impl Aggregator {
    pub fn new(
        duration: Duration,
        result_rx: mpsc::Receiver<RequestResult>,
        snapshot_tx: watch::Sender<StatsSnapshot>,
    ) -> Self {
        Self {
            stats: Stats::new(duration),
            result_rx,
            snapshot_tx,
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
                            self.stats.record(&req_result);
                        }
                        None => {
                            self.send_snapshot();
                            break;
                        }
                    }
                }

                _ = snapshot_interval.tick() => {
                    self.send_snapshot();
                }
            }
        }

        self.stats
    }

    fn send_snapshot(&self) {
        let snapshot = create_snapshot(&self.stats);
        let _ = self.snapshot_tx.send(snapshot);
    }
}
