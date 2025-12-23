use crate::engine::scheduler::RateLimiter;
use crate::http::execute_request;
use crate::types::{RequestResult, Scenario};
use reqwest::{Client, Method};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub struct Worker {
    id: u32,
    client: Client,
    url: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    scenarios: Arc<Vec<Scenario>>,
    total_weight: u32,
    result_tx: mpsc::Sender<RequestResult>,
    cancel_token: CancellationToken,
    rate_limiter: Option<Arc<RateLimiter>>,
    ramp_permits: Arc<Semaphore>,
    think_time: Option<Duration>,
}

impl Worker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        client: Client,
        url: String,
        method: Method,
        headers: Vec<(String, String)>,
        body: Option<String>,
        scenarios: Arc<Vec<Scenario>>,
        result_tx: mpsc::Sender<RequestResult>,
        cancel_token: CancellationToken,
        rate_limiter: Option<Arc<RateLimiter>>,
        ramp_permits: Arc<Semaphore>,
        think_time: Option<Duration>,
    ) -> Self {
        let total_weight: u32 = scenarios.iter().map(|s| s.weight).sum();

        Self {
            id,
            client,
            url,
            method,
            headers,
            body,
            scenarios,
            total_weight,
            result_tx,
            cancel_token,
            rate_limiter,
            ramp_permits,
            think_time,
        }
    }

    pub async fn run(self) {
        // Wait for ramp-up activation
        let _permit = self.ramp_permits.acquire().await.unwrap();
        tracing::debug!("Worker {} activated", self.id);

        let mut request_counter: u64 = 0;
        let base_request_id = (self.id as u64) * 1_000_000_000;
        let use_scenarios = !self.scenarios.is_empty();

        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            // Acquire rate limit permit if configured
            if let Some(ref limiter) = self.rate_limiter {
                tokio::select! {
                    _ = limiter.acquire() => {}
                    _ = self.cancel_token.cancelled() => break,
                }
            }

            if self.cancel_token.is_cancelled() {
                break;
            }

            request_counter += 1;
            let request_id = base_request_id + request_counter;
            let timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);

            // Select scenario or use default target
            let (url, method, headers, body) = if use_scenarios {
                let scenario = self.select_scenario(request_counter);
                let url = interpolate_vars(&scenario.url, request_id, timestamp_ms);
                let headers: Vec<(String, String)> = scenario
                    .headers
                    .iter()
                    .map(|(k, v)| (k.clone(), interpolate_vars(v, request_id, timestamp_ms)))
                    .collect();
                let body = scenario
                    .body
                    .as_ref()
                    .map(|b| interpolate_vars(b, request_id, timestamp_ms));
                (url, scenario.method.clone(), headers, body)
            } else {
                let url = interpolate_vars(&self.url, request_id, timestamp_ms);
                let headers: Vec<(String, String)> = self
                    .headers
                    .iter()
                    .map(|(k, v)| (k.clone(), interpolate_vars(v, request_id, timestamp_ms)))
                    .collect();
                let body = self
                    .body
                    .as_ref()
                    .map(|b| interpolate_vars(b, request_id, timestamp_ms));
                (url, self.method.clone(), headers, body)
            };

            let result = execute_request(&self.client, &url, &method, &headers, body.as_deref())
                .await;

            if self.result_tx.send(result).await.is_err() {
                break;
            }

            // Think time - pause between requests
            if let Some(think_time) = self.think_time {
                tokio::select! {
                    _ = sleep(think_time) => {}
                    _ = self.cancel_token.cancelled() => break,
                }
            }
        }

        tracing::debug!("Worker {} stopped", self.id);
    }

    fn select_scenario(&self, counter: u64) -> &Scenario {
        if self.scenarios.len() == 1 {
            return &self.scenarios[0];
        }

        // Simple weighted selection using counter as seed for deterministic distribution
        let roll = (counter % self.total_weight as u64) as u32;
        let mut cumulative = 0u32;

        for scenario in self.scenarios.iter() {
            cumulative += scenario.weight;
            if roll < cumulative {
                return scenario;
            }
        }

        // Fallback (shouldn't happen)
        &self.scenarios[0]
    }
}

fn interpolate_vars(s: &str, request_id: u64, timestamp_ms: u128) -> String {
    s.replace("${REQUEST_ID}", &request_id.to_string())
        .replace("${TIMESTAMP_MS}", &timestamp_ms.to_string())
}
