use crate::cli::RunArgs;
use crate::types::{
    BurstConfig, Check, CheckCondition, Extraction, ExtractionSource, FormField, LoadConfig,
    Scenario, Stage, Threshold, ThresholdMetric, ThresholdOp,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize, Default)]
pub struct TomlConfig {
    #[serde(default)]
    pub target: TargetConfig,
    #[serde(default)]
    pub load: LoadSettings,
    #[serde(default)]
    pub websocket: WebSocketConfig,
    #[serde(default)]
    pub scenarios: Vec<ScenarioConfig>,
    #[serde(default)]
    pub thresholds: ThresholdsConfig,
    #[serde(default)]
    pub checks: Vec<CheckConfig>,
    #[serde(default)]
    pub stages: Vec<StageConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StageConfig {
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
    pub target: Option<u32>,      // VU-based (constant VUs mode)
    pub target_rate: Option<u32>, // RPS-based (arrival rate mode)
}

/// Threshold configuration - unknown fields are rejected.
/// Valid metrics: p50_latency_ms, p75_latency_ms, p90_latency_ms, p95_latency_ms,
/// p99_latency_ms, p999_latency_ms, mean_latency_ms, max_latency_ms, error_rate,
/// rps, check_pass_rate
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ThresholdsConfig {
    pub p50_latency_ms: Option<String>,
    pub p75_latency_ms: Option<String>,
    pub p90_latency_ms: Option<String>,
    pub p95_latency_ms: Option<String>,
    pub p99_latency_ms: Option<String>,
    pub p999_latency_ms: Option<String>,
    pub mean_latency_ms: Option<String>,
    pub max_latency_ms: Option<String>,
    pub error_rate: Option<String>,
    pub rps: Option<String>,
    pub check_pass_rate: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckConfig {
    pub name: String,
    pub condition: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScenarioConfig {
    pub name: Option<String>,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_file: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default)]
    pub extract: HashMap<String, String>,
    pub depends_on: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_weight() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
pub struct TargetConfig {
    pub url: Option<String>,
    pub method: Option<String>,
    #[serde(default, with = "humantime_serde::option")]
    pub timeout: Option<Duration>,
    #[serde(default, with = "humantime_serde::option")]
    pub connect_timeout: Option<Duration>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_file: Option<String>,
    /// Body lines from file (one per request, round-robin)
    pub body_lines_file: Option<String>,
    #[serde(default)]
    pub insecure: bool,
    #[serde(default)]
    pub http2: bool,
    #[serde(default)]
    pub cookie_jar: bool,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    /// HTTP/HTTPS/SOCKS5 proxy URL
    pub proxy: Option<String>,
    /// Basic authentication credentials (user:password)
    pub basic_auth: Option<String>,
    /// Client certificate file path (PEM format) for mTLS
    pub cert: Option<String>,
    /// Client private key file path (PEM format) for mTLS
    pub key: Option<String>,
    /// CA certificate file path (PEM format) for custom root CA
    pub cacert: Option<String>,
    /// Multipart form fields (name=value or name=@filepath for files)
    #[serde(default)]
    pub form_data: Vec<String>,
    /// Disable HTTP keepalive (new connection per request)
    #[serde(default)]
    pub disable_keepalive: bool,
    /// Generate random URLs from regex pattern
    pub rand_regex_url: Option<String>,
    /// Read URLs from file (one per line, round-robin)
    pub urls_from_file: Option<String>,
    /// Override host resolution (HOST:PORT:TARGET_HOST:TARGET_PORT)
    pub connect_to: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LoadSettings {
    pub concurrency: Option<u32>,
    #[serde(default, with = "humantime_serde::option")]
    pub duration: Option<Duration>,
    pub max_requests: Option<u64>,
    pub rate: Option<u32>,
    #[serde(default, with = "humantime_serde::option")]
    pub ramp_up: Option<Duration>,
    #[serde(default, with = "humantime_serde::option")]
    pub warmup: Option<Duration>,
    #[serde(default, with = "humantime_serde::option")]
    pub think_time: Option<Duration>,
    pub arrival_rate: Option<u32>,
    pub max_vus: Option<u32>,
    /// Requests per burst (enables burst mode)
    pub burst_rate: Option<u32>,
    /// Delay between bursts
    #[serde(default, with = "humantime_serde::option")]
    pub burst_delay: Option<Duration>,
}

#[derive(Debug, Deserialize, Default)]
pub struct WebSocketConfig {
    /// Message send interval (e.g., "100ms")
    #[serde(default, with = "humantime_serde::option")]
    pub message_interval: Option<Duration>,
    /// Mode: "echo" (default) or "fire_and_forget"
    #[serde(default)]
    pub mode: Option<String>,
}

pub fn load_config(path: &Path) -> Result<TomlConfig, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;

    let content = interpolate_env_vars(&content)?;

    toml::from_str(&content).map_err(|e| {
        let err_str = e.to_string();
        // Provide helpful message for unknown threshold metrics
        if err_str.contains("thresholds") && err_str.contains("unknown field") {
            format!(
                "Unknown threshold metric in config file.\n\
                 Valid metrics: p50_latency_ms, p75_latency_ms, p90_latency_ms, p95_latency_ms,\n\
                 p99_latency_ms, p999_latency_ms, mean_latency_ms, max_latency_ms,\n\
                 error_rate, rps, check_pass_rate\n\n\
                 Error: {}",
                e
            )
        } else {
            format!("Failed to parse config file: {}", e)
        }
    })
}

fn interpolate_env_vars(content: &str) -> Result<String, String> {
    let mut result = content.to_string();
    let re = regex_lite::Regex::new(r"\$\{([^}]+)\}").unwrap();

    for cap in re.captures_iter(content) {
        let full_match = cap.get(0).unwrap().as_str();
        let var_expr = cap.get(1).unwrap().as_str();

        let (var_name, default) = if let Some(pos) = var_expr.find(":-") {
            (&var_expr[..pos], Some(&var_expr[pos + 2..]))
        } else {
            (var_expr, None)
        };

        // Skip runtime variables (lowercase names like extracted values)
        // Only substitute env vars (typically UPPER_CASE)
        if var_name.chars().all(|c| c.is_lowercase() || c == '_') {
            continue; // Leave runtime variables unchanged
        }

        let value = match std::env::var(var_name) {
            Ok(v) => v,
            Err(_) => match default {
                Some(d) => d.to_string(),
                None => return Err(format!("Environment variable '{}' not set", var_name)),
            },
        };

        result = result.replace(full_match, &value);
    }

    Ok(result)
}

pub fn merge_config(args: &RunArgs, toml: Option<TomlConfig>) -> Result<LoadConfig, String> {
    let toml = toml.unwrap_or_default();

    let has_scenarios = !toml.scenarios.is_empty();

    // URL can come from: regular URL arg, rand_regex_url, first line of urls_from_file, or config
    let url = args
        .url
        .clone()
        .or_else(|| args.rand_regex_url.clone())
        .or_else(|| {
            // For urls_from_file, use first URL as the base
            args.urls_from_file.as_ref().and_then(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|content| content.lines().next().map(String::from))
            })
        })
        .or(toml.target.url)
        .or(toml.target.rand_regex_url.clone())
        .or_else(|| {
            toml.target.urls_from_file.as_ref().and_then(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|content| content.lines().next().map(String::from))
            })
        })
        .ok_or_else(|| {
            if has_scenarios {
                "URL is required in [target] section even when using [[scenarios]].\n\
                 The target URL is used as a fallback and for metadata.\n\
                 Add: [target]\n      url = \"https://your-api.com\""
                    .to_string()
            } else {
                "URL is required. Provide via argument, --rand-regex-url, --urls-from-file, or [target] section in config file.".to_string()
            }
        })?;

    let method_str = if args.method != "GET" {
        args.method.clone()
    } else {
        toml.target.method.unwrap_or_else(|| "GET".to_string())
    };

    let method: reqwest::Method = method_str
        .to_uppercase()
        .parse()
        .map_err(|_| format!("Invalid HTTP method: {}", method_str))?;

    let mut headers = args.parse_headers()?;
    for (k, v) in toml.target.headers {
        if !headers.iter().any(|(hk, _)| hk.eq_ignore_ascii_case(&k)) {
            headers.push((k, v));
        }
    }

    // Check if gRPC mode is active (needed to decide how to load body)
    #[cfg(feature = "grpc")]
    let is_grpc_mode = args
        .grpc_service
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    #[cfg(not(feature = "grpc"))]
    let is_grpc_mode = false;

    // Load body from file if specified
    // Skip read_to_string for gRPC mode with body_file (binary files handled by body_bytes)
    let body = if is_grpc_mode && (args.body_file.is_some() || toml.target.body_file.is_some()) {
        // gRPC mode with body file - skip string loading, body_bytes will handle binary
        args.body.clone().or_else(|| toml.target.body.clone())
    } else if let Some(ref path) = args.body_file {
        Some(
            fs::read_to_string(path)
                .map_err(|e| format!("Failed to read body file '{}': {}", path.display(), e))?,
        )
    } else if let Some(ref body) = args.body {
        Some(body.clone())
    } else if let Some(ref path) = toml.target.body_file {
        Some(
            fs::read_to_string(path)
                .map_err(|e| format!("Failed to read body file '{}': {}", path, e))?,
        )
    } else {
        toml.target.body.clone()
    };

    let concurrency = if args.concurrency != 50 {
        args.concurrency
    } else {
        toml.load.concurrency.unwrap_or(50)
    };

    let duration = if args.duration != Duration::from_secs(10) {
        args.duration
    } else {
        toml.load.duration.unwrap_or(Duration::from_secs(10))
    };

    let max_requests = if args.max_requests != 0 {
        args.max_requests
    } else {
        toml.load.max_requests.unwrap_or(0)
    };

    let rate = if args.rate != 0 {
        args.rate
    } else {
        toml.load.rate.unwrap_or(0)
    };

    let ramp_up = if args.ramp_up != Duration::ZERO {
        args.ramp_up
    } else {
        toml.load.ramp_up.unwrap_or(Duration::ZERO)
    };

    let warmup = if args.warmup != Duration::ZERO {
        args.warmup
    } else {
        toml.load.warmup.unwrap_or(Duration::ZERO)
    };

    let timeout = if args.timeout != Duration::from_secs(5) {
        args.timeout
    } else {
        toml.target.timeout.unwrap_or(Duration::from_secs(5))
    };

    let connect_timeout = if args.connect_timeout != Duration::from_secs(2) {
        args.connect_timeout
    } else {
        toml.target
            .connect_timeout
            .unwrap_or(Duration::from_secs(2))
    };

    let insecure = args.insecure || toml.target.insecure;
    let http2 = args.http2 || toml.target.http2;
    #[cfg(feature = "http3")]
    let http3 = args.http3;
    #[cfg(feature = "grpc")]
    let grpc_service = args.grpc_service.clone();
    #[cfg(feature = "grpc")]
    let grpc_method = args.grpc_method.clone();
    let cookie_jar = args.cookie_jar || toml.target.cookie_jar;
    let follow_redirects = !args.no_follow_redirects && toml.target.follow_redirects;
    let disable_keepalive = args.disable_keepalive || toml.target.disable_keepalive;

    // Validate HTTP/3 requires HTTPS
    #[cfg(feature = "http3")]
    if http3 && !url.starts_with("https://") {
        return Err("HTTP/3 requires HTTPS URL (https://)".to_string());
    }

    // Validate gRPC configuration
    #[cfg(feature = "grpc")]
    {
        let has_service = grpc_service
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let has_method = grpc_method.as_ref().map(|s| !s.is_empty()).unwrap_or(false);

        if has_service != has_method {
            return Err(
                "Both --grpc-service and --grpc-method must be provided together".to_string(),
            );
        }

        // Reject empty strings
        if grpc_service.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
            return Err("--grpc-service cannot be empty".to_string());
        }

        // --insecure is not supported for gRPC
        if has_service && insecure {
            return Err(
                "--insecure is not supported for gRPC. Use http:// URL for unencrypted connections."
                    .to_string(),
            );
        }
    }

    // Detect protocol conflicts (HTTP/3 + gRPC)
    #[cfg(all(feature = "http3", feature = "grpc"))]
    {
        let has_grpc = grpc_service
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if http3 && has_grpc {
            return Err("Cannot use --http3 with --grpc-service. Choose one protocol.".to_string());
        }
    }

    // Load body as binary bytes for gRPC mode (supports binary protobuf)
    #[cfg(feature = "grpc")]
    let body_bytes: Option<Vec<u8>> =
        {
            let has_grpc = grpc_service
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            if has_grpc {
                if let Some(ref path) = args.body_file {
                    Some(fs::read(path).map_err(|e| {
                        format!("Failed to read body file '{}': {}", path.display(), e)
                    })?)
                } else if let Some(ref body) = args.body {
                    Some(body.as_bytes().to_vec())
                } else if let Some(ref path) = toml.target.body_file {
                    Some(
                        fs::read(path)
                            .map_err(|e| format!("Failed to read body file '{}': {}", path, e))?,
                    )
                } else {
                    toml.target
                        .body
                        .as_ref()
                        .map(|body| body.as_bytes().to_vec())
                }
            } else {
                None
            }
        };

    // Process scenarios
    let scenarios = process_scenarios(&toml.scenarios)?;

    // Process thresholds
    let thresholds = parse_thresholds(&toml.thresholds)?;

    // Process checks
    let checks = parse_checks(&toml.checks)?;

    // Process stages
    let stages = process_stages(&toml.stages)?;

    // Think time - CLI takes precedence
    let think_time = args.think_time.or(toml.load.think_time);

    // Fail fast
    let fail_fast = args.fail_fast;

    // Arrival rate mode - CLI takes precedence
    let arrival_rate = args.arrival_rate.or(toml.load.arrival_rate);
    let max_vus = if args.max_vus != 100 {
        Some(args.max_vus)
    } else {
        toml.load.max_vus.or(Some(100))
    };

    // Validate: can't use arrival_rate with VU-based stages
    if arrival_rate.is_some() && !stages.is_empty() && stages.iter().any(|s| s.target.is_some()) {
        return Err(
            "Cannot use --arrival-rate with VU-based stages. Use target_rate in stages instead."
                .to_string(),
        );
    }

    // Auto-enable latency correction for arrival rate mode (unless explicitly disabled)
    let latency_correction = !args.no_latency_correction
        && (arrival_rate.is_some() || stages.iter().any(|s| s.target_rate.is_some()));

    // WebSocket config - CLI takes precedence
    let ws_message_interval = if args.ws_message_interval != Duration::from_millis(100) {
        args.ws_message_interval
    } else {
        toml.websocket
            .message_interval
            .unwrap_or(Duration::from_millis(100))
    };

    let ws_mode = if args.ws_fire_and_forget {
        crate::types::WsMode::FireAndForget
    } else {
        match toml.websocket.mode.as_deref() {
            Some("fire_and_forget") => crate::types::WsMode::FireAndForget,
            _ => crate::types::WsMode::Echo,
        }
    };

    // Proxy - CLI takes precedence
    let proxy = args.proxy.clone().or(toml.target.proxy);

    // Basic auth - CLI takes precedence
    let basic_auth = if let Some(ref auth_str) = args.basic_auth {
        Some(parse_basic_auth(auth_str)?)
    } else if let Some(ref auth_str) = toml.target.basic_auth {
        Some(parse_basic_auth(auth_str)?)
    } else {
        None
    };

    // mTLS certificates - CLI takes precedence
    let client_cert = args
        .cert
        .clone()
        .or_else(|| toml.target.cert.as_ref().map(std::path::PathBuf::from));
    let client_key = args
        .key
        .clone()
        .or_else(|| toml.target.key.as_ref().map(std::path::PathBuf::from));
    let ca_cert = args
        .cacert
        .clone()
        .or_else(|| toml.target.cacert.as_ref().map(std::path::PathBuf::from));

    // Validate: --cert and --key must be used together
    if client_cert.is_some() != client_key.is_some() {
        return Err("--cert and --key must be specified together for mTLS".to_string());
    }

    // Validate cert/key files exist
    if let Some(ref path) = client_cert
        && !path.exists()
    {
        return Err(format!(
            "Client certificate file not found: {}",
            path.display()
        ));
    }
    if let Some(ref path) = client_key
        && !path.exists()
    {
        return Err(format!("Client key file not found: {}", path.display()));
    }
    if let Some(ref path) = ca_cert
        && !path.exists()
    {
        return Err(format!("CA certificate file not found: {}", path.display()));
    }

    // Multipart form fields - combine CLI args and config
    let mut form_fields = Vec::new();
    for field_str in &args.form {
        let field = FormField::parse(field_str)?;
        form_fields.push(field);
    }
    for field_str in &toml.target.form_data {
        let field = FormField::parse(field_str)?;
        form_fields.push(field);
    }

    // Validate: --form and --body are mutually exclusive
    if !form_fields.is_empty() && body.is_some() {
        return Err("--form and --body are mutually exclusive. Use one or the other.".to_string());
    }

    // Validate file paths in form fields exist
    for field in &form_fields {
        if let FormField::File { path, name, .. } = field
            && !path.exists()
        {
            return Err(format!(
                "Form file not found for field '{}': {}",
                name,
                path.display()
            ));
        }
    }

    // v1.3 features: rand_regex_url, urls_from_file, body_lines, connect_to, burst mode, db_url

    // rand_regex_url - CLI takes precedence
    let rand_regex_url = args.rand_regex_url.clone().or(toml.target.rand_regex_url);

    // Load URLs from file
    let url_list: Option<Vec<String>> = if let Some(ref path) = args.urls_from_file {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read URLs file '{}': {}", path.display(), e))?;
        let urls: Vec<String> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();
        if urls.is_empty() {
            return Err(format!("URLs file '{}' is empty", path.display()));
        }
        Some(urls)
    } else if let Some(ref path) = toml.target.urls_from_file {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read URLs file '{}': {}", path, e))?;
        let urls: Vec<String> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();
        if urls.is_empty() {
            return Err(format!("URLs file '{}' is empty", path));
        }
        Some(urls)
    } else {
        None
    };

    // Validate: rand_regex_url and urls_from_file are mutually exclusive
    if rand_regex_url.is_some() && url_list.is_some() {
        return Err(
            "--rand-regex-url and --urls-from-file are mutually exclusive".to_string(),
        );
    }

    // Load body lines from file
    let body_lines: Option<Vec<String>> = if let Some(ref path) = args.body_lines_file {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read body lines file '{}': {}", path.display(), e))?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        if lines.is_empty() {
            return Err(format!("Body lines file '{}' is empty", path.display()));
        }
        Some(lines)
    } else if let Some(ref path) = toml.target.body_lines_file {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read body lines file '{}': {}", path, e))?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        if lines.is_empty() {
            return Err(format!("Body lines file '{}' is empty", path));
        }
        Some(lines)
    } else {
        None
    };

    // Validate: body_lines and body/body_file are mutually exclusive
    if body_lines.is_some() && body.is_some() {
        return Err(
            "-Z/--body-lines and --body/--body-file are mutually exclusive".to_string(),
        );
    }

    // Parse connect_to (HOST:PORT:TARGET_HOST:TARGET_PORT or HOST:TARGET_IP:TARGET_PORT)
    let connect_to: Option<(String, std::net::SocketAddr)> =
        if let Some(ref mapping) = args.connect_to {
            Some(parse_connect_to(mapping)?)
        } else if let Some(ref mapping) = toml.target.connect_to {
            Some(parse_connect_to(mapping)?)
        } else {
            None
        };

    // Burst mode configuration
    let burst_config = if let Some(burst_rate) = args.burst_rate {
        let burst_delay = args
            .burst_delay
            .ok_or("--burst-rate requires --burst-delay")?;
        Some(BurstConfig {
            requests_per_burst: burst_rate,
            delay_between_bursts: burst_delay,
        })
    } else if let Some(burst_rate) = toml.load.burst_rate {
        let burst_delay = toml
            .load
            .burst_delay
            .ok_or("burst_rate requires burst_delay in config")?;
        Some(BurstConfig {
            requests_per_burst: burst_rate,
            delay_between_bursts: burst_delay,
        })
    } else {
        None
    };

    // Validate: burst mode is incompatible with arrival rate
    if burst_config.is_some() && arrival_rate.is_some() {
        return Err("Burst mode (--burst-rate) is incompatible with --arrival-rate".to_string());
    }

    // db_url for SQLite logging
    let db_url = args.db_url.clone();

    Ok(LoadConfig {
        url,
        method,
        headers,
        body,
        scenarios,
        concurrency,
        duration,
        max_requests,
        rate,
        ramp_up,
        warmup,
        timeout,
        connect_timeout,
        insecure,
        http2,
        #[cfg(feature = "http3")]
        http3,
        #[cfg(feature = "grpc")]
        grpc_service,
        #[cfg(feature = "grpc")]
        grpc_method,
        #[cfg(feature = "grpc")]
        body_bytes,
        cookie_jar,
        follow_redirects,
        disable_keepalive,
        thresholds,
        checks,
        stages,
        think_time,
        fail_fast,
        arrival_rate,
        max_vus,
        latency_correction,
        ws_mode,
        ws_message_interval,
        proxy,
        basic_auth,
        client_cert,
        client_key,
        ca_cert,
        form_fields,
        rand_regex_url,
        url_list,
        body_lines,
        connect_to,
        burst_config,
        db_url,
    })
}

