use crate::http::execute_request;
use crate::types::{Check, CheckCondition, RequestResult, Scenario};
use reqwest::Client;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use super::worker::CheckResult;

/// Executes load test at a constant arrival rate (fixed RPS).
/// Unlike constant VUs, this spawns iterations at a fixed rate regardless of response time.
pub struct ArrivalRateExecutor {
    rate: u32,
    duration: Duration,
    max_vus: u32,
    pre_allocated_vus: u32,

    // Request configuration
    client: Client,
    url: String,
    method: reqwest::Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    scenarios: Arc<Vec<Scenario>>,
    checks: Arc<Vec<Check>>,

    // Runtime state
    vus_available: Arc<Semaphore>,
    vus_active: Arc<AtomicU32>,
    dropped_iterations: Arc<AtomicU64>,
    iteration_counter: Arc<AtomicU64>,

    // Channels
    result_tx: mpsc::Sender<RequestResult>,
    check_tx: Option<mpsc::Sender<CheckResult>>,
    cancel_token: CancellationToken,
}

impl ArrivalRateExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rate: u32,
        duration: Duration,
        max_vus: u32,
        pre_allocated_vus: u32,
        client: Client,
        url: String,
        method: reqwest::Method,
        headers: Vec<(String, String)>,
        body: Option<String>,
        scenarios: Arc<Vec<Scenario>>,
        checks: Arc<Vec<Check>>,
        result_tx: mpsc::Sender<RequestResult>,
        check_tx: Option<mpsc::Sender<CheckResult>>,
        cancel_token: CancellationToken,
    ) -> Self {
        let effective_pre_allocated = pre_allocated_vus.min(max_vus).max(1);

        Self {
            rate,
            duration,
            max_vus,
            pre_allocated_vus: effective_pre_allocated,
            client,
            url,
            method,
            headers,
            body,
            scenarios,
            checks,
            vus_available: Arc::new(Semaphore::new(effective_pre_allocated as usize)),
            vus_active: Arc::new(AtomicU32::new(0)),
            dropped_iterations: Arc::new(AtomicU64::new(0)),
            iteration_counter: Arc::new(AtomicU64::new(0)),
            result_tx,
            check_tx,
            cancel_token,
        }
    }

    pub fn dropped_iterations(&self) -> Arc<AtomicU64> {
        self.dropped_iterations.clone()
    }

    pub fn vus_active(&self) -> Arc<AtomicU32> {
        self.vus_active.clone()
    }

    pub async fn run(self) {
        if self.rate == 0 {
            tracing::warn!("Arrival rate is 0, no iterations will be spawned");
            return;
        }

        let interval_ns = 1_000_000_000u64 / self.rate as u64;
        let interval = Duration::from_nanos(interval_ns);

        let start = Instant::now();
        let mut next_spawn = start + interval;

        tracing::info!(
            "Starting arrival rate executor: {} req/s, max {} VUs, duration {:?}",
            self.rate,
            self.max_vus,
            self.duration
        );

        // Track total VUs we've allocated (for dynamic scaling)
        let mut total_vus_allocated = self.pre_allocated_vus;

        while start.elapsed() < self.duration {
            if self.cancel_token.is_cancelled() {
                break;
            }

            // Sleep until next spawn time
            let now = Instant::now();
            if next_spawn > now {
                tokio::select! {
                    _ = tokio::time::sleep(next_spawn - now) => {}
                    _ = self.cancel_token.cancelled() => break,
                }
            }
            next_spawn += interval;

            // Try to acquire a VU permit
            match self.vus_available.clone().try_acquire_owned() {
                Ok(permit) => {
                    self.spawn_iteration(permit);
                }
                Err(_) => {
                    // No VU available - can we allocate more?
                    if total_vus_allocated < self.max_vus {
                        // Add more VU capacity
                        let to_add = (self.max_vus - total_vus_allocated).min(10); // Add up to 10 at a time
                        self.vus_available.add_permits(to_add as usize);
                        total_vus_allocated += to_add;
                        tracing::debug!("Scaled VUs to {}", total_vus_allocated);

                        // Try again
                        if let Ok(permit) = self.vus_available.clone().try_acquire_owned() {
                            self.spawn_iteration(permit);
                        } else {
                            self.dropped_iterations.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        // At max capacity - drop this iteration
                        self.dropped_iterations.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        tracing::info!(
            "Arrival rate executor finished. Dropped iterations: {}",
            self.dropped_iterations.load(Ordering::Relaxed)
        );
    }

    fn spawn_iteration(&self, permit: tokio::sync::OwnedSemaphorePermit) {
        let iteration_id = self.iteration_counter.fetch_add(1, Ordering::Relaxed);
        let vus_active = self.vus_active.clone();
        let result_tx = self.result_tx.clone();
        let check_tx = self.check_tx.clone();
        let cancel_token = self.cancel_token.clone();

        let client = self.client.clone();
        let url = self.url.clone();
        let method = self.method.clone();
        let headers = self.headers.clone();
        let body = self.body.clone();
        let scenarios = self.scenarios.clone();
        let checks = self.checks.clone();

        tokio::spawn(async move {
            vus_active.fetch_add(1, Ordering::Relaxed);

            // Execute single iteration
            let result = execute_iteration(
                iteration_id,
                &client,
                &url,
                &method,
                &headers,
                body.as_deref(),
                &scenarios,
                &checks,
                &check_tx,
                &cancel_token,
            )
            .await;

            if let Some(result) = result {
                let _ = result_tx.send(result).await;
            }

            vus_active.fetch_sub(1, Ordering::Relaxed);
            drop(permit); // Release VU back to pool
        });
    }
}

async fn execute_iteration(
    iteration_id: u64,
    client: &Client,
    base_url: &str,
    base_method: &reqwest::Method,
    base_headers: &[(String, String)],
    base_body: Option<&str>,
    scenarios: &[Scenario],
    checks: &[Check],
    check_tx: &Option<mpsc::Sender<CheckResult>>,
    cancel_token: &CancellationToken,
) -> Option<RequestResult> {
    if cancel_token.is_cancelled() {
        return None;
    }

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // Determine if we need to capture body
    let has_body_checks = checks.iter().any(|c| {
        matches!(
            c.condition,
            CheckCondition::BodyContains(_)
                | CheckCondition::BodyNotContains(_)
                | CheckCondition::BodyMatches(_)
        )
    });
    let has_extractions = scenarios.iter().any(|s| !s.extractions.is_empty());
    let capture_body = has_body_checks || has_extractions;

    // Select scenario or use default target
    let (url, method, headers, body) = if !scenarios.is_empty() {
        let scenario = select_scenario(scenarios, iteration_id);
        let url = interpolate_vars(&scenario.url, iteration_id, timestamp_ms);
        let headers: Vec<(String, String)> = scenario
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), interpolate_vars(v, iteration_id, timestamp_ms)))
            .collect();
        let body = scenario
            .body
            .as_ref()
            .map(|b| interpolate_vars(b, iteration_id, timestamp_ms));
        (url, scenario.method.clone(), headers, body)
    } else {
        let url = interpolate_vars(base_url, iteration_id, timestamp_ms);
        let headers: Vec<(String, String)> = base_headers
            .iter()
            .map(|(k, v)| (k.clone(), interpolate_vars(v, iteration_id, timestamp_ms)))
            .collect();
        let body = base_body.map(|b| interpolate_vars(b, iteration_id, timestamp_ms));
        (url, base_method.clone(), headers, body)
    };

    let result = execute_request(client, &url, &method, &headers, body.as_deref(), capture_body).await;

    // Evaluate checks
    if !checks.is_empty() {
        if let Some(tx) = &check_tx {
            let body_str = result.body.as_deref().unwrap_or("");
            for check in checks.iter() {
                let passed = check.condition.evaluate(result.status, body_str);
                let _ = tx
                    .send(CheckResult {
                        name: check.name.clone(),
                        passed,
                    })
                    .await;
            }
        }
    }

    Some(result)
}

fn select_scenario(scenarios: &[Scenario], iteration_id: u64) -> &Scenario {
    if scenarios.len() == 1 {
        return &scenarios[0];
    }

    let total_weight: u32 = scenarios.iter().map(|s| s.weight).sum();
    if total_weight == 0 {
        return &scenarios[0];
    }

    let roll = (iteration_id % total_weight as u64) as u32;
    let mut cumulative = 0u32;

    for scenario in scenarios.iter() {
        cumulative += scenario.weight;
        if roll < cumulative {
            return scenario;
        }
    }

    &scenarios[0]
}

fn interpolate_vars(s: &str, request_id: u64, timestamp_ms: u128) -> String {
    s.replace("${REQUEST_ID}", &request_id.to_string())
        .replace("${TIMESTAMP_MS}", &timestamp_ms.to_string())
}
