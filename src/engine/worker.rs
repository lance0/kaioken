use crate::engine::scheduler::RateLimiter;
use crate::http::execute_request;
use crate::types::RequestResult;
use reqwest::{Client, Method};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct Worker {
    id: u32,
    client: Client,
    url: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    result_tx: mpsc::Sender<RequestResult>,
    cancel_token: CancellationToken,
    rate_limiter: Option<Arc<RateLimiter>>,
    ramp_permits: Arc<Semaphore>,
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
        result_tx: mpsc::Sender<RequestResult>,
        cancel_token: CancellationToken,
        rate_limiter: Option<Arc<RateLimiter>>,
        ramp_permits: Arc<Semaphore>,
    ) -> Self {
        Self {
            id,
            client,
            url,
            method,
            headers,
            body,
            result_tx,
            cancel_token,
            rate_limiter,
            ramp_permits,
        }
    }

    pub async fn run(self) {
        // Wait for ramp-up activation
        let _permit = self.ramp_permits.acquire().await.unwrap();
        tracing::debug!("Worker {} activated", self.id);

        let mut request_counter: u64 = 0;
        let base_request_id = (self.id as u64) * 1_000_000_000;

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

            // Interpolate variables
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

            let result = execute_request(
                &self.client,
                &url,
                &self.method,
                &headers,
                body.as_deref(),
            )
            .await;

            if self.result_tx.send(result).await.is_err() {
                break;
            }
        }

        tracing::debug!("Worker {} stopped", self.id);
    }
}

fn interpolate_vars(s: &str, request_id: u64, timestamp_ms: u128) -> String {
    s.replace("${REQUEST_ID}", &request_id.to_string())
        .replace("${TIMESTAMP_MS}", &timestamp_ms.to_string())
}