/// Parse connect_to mapping string
/// Format: "HOST:PORT:TARGET_IP:TARGET_PORT" or "HOST:TARGET_IP:TARGET_PORT"
fn parse_connect_to(mapping: &str) -> Result<(String, std::net::SocketAddr), String> {
    let parts: Vec<&str> = mapping.split(':').collect();

    match parts.len() {
        // HOST:TARGET_IP:TARGET_PORT (e.g., "example.com:127.0.0.1:8080")
        3 => {
            let host = parts[0].to_string();
            let target_addr = format!("{}:{}", parts[1], parts[2]);
            let socket_addr: std::net::SocketAddr = target_addr
                .parse()
                .map_err(|e| format!("Invalid target address '{}': {}", target_addr, e))?;
            Ok((host, socket_addr))
        }
        // HOST:PORT:TARGET_IP:TARGET_PORT (e.g., "example.com:443:127.0.0.1:8080")
        4 => {
            let target_addr = format!("{}:{}", parts[2], parts[3]);
            let socket_addr: std::net::SocketAddr = target_addr
                .parse()
                .map_err(|e| format!("Invalid target address '{}': {}", target_addr, e))?;
            // For reqwest resolve(), we only need the hostname, not the port
            Ok((parts[0].to_string(), socket_addr))
        }
        _ => Err(format!(
            "Invalid connect-to format: '{}'. Expected 'HOST:TARGET_IP:TARGET_PORT' or 'HOST:PORT:TARGET_IP:TARGET_PORT'",
            mapping
        )),
    }
}

