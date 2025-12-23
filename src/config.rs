use crate::cli::RunArgs;
use crate::types::{LoadConfig, Scenario};
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
}

pub fn load_config(path: &Path) -> Result<TomlConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let content = interpolate_env_vars(&content)?;

    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))
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

    let url = args
        .url
        .clone()
        .or(toml.target.url)
        .ok_or("URL is required. Provide via argument or config file.")?;

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
        Some(fs::read_to_string(path)
            .map_err(|e| format!("Failed to read body file '{}': {}", path.display(), e))?)
    } else if let Some(ref body) = args.body {
        Some(body.clone())
    } else if let Some(ref path) = toml.target.body_file {
        Some(fs::read_to_string(path)
            .map_err(|e| format!("Failed to read body file '{}': {}", path, e))?)
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
        toml.target.connect_timeout.unwrap_or(Duration::from_secs(2))
    };

    let insecure = args.insecure || toml.target.insecure;
    let http2 = args.http2 || toml.target.http2;

    // Process scenarios
    let scenarios = process_scenarios(&toml.scenarios)?;

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
    })
}

fn process_scenarios(configs: &[ScenarioConfig]) -> Result<Vec<Scenario>, String> {
    let mut scenarios = Vec::with_capacity(configs.len());

    for (i, cfg) in configs.iter().enumerate() {
        let name = cfg
            .name
            .clone()
            .unwrap_or_else(|| format!("scenario_{}", i + 1));

        let method: reqwest::Method = cfg
            .method
            .to_uppercase()
            .parse()
            .map_err(|_| format!("Invalid HTTP method in scenario '{}': {}", name, cfg.method))?;

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

        scenarios.push(Scenario {
            name,
            url: cfg.url.clone(),
            method,
            headers,
            body,
            weight: cfg.weight,
        });
    }

    Ok(scenarios)
}
