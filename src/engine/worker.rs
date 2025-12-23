use crate::http::execute_request;
use crate::types::RequestResult;
use reqwest::{Client, Method};
use tokio::sync::mpsc;
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
}

impl Worker {
    pub fn new(
        id: u32,
        client: Client,
        url: String,
        method: Method,
        headers: Vec<(String, String)>,
        body: Option<String>,
        result_tx: mpsc::Sender<RequestResult>,
        cancel_token: CancellationToken,
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
        }
    }

    pub async fn run(self) {
        tracing::debug!("Worker {} started", self.id);

        loop {
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