/// Parse basic auth string "user:password" or "user" into (user, Option<password>)
fn parse_basic_auth(s: &str) -> Result<(String, Option<String>), String> {
    if let Some(pos) = s.find(':') {
        let user = s[..pos].to_string();
        let pass = s[pos + 1..].to_string();
        if user.is_empty() {
            return Err("Basic auth username cannot be empty".to_string());
        }
        Ok((user, Some(pass)))
    } else {
        if s.is_empty() {
            return Err("Basic auth username cannot be empty".to_string());
        }
        Ok((s.to_string(), None))
    }
}

fn process_scenarios(configs: &[ScenarioConfig]) -> Result<Vec<Scenario>, String> {
    let mut scenarios = Vec::with_capacity(configs.len());

    for (i, cfg) in configs.iter().enumerate() {
        let name = cfg
            .name
            .clone()
            .unwrap_or_else(|| format!("scenario_{}", i + 1));

        let method: reqwest::Method =
            cfg.method.to_uppercase().parse().map_err(|_| {
                format!("Invalid HTTP method in scenario '{}': {}", name, cfg.method)
            })?;

        let headers: Vec<(String, String)> = cfg
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Load body from file if specified
        let body = if let Some(ref path) = cfg.body_file {
            Some(
                fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read body file for '{}': {}", name, e))?,
            )
        } else {
            cfg.body.clone()
        };

        // Parse extractions
        let mut extractions = Vec::new();
        for (var_name, source_str) in &cfg.extract {
            let source = ExtractionSource::parse(source_str)
                .map_err(|e| format!("Invalid extraction in '{}': {}", name, e))?;
            extractions.push(Extraction {
                name: var_name.clone(),
                source,
            });
        }

        scenarios.push(Scenario {
            name,
            url: cfg.url.clone(),
            method,
            headers,
            body,
            weight: cfg.weight,
            extractions,
            depends_on: cfg.depends_on.clone(),
            tags: cfg.tags.clone(),
        });
    }

    Ok(scenarios)
}

