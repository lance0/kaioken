use crate::types::{ErrorKind, RequestResult};
use reqwest::{Client, Method};
use std::time::Instant;

pub async fn execute_request(
    client: &Client,
    url: &str,
    method: &Method,
    headers: &[(String, String)],
    body: Option<&str>,
) -> RequestResult {
    let start = Instant::now();

    let mut request = client.request(method.clone(), url);

    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }

    if let Some(body_str) = body {
        request = request.body(body_str.to_string());
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let latency_us = start.elapsed().as_micros() as u64;
            let content_length = response.content_length().unwrap_or(0);
            RequestResult::success(latency_us, status, content_length)
        }
        Err(err) => {
            let latency_us = start.elapsed().as_micros() as u64;
            let kind = ErrorKind::from_reqwest_error(&err);
            RequestResult::error(latency_us, kind)
        }
    }
}
