use crate::cli::RunArgs;
use crate::types::{
    Check, CheckCondition, Extraction, ExtractionSource, LoadConfig, Scenario, Stage, Threshold,
    ThresholdMetric, ThresholdOp,
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
    #[serde(default)]
    pub insecure: bool,
    #[serde(default)]
    pub http2: bool,
    #[serde(default)]
    pub cookie_jar: bool,
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
    let url = args.url.clone().or(toml.target.url).ok_or_else(|| {
        if has_scenarios {
            "URL is required in [target] section even when using [[scenarios]].\n\
                 The target URL is used as a fallback and for metadata.\n\
                 Add: [target]\n      url = \"https://your-api.com\""
                .to_string()
        } else {
            "URL is required. Provide via argument or [target] section in config file.".to_string()
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

    // Load body from file if specified
    let body = if let Some(ref path) = args.body_file {
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
        toml.target.body
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
    let cookie_jar = args.cookie_jar || toml.target.cookie_jar;

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
        cookie_jar,
        thresholds,
        checks,
        stages,
        think_time,
        fail_fast,
        arrival_rate,
        max_vus,
        latency_correction,
    })
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