fn parse_thresholds(config: &ThresholdsConfig) -> Result<Vec<Threshold>, String> {
    let mut thresholds = Vec::new();

    let entries: Vec<(ThresholdMetric, &Option<String>)> = vec![
        (ThresholdMetric::P50LatencyMs, &config.p50_latency_ms),
        (ThresholdMetric::P75LatencyMs, &config.p75_latency_ms),
        (ThresholdMetric::P90LatencyMs, &config.p90_latency_ms),
        (ThresholdMetric::P95LatencyMs, &config.p95_latency_ms),
        (ThresholdMetric::P99LatencyMs, &config.p99_latency_ms),
        (ThresholdMetric::P999LatencyMs, &config.p999_latency_ms),
        (ThresholdMetric::MeanLatencyMs, &config.mean_latency_ms),
        (ThresholdMetric::MaxLatencyMs, &config.max_latency_ms),
        (ThresholdMetric::ErrorRate, &config.error_rate),
        (ThresholdMetric::Rps, &config.rps),
        (ThresholdMetric::CheckPassRate, &config.check_pass_rate),
    ];

    for (metric, value) in entries {
        if let Some(expr) = value {
            let threshold = parse_threshold_expr(metric, expr)?;
            thresholds.push(threshold);
        }
    }

    Ok(thresholds)
}

