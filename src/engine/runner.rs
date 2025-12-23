use crate::engine::aggregator::Aggregator;
use crate::engine::scheduler::{RampUpScheduler, RateLimiter};
use crate::engine::worker::Worker;
use crate::engine::Stats;
use crate::http::create_client;
use crate::types::{LoadConfig, RequestResult, RunPhase, RunState, StatsSnapshot};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

const RESULT_CHANNEL_SIZE: usize = 10_000;

pub struct Engine {
    config: LoadConfig,
    cancel_token: CancellationToken,
    state_tx: watch::Sender<RunState>,
    phase_tx: watch::Sender<RunPhase>,
    snapshot_rx: watch::Receiver<StatsSnapshot>,
    snapshot_tx: watch::Sender<StatsSnapshot>,
}

impl Engine {
    pub fn new(config: LoadConfig) -> Self {
        let cancel_token = CancellationToken::new();
        let (state_tx, _) = watch::channel(RunState::Initializing);
        let (phase_tx, _) = watch::channel(RunPhase::Warmup);
        let (snapshot_tx, snapshot_rx) = watch::channel(StatsSnapshot::default());

        Self {
            config,
            cancel_token,
            state_tx,
            phase_tx,
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

    pub fn phase_rx(&self) -> watch::Receiver<RunPhase> {
        self.phase_tx.subscribe()
    }

    pub async fn run(self) -> Result<Stats, String> {
        let client = create_client(
            self.config.concurrency,
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
            self.config.http2,
        )
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Set up rate limiter if configured
        let rate_limiter = if self.config.rate > 0 {
            let limiter = RateLimiter::new(self.config.rate);
            let refiller = limiter.clone();
            tokio::spawn(async move { refiller.run_refiller().await });
            Some(limiter)
        } else {
            None
        };

        // Set up ramp-up scheduler
        let ramp_scheduler = RampUpScheduler::new(self.config.concurrency, self.config.ramp_up);
        let ramp_permits = ramp_scheduler.permits();

        // Start ramp-up task
        tokio::spawn(ramp_scheduler.run());

        let (result_tx, result_rx) = mpsc::channel::<RequestResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);

        // Create aggregator with combined duration (warmup + actual)
        let total_duration = self.config.warmup + self.config.duration;
        let aggregator = Aggregator::new(
            total_duration,
            result_rx,
            self.snapshot_tx.clone(),
            self.config.warmup,
            self.phase_tx.clone(),
            self.config.max_requests,
            self.cancel_token.clone(),
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Spawn workers
        let mut worker_handles = Vec::with_capacity(self.config.concurrency as usize);
        let scenarios = Arc::new(self.config.scenarios.clone());

        for id in 0..self.config.concurrency {
            let worker = Worker::new(
                id,
                client.clone(),
                self.config.url.clone(),
                self.config.method.clone(),
                self.config.headers.clone(),
                self.config.body.clone(),
                scenarios.clone(),
                result_tx.clone(),
                self.cancel_token.clone(),
                rate_limiter.clone(),
                ramp_permits.clone(),
            );
            worker_handles.push(tokio::spawn(worker.run()));
        }

        drop(result_tx);

        let cancel_token = self.cancel_token.clone();

        // Wait for warmup + duration
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping workers");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
        }

        // Wait for workers to finish (with timeout)
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
