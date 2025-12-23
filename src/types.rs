use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Stages (v0.7)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Stage {
    pub duration: Duration,
    pub target: u32,
}

// ============================================================================
// Checks & Thresholds (v0.6)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub condition: CheckCondition,
}

#[derive(Debug, Clone)]
pub enum CheckCondition {
    StatusEquals(u16),
    StatusIn(Vec<u16>),
    StatusLt(u16),
    StatusGt(u16),
    BodyContains(String),
    BodyNotContains(String),
    BodyMatches(regex_lite::Regex),
}

impl CheckCondition {
    pub fn evaluate(&self, status: Option<u16>, body: &str) -> bool {
        match self {
            CheckCondition::StatusEquals(expected) => status == Some(*expected),
            CheckCondition::StatusIn(codes) => status.map(|s| codes.contains(&s)).unwrap_or(false),
            CheckCondition::StatusLt(threshold) => status.map(|s| s < *threshold).unwrap_or(false),
            CheckCondition::StatusGt(threshold) => status.map(|s| s > *threshold).unwrap_or(false),
            CheckCondition::BodyContains(needle) => body.contains(needle),
            CheckCondition::BodyNotContains(needle) => !body.contains(needle),
            CheckCondition::BodyMatches(re) => re.is_match(body),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Threshold {
    pub metric: ThresholdMetric,
    pub operator: ThresholdOp,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdMetric {
    P50LatencyMs,
    P75LatencyMs,
    P90LatencyMs,
    P95LatencyMs,
    P99LatencyMs,
    P999LatencyMs,
    MeanLatencyMs,
    MaxLatencyMs,
    ErrorRate,
    Rps,
}

impl ThresholdMetric {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThresholdMetric::P50LatencyMs => "p50_latency_ms",
            ThresholdMetric::P75LatencyMs => "p75_latency_ms",
            ThresholdMetric::P90LatencyMs => "p90_latency_ms",
            ThresholdMetric::P95LatencyMs => "p95_latency_ms",
            ThresholdMetric::P99LatencyMs => "p99_latency_ms",
            ThresholdMetric::P999LatencyMs => "p999_latency_ms",
            ThresholdMetric::MeanLatencyMs => "mean_latency_ms",
            ThresholdMetric::MaxLatencyMs => "max_latency_ms",
            ThresholdMetric::ErrorRate => "error_rate",
            ThresholdMetric::Rps => "rps",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdOp {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
}

impl ThresholdOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThresholdOp::Lt => "<",
            ThresholdOp::Lte => "<=",
            ThresholdOp::Gt => ">",
            ThresholdOp::Gte => ">=",
            ThresholdOp::Eq => "==",
        }
    }

    pub fn evaluate(&self, actual: f64, expected: f64) -> bool {
        match self {
            ThresholdOp::Lt => actual < expected,
            ThresholdOp::Lte => actual <= expected,
            ThresholdOp::Gt => actual > expected,
            ThresholdOp::Gte => actual >= expected,
            ThresholdOp::Eq => (actual - expected).abs() < f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdResult {
    pub metric: String,
    pub condition: String,
    pub actual: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckStats {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
}

impl CheckStats {
    pub fn pass_rate(&self) -> f64 {
        if self.total > 0 {
            self.passed as f64 / self.total as f64
        } else {
            1.0
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Timeout,
    Dns,
    Connect,
    Tls,
    Refused,
    Reset,
    Http,
    Body,
    Other,
}

impl ErrorKind {
    pub fn from_reqwest_error(err: &reqwest::Error) -> Self {
        if err.is_timeout() {
            ErrorKind::Timeout
        } else if err.is_connect() {
            if err.to_string().contains("dns") || err.to_string().contains("resolve") {
                ErrorKind::Dns
            } else if err.to_string().contains("refused") {
                ErrorKind::Refused
            } else if err.to_string().contains("reset") {
                ErrorKind::Reset
            } else {
                ErrorKind::Connect
            }
        } else if err.is_request() {
            ErrorKind::Http
        } else if err.is_body() {
            ErrorKind::Body
        } else if err.to_string().contains("tls") || err.to_string().contains("certificate") {
            ErrorKind::Tls
        } else {
            ErrorKind::Other
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::Timeout => "timeout",
            ErrorKind::Dns => "dns",
            ErrorKind::Connect => "connect",
            ErrorKind::Tls => "tls",
            ErrorKind::Refused => "refused",
            ErrorKind::Reset => "reset",
            ErrorKind::Http => "http",
            ErrorKind::Body => "body",
            ErrorKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub latency_us: u64,
    pub status: Option<u16>,
    pub error: Option<ErrorKind>,
    pub bytes_received: u64,
    pub body: Option<String>,
}

impl RequestResult {
    pub fn success(latency_us: u64, status: u16, bytes_received: u64, body: Option<String>) -> Self {
        Self {
            latency_us,
            status: Some(status),
            error: None,
            bytes_received,
            body,
        }
    }

    pub fn error(latency_us: u64, kind: ErrorKind) -> Self {
        Self {
            latency_us,
            status: None,
            error: Some(kind),
            bytes_received: 0,
            body: None,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status.map(|s| s < 400).unwrap_or(false)
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some() || self.status.map(|s| s >= 400).unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StatsSnapshot {
    pub elapsed: Duration,
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub bytes_received: u64,

    pub rolling_rps: f64,
    pub requests_per_sec: f64,
    pub error_rate: f64,

    pub latency_min_us: u64,
    pub latency_max_us: u64,
    pub latency_mean_us: f64,
    pub latency_stddev_us: f64,
    pub latency_p50_us: u64,
    pub latency_p75_us: u64,
    pub latency_p90_us: u64,
    pub latency_p95_us: u64,
    pub latency_p99_us: u64,
    pub latency_p999_us: u64,

    pub status_codes: HashMap<u16, u64>,
    pub errors: HashMap<ErrorKind, u64>,

    pub timeline: Vec<TimelineBucket>,

    pub check_stats: HashMap<String, CheckStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub elapsed_secs: u32,
    pub requests: u64,
    pub errors: u64,
}

// ============================================================================
// Extraction (v0.8)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Extraction {
    pub name: String,
    pub source: ExtractionSource,
}

#[derive(Debug, Clone)]
pub enum ExtractionSource {
    JsonPath(String),         // json:$.access_token
    Header(String),           // header:X-Request-Id
    Regex(String, usize),     // regex:token=(\w+):1
    Body,                     // body (entire response)
}

impl ExtractionSource {
    pub fn parse(s: &str) -> Result<Self, String> {
        if let Some(path) = s.strip_prefix("json:") {
            Ok(ExtractionSource::JsonPath(path.to_string()))
        } else if let Some(header) = s.strip_prefix("header:") {
            Ok(ExtractionSource::Header(header.to_string()))
        } else if let Some(rest) = s.strip_prefix("regex:") {
            // Format: regex:pattern:group
            let parts: Vec<&str> = rest.rsplitn(2, ':').collect();
            if parts.len() == 2 {
                let group: usize = parts[0].parse().map_err(|_| "Invalid regex group number")?;
                let pattern = parts[1].to_string();
                Ok(ExtractionSource::Regex(pattern, group))
            } else {
                Ok(ExtractionSource::Regex(rest.to_string(), 0))
            }
        } else if s == "body" {
            Ok(ExtractionSource::Body)
        } else {
            Err(format!("Unknown extraction source: '{}'. Expected json:, header:, regex:, or body", s))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub url: String,
    pub method: reqwest::Method,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub weight: u32,
    pub extractions: Vec<Extraction>,
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadConfig {
    pub url: String,
    pub method: reqwest::Method,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub scenarios: Vec<Scenario>,
    pub concurrency: u32,
    pub duration: Duration,
    pub max_requests: u64,
    pub rate: u32,
    pub ramp_up: Duration,
    pub warmup: Duration,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub insecure: bool,
    pub http2: bool,
    pub thresholds: Vec<Threshold>,
    pub checks: Vec<Check>,
    pub stages: Vec<Stage>,
    pub think_time: Option<Duration>,
    pub fail_fast: bool,
    pub arrival_rate: Option<u32>,  // Requests per second
    pub max_vus: Option<u32>,       // Max concurrent requests
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: reqwest::Method::GET,
            headers: Vec::new(),
            body: None,
            scenarios: Vec::new(),
            concurrency: 50,
            duration: Duration::from_secs(10),
            max_requests: 0,
            rate: 0,
            ramp_up: Duration::ZERO,
            warmup: Duration::ZERO,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            insecure: false,
            http2: false,
            thresholds: Vec::new(),
            checks: Vec::new(),
            stages: Vec::new(),
            think_time: None,
            fail_fast: false,
            arrival_rate: None,
            max_vus: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Warmup,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Initializing,
    Running,
    Paused,
    Stopping,
    Completed,
    Cancelled,
    Error,
}

impl RunState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, RunState::Completed | RunState::Cancelled | RunState::Error)
    }
}
