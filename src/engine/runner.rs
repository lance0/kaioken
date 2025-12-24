use crate::engine::aggregator::Aggregator;
use crate::engine::arrival_rate::ArrivalRateExecutor;
use crate::engine::scheduler::{RampUpScheduler, RateLimiter, StageInfo, StagesScheduler};
use crate::engine::thresholds::evaluate_thresholds;
use crate::engine::worker::{CheckResult, Worker};
use crate::engine::Stats;
use crate::http::create_client;
use crate::types::{LoadConfig, RequestResult, RunPhase, RunState, StatsSnapshot, Threshold};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, Semaphore};
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
    stage_info_rx: Option<watch::Receiver<StageInfo>>,
    threshold_failed: Arc<AtomicBool>,
    check_stats: Arc<std::sync::Mutex<HashMap<String, (u64, u64)>>>, // (passed, total)
    // Arrival rate metrics
    dropped_iterations: Arc<AtomicU64>,
    vus_active: Arc<AtomicU32>,
    vus_max: Arc<AtomicU32>,
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
            stage_info_rx: None,
            threshold_failed: Arc::new(AtomicBool::new(false)),
            check_stats: Arc::new(std::sync::Mutex::new(HashMap::new())),
            dropped_iterations: Arc::new(AtomicU64::new(0)),
            vus_active: Arc::new(AtomicU32::new(0)),
            vus_max: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Check if arrival rate mode is enabled
    fn is_arrival_rate_mode(&self) -> bool {
        self.config.arrival_rate.is_some()
            || self.config.stages.iter().any(|s| s.target_rate.is_some())
    }

    pub fn dropped_iterations(&self) -> u64 {
        self.dropped_iterations.load(Ordering::Relaxed)
    }

    pub fn vus_active(&self) -> u32 {
        self.vus_active.load(Ordering::Relaxed)
    }

    pub fn vus_max(&self) -> u32 {
        self.vus_max.load(Ordering::Relaxed)
    }

    pub fn threshold_failed(&self) -> bool {
        self.threshold_failed.load(Ordering::Relaxed)
    }

    pub fn threshold_failed_flag(&self) -> Arc<AtomicBool> {
        self.threshold_failed.clone()
    }

    pub fn check_stats(&self) -> HashMap<String, (u64, u64)> {
        self.check_stats.lock().unwrap().clone()
    }

    pub fn check_stats_ref(&self) -> Arc<std::sync::Mutex<HashMap<String, (u64, u64)>>> {
        self.check_stats.clone()
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

    pub fn stage_info_rx(&self) -> Option<watch::Receiver<StageInfo>> {
        self.stage_info_rx.clone()
    }

    pub async fn run(self) -> Result<Stats, String> {
        // Check if we should use arrival rate mode
        if self.is_arrival_rate_mode() {
            return self.run_arrival_rate_mode().await;
        }

        // Otherwise, use constant VUs mode
        self.run_constant_vus_mode().await
    }

    async fn run_arrival_rate_mode(self) -> Result<Stats, String> {
        let client = create_client(
            self.config.max_vus.unwrap_or(100),
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
            self.config.http2,
            self.config.cookie_jar,
        )
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let arrival_rate = self.config.arrival_rate.unwrap_or(10);
        let max_vus = self.config.max_vus.unwrap_or(100);
        let pre_allocated_vus = (arrival_rate / 10).max(1).min(max_vus); // Start with ~10% of rate

        self.vus_max.store(max_vus, Ordering::Relaxed);

        let (result_tx, result_rx) = mpsc::channel::<RequestResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);
        let _ = self.phase_tx.send(RunPhase::Running);

        // Create aggregator
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

        let scenarios = Arc::new(self.config.scenarios.clone());
        let checks = Arc::new(self.config.checks.clone());

        // Create check results channel if checks are configured
        let (check_tx, check_rx) = if !self.config.checks.is_empty() {
            let (tx, rx) = mpsc::channel::<CheckResult>(RESULT_CHANNEL_SIZE);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        // Spawn check stats aggregator
        let check_stats_clone = self.check_stats.clone();
        let check_agg_handle = if let Some(mut rx) = check_rx {
            Some(tokio::spawn(async move {
                while let Some(check_result) = rx.recv().await {
                    let mut stats = check_stats_clone.lock().unwrap();
                    let entry = stats.entry(check_result.name).or_insert((0, 0));
                    if check_result.passed {
                        entry.0 += 1;
                    }
                    entry.1 += 1;
                }
            }))
        } else {
            None
        };

        // Create and run the arrival rate executor
        let executor = ArrivalRateExecutor::new(
            arrival_rate,
            self.config.duration,
            max_vus,
            pre_allocated_vus,
            client,
            self.config.url.clone(),
            self.config.method.clone(),
            self.config.headers.clone(),
            self.config.body.clone(),
            scenarios,
            checks,
            result_tx,
            check_tx,
            self.cancel_token.clone(),
        );

        // Store references to metrics
        let dropped_ref = executor.dropped_iterations();
        let vus_active_ref = executor.vus_active();

        // Spawn fail-fast threshold checker if enabled
        let fail_fast_handle = if self.config.fail_fast && !self.config.thresholds.is_empty() {
            let thresholds = self.config.thresholds.clone();
            let snapshot_rx = self.snapshot_rx.clone();
            let cancel = self.cancel_token.clone();
            let threshold_failed = self.threshold_failed.clone();
            Some(tokio::spawn(async move {
                run_fail_fast_checker(thresholds, snapshot_rx, cancel, threshold_failed).await
            }))
        } else {
            None
        };

        // Run the executor
        let cancel_token = self.cancel_token.clone();
        let executor_handle = tokio::spawn(async move {
            executor.run().await;
        });

        // Wait for duration or cancellation
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
            _ = executor_handle => {
                tracing::info!("Executor finished");
            }
        }

        // Store final metrics
        self.dropped_iterations.store(dropped_ref.load(Ordering::Relaxed), Ordering::Relaxed);
        self.vus_active.store(vus_active_ref.load(Ordering::Relaxed), Ordering::Relaxed);

        if let Some(handle) = fail_fast_handle {
            handle.abort();
        }

        if let Some(handle) = check_agg_handle {
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

    async fn run_constant_vus_mode(mut self) -> Result<Stats, String> {
        let client = create_client(
            self.config.concurrency,
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
            self.config.http2,
            self.config.cookie_jar,
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

        // Determine if using stages or simple concurrency
        let use_stages = !self.config.stages.is_empty() 
            && self.config.stages.iter().any(|s| s.target.is_some());
        let (worker_permits, total_duration, max_workers): (Arc<Semaphore>, Duration, u32) = if use_stages {
            // Stages mode: use StagesScheduler
            let max_target = self.config.stages.iter()
                .filter_map(|s| s.target)
                .max()
                .unwrap_or(1);
            let (stages_scheduler, stage_info_rx) = 
                StagesScheduler::new(self.config.stages.clone(), max_target);
            let permits = stages_scheduler.permits();
            let duration = stages_scheduler.total_duration();
            self.stage_info_rx = Some(stage_info_rx);
            tokio::spawn(stages_scheduler.run());
            (permits, self.config.warmup + duration, max_target)
        } else {
            // Simple mode: use RampUpScheduler
            let ramp_scheduler = RampUpScheduler::new(self.config.concurrency, self.config.ramp_up);
            let permits = ramp_scheduler.permits();
            tokio::spawn(ramp_scheduler.run());
            (permits, self.config.warmup + self.config.duration, self.config.concurrency)
        };

        let (result_tx, result_rx) = mpsc::channel::<RequestResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);

        // Create aggregator
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

        // Spawn workers (up to max needed)
        let mut worker_handles = Vec::with_capacity(max_workers as usize);
        let scenarios = Arc::new(self.config.scenarios.clone());
        let checks = Arc::new(self.config.checks.clone());

        // Create check results channel if checks are configured
        let (check_tx, check_rx) = if !self.config.checks.is_empty() {
            let (tx, rx) = mpsc::channel::<CheckResult>(RESULT_CHANNEL_SIZE);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        // Spawn check stats aggregator - drains channel completely
        let check_stats_clone = self.check_stats.clone();
        let check_agg_handle = if let Some(mut rx) = check_rx {
            Some(tokio::spawn(async move {
                while let Some(check_result) = rx.recv().await {
                    let mut stats = check_stats_clone.lock().unwrap();
                    let entry = stats.entry(check_result.name).or_insert((0, 0));
                    if check_result.passed {
                        entry.0 += 1;
                    }
                    entry.1 += 1;
                }
            }))
        } else {
            None
        };

        for id in 0..max_workers {
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
                worker_permits.clone(),
                self.config.think_time,
                checks.clone(),
                check_tx.clone(),
            );
            worker_handles.push(tokio::spawn(worker.run()));
        }

        drop(result_tx);
        drop(check_tx);

        let cancel_token = self.cancel_token.clone();

        // Spawn fail-fast threshold checker if enabled
        let fail_fast_handle = if self.config.fail_fast && !self.config.thresholds.is_empty() {
            let thresholds = self.config.thresholds.clone();
            let snapshot_rx = self.snapshot_rx.clone();
            let cancel = cancel_token.clone();
            let threshold_failed = self.threshold_failed.clone();
            Some(tokio::spawn(async move {
                run_fail_fast_checker(thresholds, snapshot_rx, cancel, threshold_failed).await
            }))
        } else {
            None
        };

        // Wait for total duration
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping workers");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
        }

        // Cancel fail-fast checker if running
        if let Some(handle) = fail_fast_handle {
            handle.abort();
        }

        // Wait for workers to finish (with timeout)
        for handle in worker_handles {
            let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
        }

        // Wait for check aggregator to drain all results
        if let Some(handle) = check_agg_handle {
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

async fn run_fail_fast_checker(
    thresholds: Vec<Threshold>,
    mut snapshot_rx: watch::Receiver<StatsSnapshot>,
    cancel_token: CancellationToken,
    threshold_failed: Arc<AtomicBool>,
) {
    // Wait a bit before starting checks (need some data first)
    sleep(Duration::from_secs(2)).await;

    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {
                let snapshot = snapshot_rx.borrow().clone();

                // Only check if we have some requests
                if snapshot.total_requests == 0 {
                    continue;
                }

                let results = evaluate_thresholds(&thresholds, &snapshot);
                let any_failed = results.iter().any(|r| !r.passed);

                if any_failed {
                    eprintln!("\n\x1b[31m⚠ FAIL-FAST: Threshold breached, aborting test\x1b[0m");
                    for result in &results {
                        if !result.passed {
                            eprintln!("  \x1b[31m✗ {} (actual: {:.2})\x1b[0m", result.condition, result.actual);
                        }
                    }
                    threshold_failed.store(true, Ordering::Relaxed);
                    cancel_token.cancel();
                    break;
                }
            }
            _ = cancel_token.cancelled() => {
                break;
            }
        }
    }
}
