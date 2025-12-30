use crate::engine::Stats;
use crate::engine::aggregator::Aggregator;
use crate::engine::arrival_rate::{ArrivalRateExecutor, RampingArrivalRateExecutor, RateStage};
use crate::engine::scheduler::{RampUpScheduler, RateLimiter, StageInfo, StagesScheduler};
use crate::engine::thresholds::evaluate_thresholds;
use crate::engine::worker::{CheckResult, Worker};
use crate::engine::ws_aggregator::WsAggregator;
use crate::engine::ws_worker::WsWorker;
#[cfg(feature = "grpc")]
use crate::grpc::{GrpcConfig, GrpcError, execute_grpc_request};
use crate::http::create_client;
#[cfg(feature = "http3")]
use crate::http3::{Http3Client, execute_http3_request};
use crate::types::{
    LoadConfig, RequestResult, RunPhase, RunState, StatsSnapshot, Threshold, WsMessageResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, mpsc, watch};
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

    /// Check if this is a WebSocket URL
    fn is_websocket(&self) -> bool {
        self.config.url.starts_with("ws://") || self.config.url.starts_with("wss://")
    }

    /// Check if arrival rate mode is enabled
    fn is_arrival_rate_mode(&self) -> bool {
        self.config.arrival_rate.is_some()
            || self.config.stages.iter().any(|s| s.target_rate.is_some())
    }

    /// Check if burst mode is enabled
    fn is_burst_mode(&self) -> bool {
        self.config.burst_config.is_some()
    }

    /// Check if HTTP/3 mode is enabled
    #[cfg(feature = "http3")]
    fn is_http3(&self) -> bool {
        self.config.http3
    }

    /// Check if gRPC mode is enabled
    #[cfg(feature = "grpc")]
    fn is_grpc(&self) -> bool {
        self.config.grpc_service.is_some() && self.config.grpc_method.is_some()
    }

    #[allow(dead_code)]
    pub fn dropped_iterations(&self) -> u64 {
        self.dropped_iterations.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn vus_active(&self) -> u32 {
        self.vus_active.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn vus_max(&self) -> u32 {
        self.vus_max.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn threshold_failed(&self) -> bool {
        self.threshold_failed.load(Ordering::Relaxed)
    }

    pub fn threshold_failed_flag(&self) -> Arc<AtomicBool> {
        self.threshold_failed.clone()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn stage_info_rx(&self) -> Option<watch::Receiver<StageInfo>> {
        self.stage_info_rx.clone()
    }

    pub async fn run(self) -> Result<Stats, String> {
        // Check if this is a WebSocket test
        if self.is_websocket() {
            return self.run_websocket_mode().await;
        }

        // Check if this is a gRPC test
        #[cfg(feature = "grpc")]
        if self.is_grpc() {
            return self.run_grpc_mode().await;
        }

        // Check if this is an HTTP/3 test
        #[cfg(feature = "http3")]
        if self.is_http3() {
            return self.run_http3_mode().await;
        }

        // Check if we should use arrival rate mode
        if self.is_arrival_rate_mode() {
            return self.run_arrival_rate_mode().await;
        }

        // Check if we should use burst mode
        if self.is_burst_mode() {
            return self.run_burst_mode().await;
        }

        // Otherwise, use constant VUs mode
        self.run_constant_vus_mode().await
    }

    async fn run_arrival_rate_mode(self) -> Result<Stats, String> {
        let max_vus = self.config.max_vus.unwrap_or(100);

        let client = create_client(
            max_vus,
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
            self.config.http2,
            self.config.cookie_jar,
            self.config.follow_redirects,
            self.config.disable_keepalive,
            self.config.proxy.as_deref(),
            self.config.client_cert.as_deref(),
            self.config.client_key.as_deref(),
            self.config.ca_cert.as_deref(),
            self.config
                .connect_to
                .as_ref()
                .map(|(h, a)| (h.as_str(), *a)),
        )
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        self.vus_max.store(max_vus, Ordering::Relaxed);

        let (result_tx, result_rx) = mpsc::channel::<RequestResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);
        let _ = self.phase_tx.send(RunPhase::Running);

        // Check if we have rate-based stages
        let has_rate_stages = self.config.stages.iter().any(|s| s.target_rate.is_some());

        // Calculate total duration
        let total_duration = if has_rate_stages {
            self.config.warmup
                + self
                    .config
                    .stages
                    .iter()
                    .map(|s| s.duration)
                    .sum::<Duration>()
        } else {
            self.config.warmup + self.config.duration
        };

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
        let check_agg_handle = check_rx.map(|mut rx| {
            tokio::spawn(async move {
                while let Some(check_result) = rx.recv().await {
                    let mut stats = check_stats_clone.lock().unwrap();
                    let entry = stats.entry(check_result.name).or_insert((0, 0));
                    if check_result.passed {
                        entry.0 += 1;
                    }
                    entry.1 += 1;
                }
            })
        });

        // Create shared metric references for arrival rate tracking
        let dropped_ref = Arc::new(AtomicU64::new(0));
        let vus_active_ref = Arc::new(AtomicU32::new(0));
        let target_rate_ref = Arc::new(AtomicU32::new(0));

        // Determine initial target rate
        let initial_target_rate = if has_rate_stages {
            self.config
                .stages
                .first()
                .and_then(|s| s.target_rate)
                .unwrap_or(0)
        } else {
            self.config.arrival_rate.unwrap_or(0)
        };
        target_rate_ref.store(initial_target_rate, Ordering::Relaxed);

        // Create aggregator with arrival rate metrics
        let aggregator = Aggregator::with_arrival_rate_metrics(
            total_duration,
            result_rx,
            self.snapshot_tx.clone(),
            self.config.warmup,
            self.phase_tx.clone(),
            self.config.max_requests,
            self.cancel_token.clone(),
            Some(dropped_ref.clone()),
            Some(vus_active_ref.clone()),
            max_vus,
            initial_target_rate,
            self.config.db_url.clone(),
            self.config.prometheus.clone(),
            &self.config.url,
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Create and spawn appropriate executor based on configuration
        let executor_handle = if has_rate_stages {
            // Use ramping arrival rate executor with stages
            let rate_stages: Vec<RateStage> = self
                .config
                .stages
                .iter()
                .filter_map(|s| {
                    s.target_rate.map(|rate| RateStage {
                        duration: s.duration,
                        target_rate: rate,
                    })
                })
                .collect();

            let max_rate = rate_stages
                .iter()
                .map(|s| s.target_rate)
                .max()
                .unwrap_or(10);
            let pre_allocated_vus = (max_rate / 10).max(1).min(max_vus);

            let executor = RampingArrivalRateExecutor::new(
                rate_stages,
                max_vus,
                pre_allocated_vus,
                self.config.latency_correction,
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

            // Link our shared metrics to executor's metrics
            let exec_dropped = executor.dropped_iterations();
            let exec_active = executor.vus_active();
            let dropped_clone = dropped_ref.clone();
            let active_clone = vus_active_ref.clone();

            tokio::spawn(async move {
                // Spawn a task to sync metrics periodically
                let sync_dropped = dropped_clone;
                let sync_active = active_clone;
                let sync_exec_dropped = exec_dropped.clone();
                let sync_exec_active = exec_active.clone();

                tokio::spawn(async move {
                    loop {
                        sync_dropped
                            .store(sync_exec_dropped.load(Ordering::Relaxed), Ordering::Relaxed);
                        sync_active
                            .store(sync_exec_active.load(Ordering::Relaxed), Ordering::Relaxed);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                });

                executor.run().await;
            })
        } else {
            // Use constant arrival rate executor
            let arrival_rate = self.config.arrival_rate.unwrap_or(10);
            let pre_allocated_vus = (arrival_rate / 10).max(1).min(max_vus);

            let executor = ArrivalRateExecutor::new(
                arrival_rate,
                self.config.duration,
                max_vus,
                pre_allocated_vus,
                self.config.latency_correction,
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

            // Link our shared metrics to executor's metrics
            let exec_dropped = executor.dropped_iterations();
            let exec_active = executor.vus_active();
            let dropped_clone = dropped_ref.clone();
            let active_clone = vus_active_ref.clone();

            tokio::spawn(async move {
                // Spawn a task to sync metrics periodically
                let sync_dropped = dropped_clone;
                let sync_active = active_clone;
                let sync_exec_dropped = exec_dropped.clone();
                let sync_exec_active = exec_active.clone();

                tokio::spawn(async move {
                    loop {
                        sync_dropped
                            .store(sync_exec_dropped.load(Ordering::Relaxed), Ordering::Relaxed);
                        sync_active
                            .store(sync_exec_active.load(Ordering::Relaxed), Ordering::Relaxed);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                });

                executor.run().await;
            })
        };

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

        // Wait for duration or cancellation
        let cancel_token = self.cancel_token.clone();
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
        self.dropped_iterations
            .store(dropped_ref.load(Ordering::Relaxed), Ordering::Relaxed);
        self.vus_active
            .store(vus_active_ref.load(Ordering::Relaxed), Ordering::Relaxed);

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
            self.config.follow_redirects,
            self.config.disable_keepalive,
            self.config.proxy.as_deref(),
            self.config.client_cert.as_deref(),
            self.config.client_key.as_deref(),
            self.config.ca_cert.as_deref(),
            self.config
                .connect_to
                .as_ref()
                .map(|(h, a)| (h.as_str(), *a)),
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
        let use_stages =
            !self.config.stages.is_empty() && self.config.stages.iter().any(|s| s.target.is_some());
        let (worker_permits, total_duration, max_workers): (Arc<Semaphore>, Duration, u32) =
            if use_stages {
                // Stages mode: use StagesScheduler
                let max_target = self
                    .config
                    .stages
                    .iter()
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
                let ramp_scheduler =
                    RampUpScheduler::new(self.config.concurrency, self.config.ramp_up);
                let permits = ramp_scheduler.permits();
                tokio::spawn(ramp_scheduler.run());
                (
                    permits,
                    self.config.warmup + self.config.duration,
                    self.config.concurrency,
                )
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
            self.config.db_url.clone(),
            self.config.prometheus.clone(),
            &self.config.url,
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
        let check_agg_handle = check_rx.map(|mut rx| {
            tokio::spawn(async move {
                while let Some(check_result) = rx.recv().await {
                    let mut stats = check_stats_clone.lock().unwrap();
                    let entry = stats.entry(check_result.name).or_insert((0, 0));
                    if check_result.passed {
                        entry.0 += 1;
                    }
                    entry.1 += 1;
                }
            })
        });

        let form_fields = Arc::new(self.config.form_fields.clone());

        // v1.3.0 features
        let url_list = self.config.url_list.as_ref().map(|v| Arc::new(v.clone()));
        let body_lines = self.config.body_lines.as_ref().map(|v| Arc::new(v.clone()));

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
                form_fields.clone(),
                self.config.basic_auth.clone(),
                url_list.clone(),
                body_lines.clone(),
                self.config.rand_regex_url.as_deref(),
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

    /// Run burst mode - send N requests, wait, repeat
    async fn run_burst_mode(self) -> Result<Stats, String> {
        let burst_config = self
            .config
            .burst_config
            .as_ref()
            .ok_or("Burst config not set")?
            .clone();

        let client = create_client(
            burst_config.requests_per_burst,
            self.config.timeout,
            self.config.connect_timeout,
            self.config.insecure,
            self.config.http2,
            self.config.cookie_jar,
            self.config.follow_redirects,
            self.config.disable_keepalive,
            self.config.proxy.as_deref(),
            self.config.client_cert.as_deref(),
            self.config.client_key.as_deref(),
            self.config.ca_cert.as_deref(),
            self.config
                .connect_to
                .as_ref()
                .map(|(h, a)| (h.as_str(), *a)),
        )
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let total_duration = self.config.warmup + self.config.duration;

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
            self.config.db_url.clone(),
            self.config.prometheus.clone(),
            &self.config.url,
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Spawn burst executor
        let url = self.config.url.clone();
        let method = self.config.method.clone();
        let headers = self.config.headers.clone();
        let body = self.config.body.clone();
        let cancel_token = self.cancel_token.clone();
        let form_fields = Arc::new(self.config.form_fields.clone());
        let basic_auth = self.config.basic_auth.clone();
        let burst_result_tx = result_tx.clone();
        drop(result_tx);

        let burst_handle = tokio::spawn(async move {
            let result_tx = burst_result_tx;
            let start = Instant::now();
            let mut burst_count = 0u64;

            while start.elapsed() < total_duration && !cancel_token.is_cancelled() {
                burst_count += 1;
                tracing::debug!("Starting burst {}", burst_count);

                // Send burst of requests concurrently
                let mut handles = Vec::with_capacity(burst_config.requests_per_burst as usize);

                for _ in 0..burst_config.requests_per_burst {
                    if cancel_token.is_cancelled() {
                        break;
                    }

                    let client = client.clone();
                    let url = url.clone();
                    let method = method.clone();
                    let headers = headers.clone();
                    let body = body.clone();
                    let result_tx = result_tx.clone();
                    let form_fields = form_fields.clone();
                    let basic_auth = basic_auth.clone();

                    let handle = tokio::spawn(async move {
                        let form_data = if !form_fields.is_empty() {
                            Some(form_fields.as_slice())
                        } else {
                            None
                        };
                        let basic_auth_ref =
                            basic_auth.as_ref().map(|(u, p)| (u.as_str(), p.as_deref()));

                        let result = crate::http::execute_request(
                            &client,
                            &url,
                            &method,
                            &headers,
                            body.as_deref(),
                            form_data,
                            basic_auth_ref,
                            false, // capture_body
                            None,  // scheduled_at
                        )
                        .await;

                        let _ = result_tx.send(result).await;
                    });
                    handles.push(handle);
                }

                // Wait for all requests in this burst to complete
                for handle in handles {
                    let _ = handle.await;
                }

                // Check if we should continue
                if start.elapsed() >= total_duration || cancel_token.is_cancelled() {
                    break;
                }

                // Wait before next burst
                tokio::select! {
                    _ = sleep(burst_config.delay_between_bursts) => {}
                    _ = cancel_token.cancelled() => break,
                }
            }

            tracing::info!("Burst mode completed: {} bursts sent", burst_count);
        });

        // Wait for duration or cancellation
        let cancel_token = self.cancel_token.clone();
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping burst mode");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
            _ = burst_handle => {
                tracing::info!("Burst executor finished");
            }
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

    /// Run HTTP/3 load test mode
    #[cfg(feature = "http3")]
    async fn run_http3_mode(self) -> Result<Stats, String> {
        use reqwest::Url;
        use std::net::ToSocketAddrs;

        let total_duration = self.config.warmup + self.config.duration;
        let concurrency = self.config.concurrency;

        // Parse URL for host, port, and path (including query string)
        let url = Url::parse(&self.config.url).map_err(|e| format!("Invalid URL: {}", e))?;
        let host = url.host_str().ok_or("Missing host in URL")?;
        let port = url.port().unwrap_or(443);
        let path = if let Some(query) = url.query() {
            format!("{}?{}", url.path(), query)
        } else if url.path().is_empty() {
            "/".to_string()
        } else {
            url.path().to_string()
        };

        // Resolve address
        let addr_str = format!("{}:{}", host, port);
        let addr = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("Failed to resolve {}: {}", addr_str, e))?
            .next()
            .ok_or_else(|| format!("No addresses found for {}", addr_str))?;

        // Create HTTP/3 client
        let client = Http3Client::new(self.config.insecure)
            .map_err(|e| format!("Failed to create HTTP/3 client: {}", e))?;
        let client = Arc::new(client);

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
            self.config.db_url.clone(),
            self.config.prometheus.clone(),
            &self.config.url,
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Spawn workers
        let mut worker_handles = Vec::with_capacity(concurrency as usize);
        let method = self.config.method.to_string();
        let headers: Vec<(String, String)> = self.config.headers.clone();
        let body = self.config.body.clone();
        let timeout = self.config.timeout;
        let server_name = host.to_string();

        for _id in 0..concurrency {
            let client = client.clone();
            let result_tx = result_tx.clone();
            let cancel_token = self.cancel_token.clone();
            let method = method.clone();
            let path = path.clone();
            let headers = headers.clone();
            let body = body.clone();
            let server_name = server_name.clone();

            let handle = tokio::spawn(async move {
                loop {
                    if cancel_token.is_cancelled() {
                        break;
                    }

                    let result = execute_http3_request(
                        &client,
                        addr,
                        &server_name,
                        &method,
                        &path,
                        &headers,
                        body.as_deref(),
                        timeout,
                    )
                    .await;

                    if result_tx.send(result).await.is_err() {
                        break;
                    }
                }
            });
            worker_handles.push(handle);
        }

        drop(result_tx);

        let cancel_token = self.cancel_token.clone();

        // Wait for duration or cancellation
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping HTTP/3 workers");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
        }

        // Wait for workers to finish
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

    /// Run gRPC load test mode
    #[cfg(feature = "grpc")]
    async fn run_grpc_mode(self) -> Result<Stats, String> {
        use crate::types::ErrorKind;
        use reqwest::Url;

        let total_duration = self.config.warmup + self.config.duration;
        let concurrency = self.config.concurrency;

        // Parse URL for address
        let url = Url::parse(&self.config.url).map_err(|e| format!("Invalid URL: {}", e))?;
        let host = url.host_str().ok_or("Missing host in URL")?;
        let port = url
            .port()
            .unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
        let address = format!("{}:{}", host, port);
        let tls = url.scheme() == "https";

        let service = self
            .config
            .grpc_service
            .clone()
            .ok_or("gRPC service not specified")?;
        let method = self
            .config
            .grpc_method
            .clone()
            .ok_or("gRPC method not specified")?;

        // Use binary body_bytes if available, otherwise fall back to string body as bytes
        let request_bytes = self.config.body_bytes.clone().unwrap_or_else(|| {
            self.config
                .body
                .as_ref()
                .map(|s| s.as_bytes().to_vec())
                .unwrap_or_default()
        });

        let grpc_config = GrpcConfig {
            address,
            service,
            method,
            request: request_bytes,
            timeout: self.config.timeout,
            tls,
            insecure: self.config.insecure,
            metadata: self.config.headers.clone(),
            ..Default::default()
        };
        let grpc_config = Arc::new(grpc_config);

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
            self.config.db_url.clone(),
            self.config.prometheus.clone(),
            &self.config.url,
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Spawn workers
        let mut worker_handles = Vec::with_capacity(concurrency as usize);

        for _id in 0..concurrency {
            let grpc_config = grpc_config.clone();
            let result_tx = result_tx.clone();
            let cancel_token = self.cancel_token.clone();

            let handle = tokio::spawn(async move {
                loop {
                    if cancel_token.is_cancelled() {
                        break;
                    }

                    let grpc_result = execute_grpc_request(&grpc_config).await;

                    // Convert gRPC result to HTTP-like RequestResult for aggregation
                    let result = RequestResult {
                        status: if grpc_result.error.is_some() {
                            None // Error case - no valid HTTP status
                        } else if grpc_result.status_code == 0 {
                            Some(200) // gRPC OK -> HTTP 200
                        } else {
                            // gRPC status codes are 0-16, clamp and map to 5xx range
                            Some(500 + grpc_result.status_code.clamp(0, 16) as u16)
                        },
                        latency_us: grpc_result.latency_us,
                        bytes_received: grpc_result.bytes_received,
                        error: grpc_result.error.map(|e| match e {
                            GrpcError::Connect(_) => ErrorKind::Connect,
                            GrpcError::Timeout => ErrorKind::Timeout,
                            _ => ErrorKind::Other,
                        }),
                        body: grpc_result.responses.first().cloned(),
                        scheduled_at_us: None,
                        started_at_us: None,
                        queue_time_us: None,
                    };

                    if result_tx.send(result).await.is_err() {
                        break;
                    }
                }
            });
            worker_handles.push(handle);
        }

        drop(result_tx);

        let cancel_token = self.cancel_token.clone();

        // Wait for duration or cancellation
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping gRPC workers");
                cancel_token.cancel();
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Cancellation requested");
            }
        }

        // Wait for workers to finish
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

    async fn run_websocket_mode(self) -> Result<Stats, String> {
        let total_duration = self.config.warmup + self.config.duration;
        let connection_count = self.config.concurrency;
        let message = self
            .config
            .body
            .clone()
            .unwrap_or_else(|| "ping".to_string());

        let (result_tx, result_rx) = mpsc::channel::<WsMessageResult>(RESULT_CHANNEL_SIZE);

        let _ = self.state_tx.send(RunState::Running);

        // Create WebSocket aggregator
        let aggregator = WsAggregator::new(
            total_duration,
            result_rx,
            self.snapshot_tx.clone(),
            self.config.warmup,
            self.phase_tx.clone(),
            self.cancel_token.clone(),
            connection_count,
        );
        let aggregator_handle = tokio::spawn(aggregator.run());

        // Spawn WebSocket workers
        let mut worker_handles = Vec::with_capacity(connection_count as usize);
        for id in 0..connection_count {
            let worker = WsWorker::new(
                id,
                self.config.url.clone(),
                message.clone(),
                self.config.ws_mode,
                self.config.ws_message_interval,
                self.config.timeout,
                result_tx.clone(),
                self.cancel_token.clone(),
            );
            worker_handles.push(tokio::spawn(worker.run()));
        }

        drop(result_tx);

        let cancel_token = self.cancel_token.clone();

        // Wait for total duration
        tokio::select! {
            _ = sleep(total_duration) => {
                tracing::info!("Duration elapsed, stopping WebSocket workers");
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

        // Wait for aggregator to finish
        let _ws_stats = aggregator_handle
            .await
            .map_err(|e| format!("Aggregator task failed: {}", e))?;

        let final_state = if self.cancel_token.is_cancelled() {
            RunState::Cancelled
        } else {
            RunState::Completed
        };
        let _ = self.state_tx.send(final_state);

        // Return empty HTTP Stats (WS stats are in snapshot)
        Ok(Stats::new(total_duration))
    }
}

async fn run_fail_fast_checker(
    thresholds: Vec<Threshold>,
    snapshot_rx: watch::Receiver<StatsSnapshot>,
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
