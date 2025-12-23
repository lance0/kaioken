use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
}

impl RequestResult {
    pub fn success(latency_us: u64, status: u16, bytes_received: u64) -> Self {
        Self {
            latency_us,
            status: Some(status),
            error: None,
            bytes_received,
        }
    }

    pub fn error(latency_us: u64, kind: ErrorKind) -> Self {
        Self {
            latency_us,
            status: None,
            error: Some(kind),
            bytes_received: 0,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub elapsed_secs: u32,
    pub requests: u64,
    pub errors: u64,
}

#[derive(Debug, Clone)]
pub struct LoadConfig {
    pub url: String,
    pub method: reqwest::Method,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
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
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: reqwest::Method::GET,
            headers: Vec::new(),
            body: None,
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