fn parse_threshold_expr(metric: ThresholdMetric, expr: &str) -> Result<Threshold, String> {
    let expr = expr.trim();

    // Parse operator and value: "< 500", "<= 500", "> 100", ">= 100", "== 500"
    let (operator, value_str) = if let Some(rest) = expr.strip_prefix("<=") {
        (ThresholdOp::Lte, rest.trim())
    } else if let Some(rest) = expr.strip_prefix(">=") {
        (ThresholdOp::Gte, rest.trim())
    } else if let Some(rest) = expr.strip_prefix("==") {
        (ThresholdOp::Eq, rest.trim())
    } else if let Some(rest) = expr.strip_prefix('<') {
        (ThresholdOp::Lt, rest.trim())
    } else if let Some(rest) = expr.strip_prefix('>') {
        (ThresholdOp::Gt, rest.trim())
    } else {
        return Err(format!(
            "Invalid threshold expression for '{}': '{}'. Expected format: '< 500' or '>= 100'",
            metric.as_str(),
            expr
        ));
    };

    let value: f64 = value_str.parse().map_err(|_| {
        format!(
            "Invalid threshold value for '{}': '{}'. Expected a number.",
            metric.as_str(),
            value_str
        )
    })?;

    Ok(Threshold {
        metric,
        operator,
        value,
    })
}

