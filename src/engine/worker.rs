use crate::engine::scheduler::RateLimiter;
use crate::http::execute_request;
use crate::types::{Check, CheckCondition, ExtractionSource, FormField, RequestResult, Scenario};
use rand::Rng;
use rand_regex::Regex as RandRegex;
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Semaphore, mpsc};
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
    checks: Arc<Vec<Check>>,
    check_tx: Option<mpsc::Sender<CheckResult>>,
    form_fields: Arc<Vec<FormField>>,
    basic_auth: Option<(String, Option<String>)>,
    // v1.3.0 features
    url_list: Option<Arc<Vec<String>>>,
    body_lines: Option<Arc<Vec<String>>>,
    rand_regex_generator: Option<RandRegex>,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
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
        checks: Arc<Vec<Check>>,
        check_tx: Option<mpsc::Sender<CheckResult>>,
        form_fields: Arc<Vec<FormField>>,
        basic_auth: Option<(String, Option<String>)>,
        url_list: Option<Arc<Vec<String>>>,
        body_lines: Option<Arc<Vec<String>>>,
        rand_regex_pattern: Option<&str>,
    ) -> Self {
        let total_weight: u32 = scenarios.iter().map(|s| s.weight).sum();

        // Compile rand-regex pattern if provided
        let rand_regex_generator = rand_regex_pattern.map(|pattern| {
            RandRegex::compile(pattern, 100).expect("Invalid rand-regex-url pattern")
        });

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
            checks,
            check_tx,
            form_fields,
            basic_auth,
            url_list,
            body_lines,
            rand_regex_generator,
        }
    }

    pub async fn run(self) {
        // Wait for ramp-up activation
        let _permit = self.ramp_permits.acquire().await.unwrap();
        tracing::debug!("Worker {} activated", self.id);

        let mut request_counter: u64 = 0;
        let base_request_id = (self.id as u64) * 1_000_000_000;
        let use_scenarios = !self.scenarios.is_empty();

        // Determine if we need to capture body (for checks or extractions)
        let has_body_checks = self.checks.iter().any(|c| {
            matches!(
                c.condition,
                CheckCondition::BodyContains(_)
                    | CheckCondition::BodyNotContains(_)
                    | CheckCondition::BodyMatches(_)
            )
        });
        let has_extractions =
            use_scenarios && self.scenarios.iter().any(|s| !s.extractions.is_empty());
        let capture_body = has_body_checks || has_extractions;

        // Per-worker extracted values storage
        let mut extracted_values: HashMap<String, String> = HashMap::new();

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
            let (url, method, headers, body, extractions) = if use_scenarios {
                let scenario = self.select_scenario(request_counter);
                let url =
                    interpolate_vars(&scenario.url, request_id, timestamp_ms, &extracted_values);
                let headers: Vec<(String, String)> = scenario
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            interpolate_vars(v, request_id, timestamp_ms, &extracted_values),
                        )
                    })
                    .collect();
                let body = scenario
                    .body
                    .as_ref()
                    .map(|b| interpolate_vars(b, request_id, timestamp_ms, &extracted_values));
                (
                    url,
                    scenario.method.clone(),
                    headers,
                    body,
                    scenario.extractions.clone(),
                )
            } else {
                // URL selection priority: rand_regex_generator > url_list > self.url
                let base_url = if let Some(ref generator) = self.rand_regex_generator {
                    let mut rng = rand::rng();
                    rng.sample(generator)
                } else if let Some(ref urls) = self.url_list {
                    urls[(request_counter as usize - 1) % urls.len()].clone()
                } else {
                    self.url.clone()
                };
                let url = interpolate_vars(&base_url, request_id, timestamp_ms, &extracted_values);

                let headers: Vec<(String, String)> = self
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            interpolate_vars(v, request_id, timestamp_ms, &extracted_values),
                        )
                    })
                    .collect();

                // Body selection: body_lines takes priority over self.body
                let body = if let Some(ref lines) = self.body_lines {
                    let line = &lines[(request_counter as usize - 1) % lines.len()];
                    Some(interpolate_vars(line, request_id, timestamp_ms, &extracted_values))
                } else {
                    self.body
                        .as_ref()
                        .map(|b| interpolate_vars(b, request_id, timestamp_ms, &extracted_values))
                };
                (url, self.method.clone(), headers, body, Vec::new())
            };

            // Prepare form data and basic auth for the request
            let form_data = if !self.form_fields.is_empty() {
                Some(self.form_fields.as_slice())
            } else {
                None
            };
            let basic_auth_ref = self
                .basic_auth
                .as_ref()
                .map(|(u, p)| (u.as_str(), p.as_deref()));

            let result = execute_request(
                &self.client,
                &url,
                &method,
                &headers,
                body.as_deref(),
                form_data,
                basic_auth_ref,
                capture_body,
                None, // No latency correction for closed-loop mode
            )
            .await;

            // Perform extractions if configured and request succeeded
            if !extractions.is_empty() && result.status.is_some() {
                let body_str = result.body.as_deref().unwrap_or("");
                for extraction in &extractions {
                    if let Some(value) = extract_value(&extraction.source, body_str, &headers) {
                        extracted_values.insert(extraction.name.clone(), value);
                    }
                }
            }

            // Evaluate checks if configured
            if !self.checks.is_empty()
                && let Some(ref check_tx) = self.check_tx
            {
                let body_str = result.body.as_deref().unwrap_or("");
                for check in self.checks.iter() {
                    let passed = check.condition.evaluate(result.status, body_str);
                    let _ = check_tx
                        .send(CheckResult {
                            name: check.name.clone(),
                            passed,
                        })
                        .await;
                }
            }

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

fn interpolate_vars(
    s: &str,
    request_id: u64,
    timestamp_ms: u128,
    extracted: &HashMap<String, String>,
) -> String {
    let mut result = s
        .replace("${REQUEST_ID}", &request_id.to_string())
        .replace("${TIMESTAMP_MS}", &timestamp_ms.to_string());

    // Replace extracted variables
    for (name, value) in extracted {
        let pattern = format!("${{{}}}", name);
        result = result.replace(&pattern, value);
    }

    result
}

fn extract_value(
    source: &ExtractionSource,
    body: &str,
    _headers: &[(String, String)],
) -> Option<String> {
    match source {
        ExtractionSource::JsonPath(path) => {
            use jsonpath_rust::JsonPath;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                // Use the JsonPath trait method on Value
                let results = json.query(path);
                if let Ok(values) = results
                    && let Some(first) = values.first()
                {
                    return match first {
                        serde_json::Value::String(s) => Some(s.clone()),
                        serde_json::Value::Number(n) => Some(n.to_string()),
                        serde_json::Value::Bool(b) => Some(b.to_string()),
                        serde_json::Value::Null => Some("null".to_string()),
                        other => Some(other.to_string()),
                    };
                }
            }
            None
        }
        ExtractionSource::Header(name) => {
            // Headers would need to be passed from execute_request
            // For now, this is a placeholder
            let _ = name;
            None
        }
        ExtractionSource::Regex(pattern, group) => {
            if let Ok(re) = regex_lite::Regex::new(pattern)
                && let Some(caps) = re.captures(body)
                && let Some(m) = caps.get(*group)
            {
                return Some(m.as_str().to_string());
            }
            None
        }
        ExtractionSource::Body => Some(body.to_string()),
    }
}
