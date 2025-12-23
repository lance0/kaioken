use crate::engine::scheduler::RateLimiter;
use crate::http::execute_request;
use crate::types::RequestResult;
use reqwest::{Client, Method};
use std::sync::Arc;
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

            let result = execute_request(
                &self.client,
                &self.url,
                &self.method,
                &self.headers,
                self.body.as_deref(),
            )
            .await;

            if self.result_tx.send(result).await.is_err() {
                break;
            }
        }

        tracing::debug!("Worker {} stopped", self.id);
    }
}