fn parse_checks(configs: &[CheckConfig]) -> Result<Vec<Check>, String> {
    let mut checks = Vec::with_capacity(configs.len());

    for cfg in configs {
        let condition = parse_check_condition(&cfg.condition)
            .map_err(|e| format!("Invalid check condition for '{}': {}", cfg.name, e))?;

        checks.push(Check {
            name: cfg.name.clone(),
            condition,
        });
    }

    Ok(checks)
}

fn parse_check_condition(expr: &str) -> Result<CheckCondition, String> {
    let expr = expr.trim();

    // status == 200
    if let Some(rest) = expr.strip_prefix("status") {
        let rest = rest.trim();
        if let Some(value) = rest.strip_prefix("==") {
            let status: u16 = value.trim().parse().map_err(|_| "Invalid status code")?;
            return Ok(CheckCondition::StatusEquals(status));
        }
        if let Some(value) = rest.strip_prefix("<") {
            let status: u16 = value.trim().parse().map_err(|_| "Invalid status code")?;
            return Ok(CheckCondition::StatusLt(status));
        }
        if let Some(value) = rest.strip_prefix(">") {
            let status: u16 = value.trim().parse().map_err(|_| "Invalid status code")?;
            return Ok(CheckCondition::StatusGt(status));
        }
        if let Some(rest) = rest.strip_prefix("in") {
            // status in [200, 201, 204]
            let rest = rest.trim();
            if rest.starts_with('[') && rest.ends_with(']') {
                let inner = &rest[1..rest.len() - 1];
                let codes: Result<Vec<u16>, _> =
                    inner.split(',').map(|s| s.trim().parse::<u16>()).collect();
                let codes = codes.map_err(|_| "Invalid status codes in list")?;
                return Ok(CheckCondition::StatusIn(codes));
            }
        }
        return Err(format!("Unknown status condition: '{}'", expr));
    }

    // body contains "..."
    if let Some(rest) = expr.strip_prefix("body") {
        let rest = rest.trim();
        if let Some(rest) = rest.strip_prefix("contains") {
            let needle = parse_quoted_string(rest.trim())?;
            return Ok(CheckCondition::BodyContains(needle));
        }
        if let Some(rest) = rest.strip_prefix("not contains") {
            let needle = parse_quoted_string(rest.trim())?;
            return Ok(CheckCondition::BodyNotContains(needle));
        }
        if let Some(rest) = rest.strip_prefix("matches") {
            let pattern = parse_quoted_string(rest.trim())?;
            let re =
                regex_lite::Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;
            return Ok(CheckCondition::BodyMatches(re));
        }
        return Err(format!("Unknown body condition: '{}'", expr));
    }

    Err(format!(
        "Unknown condition: '{}'. Expected 'status ...' or 'body ...'",
        expr
    ))
}

