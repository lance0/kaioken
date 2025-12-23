use crate::types::{LoadConfig, StatsSnapshot, ThresholdResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter};

#[derive(Serialize, Deserialize)]
pub struct JsonOutput {
    pub metadata: Metadata,
    pub summary: Summary,
    pub latency_us: Latency,
    pub status_codes: HashMap<String, u64>,
    pub errors: HashMap<String, u64>,
    pub timeline: Vec<TimelineEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thresholds: Option<ThresholdsOutput>,
}

#[derive(Serialize, Deserialize)]
pub struct ThresholdsOutput {
    pub passed: bool,
    pub results: Vec<ThresholdResult>,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub tool: String,
    pub version: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub target: Target,
    pub load: Load,
    pub env: Environment,
}

#[derive(Serialize, Deserialize)]
pub struct Target {
    pub url: String,
    pub method: String,
    pub headers: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Load {
    pub concurrency: u32,
    pub rate: u32,
    pub ramp_up_secs: u64,
    pub warmup_secs: u64,
    pub timeout_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Environment {
    pub hostname: String,
    pub os: String,
    pub cpus: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Summary {
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub error_rate: f64,
    pub requests_per_sec: f64,
    pub bytes_received: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Latency {
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub stddev: f64,
    pub p50: u64,
    pub p75: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TimelineEntry {
    pub elapsed_secs: u32,
    pub requests: u64,
    pub errors: u64,
}

fn redact_header(header: &str) -> String {
    let lower = header.to_lowercase();
    if lower.starts_with("authorization:")
        || lower.starts_with("cookie:")
        || lower.starts_with("x-api-key:")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
    {
        let parts: Vec<&str> = header.splitn(2, ':').collect();
        if parts.len() == 2 {
            return format!("{}: <redacted>", parts[0]);
        }
    }
    header.to_string()
}

pub fn create_output(
    snapshot: &StatsSnapshot,
    config: &LoadConfig,
    threshold_results: Option<&[ThresholdResult]>,
) -> JsonOutput {
    let now = Utc::now();
    let started_at = now - chrono::Duration::from_std(snapshot.elapsed).unwrap_or_default();

    let headers: Vec<String> = config
        .headers
        .iter()
        .map(|(k, v)| redact_header(&format!("{}: {}", k, v)))
        .collect();

    let status_codes: HashMap<String, u64> = snapshot
        .status_codes
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let errors: HashMap<String, u64> = snapshot
        .errors
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), *v))
        .collect();

    let timeline: Vec<TimelineEntry> = snapshot
        .timeline
        .iter()
        .map(|b| TimelineEntry {
            elapsed_secs: b.elapsed_secs,
            requests: b.requests,
            errors: b.errors,
        })
        .collect();

    JsonOutput {
        metadata: Metadata {
            tool: "kaioken".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at,
            ended_at: now,
            duration_secs: snapshot.elapsed.as_secs(),
            target: Target {
                url: config.url.clone(),
                method: config.method.to_string(),
                headers,
            },
            load: Load {
                concurrency: config.concurrency,
                rate: config.rate,
                ramp_up_secs: config.ramp_up.as_secs(),
                warmup_secs: config.warmup.as_secs(),
                timeout_ms: config.timeout.as_millis() as u64,
            },
            env: Environment {
                hostname: hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string()),
                os: std::env::consts::OS.to_string(),
                cpus: num_cpus(),
            },
        },
        summary: Summary {
            total_requests: snapshot.total_requests,
            successful: snapshot.successful,
            failed: snapshot.failed,
            error_rate: snapshot.error_rate,
            requests_per_sec: snapshot.requests_per_sec,
            bytes_received: snapshot.bytes_received,
        },
        latency_us: Latency {
            min: snapshot.latency_min_us,
            max: snapshot.latency_max_us,
            mean: snapshot.latency_mean_us,
            stddev: snapshot.latency_stddev_us,
            p50: snapshot.latency_p50_us,
            p75: snapshot.latency_p75_us,
            p90: snapshot.latency_p90_us,
            p95: snapshot.latency_p95_us,
            p99: snapshot.latency_p99_us,
            p999: snapshot.latency_p999_us,
        },
        status_codes,
        errors,
        timeline,
        thresholds: threshold_results.map(|results| ThresholdsOutput {
            passed: results.iter().all(|r| r.passed),
            results: results.to_vec(),
        }),
    }
}

pub fn write_json(
    snapshot: &StatsSnapshot,
    config: &LoadConfig,
    path: &str,
    threshold_results: Option<&[ThresholdResult]>,
) -> io::Result<()> {
    let output = create_output(snapshot, config, threshold_results);
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}

pub fn print_json(
    snapshot: &StatsSnapshot,
    config: &LoadConfig,
    threshold_results: Option<&[ThresholdResult]>,
) -> io::Result<()> {
    let output = create_output(snapshot, config, threshold_results);
    let stdout = io::stdout();
    let writer = BufWriter::new(stdout.lock());
    serde_json::to_writer_pretty(writer, &output)?;
    println!();
    Ok(())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}
