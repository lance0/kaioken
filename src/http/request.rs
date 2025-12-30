use crate::types::{ErrorKind, FormField, RequestResult};
use reqwest::{Client, Method};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Get current time in microseconds since UNIX epoch
pub fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_request(
    client: &Client,
    url: &str,
    method: &Method,
    headers: &[(String, String)],
    body: Option<&str>,
    form_data: Option<&[FormField]>,
    basic_auth: Option<(&str, Option<&str>)>,
    capture_body: bool,
    scheduled_at_us: Option<u64>, // For latency correction
) -> RequestResult {
    let started_at_us = now_us();
    let start = Instant::now();

    let mut request = client.request(method.clone(), url);

    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }

    // Apply basic auth if provided
    if let Some((username, password)) = basic_auth {
        request = request.basic_auth(username, password);
    }

    // Build multipart form if form_data provided
    if let Some(fields) = form_data {
        match build_multipart_form(fields).await {
            Ok(form) => {
                request = request.multipart(form);
            }
            Err(_) => {
                let latency_us = start.elapsed().as_micros() as u64;
                return RequestResult::error(latency_us, ErrorKind::Other);
            }
        }
    } else if let Some(body_str) = body {
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

/// Build a multipart form from FormField entries
async fn build_multipart_form(
    fields: &[FormField],
) -> Result<reqwest::multipart::Form, Box<dyn std::error::Error + Send + Sync>> {
    use reqwest::multipart::{Form, Part};

    let mut form = Form::new();

    for field in fields {
        match field {
            FormField::Text { name, value } => {
                form = form.text(name.clone(), value.clone());
            }
            FormField::File {
                name,
                path,
                filename,
                mime_type,
            } => {
                let bytes = tokio::fs::read(path).await?;
                let file_name = filename
                    .clone()
                    .or_else(|| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| "file".to_string());

                let mut part = Part::bytes(bytes).file_name(file_name);

                if let Some(mime) = mime_type {
                    part = part.mime_str(mime)?;
                }

                form = form.part(name.clone(), part);
            }
        }
    }

    Ok(form)
}