fn parse_quoted_string(s: &str) -> Result<String, String> {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Ok(s[1..s.len() - 1].to_string())
    } else {
        Err(format!("Expected quoted string, got: '{}'", s))
    }
}

fn process_stages(configs: &[StageConfig]) -> Result<Vec<Stage>, String> {
    let mut stages = Vec::with_capacity(configs.len());

    for (i, cfg) in configs.iter().enumerate() {
        // Validate: can't have both target and target_rate
        if cfg.target.is_some() && cfg.target_rate.is_some() {
            return Err(format!(
                "Stage {} cannot have both 'target' (VUs) and 'target_rate' (RPS)",
                i + 1
            ));
        }

        // Validate: must have at least one
        if cfg.target.is_none() && cfg.target_rate.is_none() {
            return Err(format!(
                "Stage {} must have either 'target' (VUs) or 'target_rate' (RPS)",
                i + 1
            ));
        }

        stages.push(Stage {
            duration: cfg.duration,
            target: cfg.target,
            target_rate: cfg.target_rate,
        });
    }

    // Validate: all stages must use the same mode
    let has_vu_stages = stages.iter().any(|s| s.target.is_some());
    let has_rate_stages = stages.iter().any(|s| s.target_rate.is_some());
    if has_vu_stages && has_rate_stages {
        return Err(
            "Cannot mix VU-based stages (target) with rate-based stages (target_rate)".to_string(),
        );
    }

    Ok(stages)
}
