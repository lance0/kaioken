use crate::types::{ErrorKind, RequestResult};
use reqwest::{Client, Method};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Get current time in microseconds since UNIX epoch
pub fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

pub async fn execute_request(
    client: &Client,
    url: &str,
    method: &Method,
    headers: &[(String, String)],
    body: Option<&str>,
    capture_body: bool,
    scheduled_at_us: Option<u64>, // For latency correction
) -> RequestResult {
    let started_at_us = now_us();
    let start = Instant::now();

    let mut request = client.request(method.clone(), url);

    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }

    if let Some(body_str) = body {
        request = request.body(body_str.to_string());
    }

    let result = match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let content_length = response.content_length().unwrap_or(0);

            let response_body = if capture_body {
                (response.text().await).ok()
            } else {
                // Consume body to allow connection reuse
                let _ = response.bytes().await;
                None
            };

            let latency_us = start.elapsed().as_micros() as u64;
            RequestResult::success(latency_us, status, content_length, response_body)
        }
        Err(err) => {
            let latency_us = start.elapsed().as_micros() as u64;
            let kind = ErrorKind::from_reqwest_error(&err);
            RequestResult::error(latency_us, kind)
        }
    };

    // Apply timing info for latency correction if scheduled time was provided
    if let Some(scheduled) = scheduled_at_us {
        result.with_timing(scheduled, started_at_us)
    } else {
        result
    }
}
