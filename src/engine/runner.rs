use crate::engine::aggregator::Aggregator;
use crate::engine::worker::Worker;
use crate::engine::Stats;
use crate::http::create_client;
use crate::types::{LoadConfig, RequestResult, RunState, StatsSnapshot};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

const RESULT_CHANNEL_SIZE: usize = 10_000;

pub struct Engine {
    config: LoadConfig,
    cancel_token: CancellationToken,
    state_tx: watch::Sender<RunState>,
    snapshot_rx: watch::Receiver<StatsSnapshot>,
    snapshot_tx: watch::Sender<StatsSnapshot>,
}

impl Engine {
    pub fn new(config: LoadConfig) -> Self {
        let cancel_token = CancellationToken::new();
        let (state_tx, _) = watch::channel(RunState::Initializing);
        let (snapshot_tx, snapshot_rx) = watch::channel(StatsSnapshot::default());

        Self {
            config,
            cancel_token,
            state_tx,
            snapshot_rx,
            snapshot_tx,
        }
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn snapshot_rx(&self) -> watch::Receiver<StatsSnapshot> {
        self.snapshot_rx.clone()
    }

    pub fn state_rx(&self) -> watch::Receiver<RunState> {
        self.state_tx.subscribe()
    }

    pub async fn run(self) -> Result<Stats, String> {
        let client = create_client(
            self.config.concurrency,
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
        )
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let (result_tx, result_rx) = mpsc::channel::<RequestResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);

        let aggregator = Aggregator::new(self.config.duration, result_rx, self.snapshot_tx);
        let aggregator_handle = tokio::spawn(aggregator.run());

        let mut worker_handles = Vec::with_capacity(self.config.concurrency as usize);

        for id in 0..self.config.concurrency {
            let worker = Worker::new(
                id,
                client.clone(),
                self.config.url.clone(),
                self.config.method.clone(),
                self.config.headers.clone(),
                self.config.body.clone(),
                result_tx.clone(),
                self.cancel_token.clone(),
            );
            worker_handles.push(tokio::spawn(worker.run()));
        }

        drop(result_tx);

        let cancel_token = self.cancel_token.clone();
        let duration = self.config.duration;

        tokio::select! {
            _ = sleep(duration) => {
                tracing::info!("Duration elapsed, stopping workers");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
        }

        for handle in worker_handles {
            let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
        }

        let stats = aggregator_handle
            .await
            .map_err(|e| format!("Aggregator task failed: {}", e))?;

        let final_state = if self.cancel_token.is_cancelled() {
            RunState::Cancelled
        } else {
            RunState::Completed
        };
        let _ = self.state_tx.send(final_state);

        Ok(stats)
    }
}
