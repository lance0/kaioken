use crate::cli::Cli;
use crate::types::LoadConfig;
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
    #[serde(default)]
    pub insecure: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct LoadSettings {
    pub concurrency: Option<u32>,
    #[serde(default, with = "humantime_serde::option")]
    pub duration: Option<Duration>,
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

pub fn merge_config(cli: &Cli, toml: Option<TomlConfig>) -> Result<LoadConfig, String> {
    let toml = toml.unwrap_or_default();

    let url = cli
        .url
        .clone()
        .or(toml.target.url)
        .ok_or("URL is required. Provide via argument or config file.")?;

    let method_str = if cli.method != "GET" {
        cli.method.clone()
    } else {
        toml.target.method.unwrap_or_else(|| "GET".to_string())
    };

    let method: reqwest::Method = method_str
        .to_uppercase()
        .parse()
        .map_err(|_| format!("Invalid HTTP method: {}", method_str))?;

    let mut headers = cli.parse_headers()?;
    for (k, v) in toml.target.headers {
        if !headers.iter().any(|(hk, _)| hk.eq_ignore_ascii_case(&k)) {
            headers.push((k, v));
        }
    }

    let body = cli.body.clone().or(toml.target.body);

    let concurrency = if cli.concurrency != 50 {
        cli.concurrency
    } else {
        toml.load.concurrency.unwrap_or(50)
    };

    let duration = if cli.duration != Duration::from_secs(10) {
        cli.duration
    } else {
        toml.load.duration.unwrap_or(Duration::from_secs(10))
    };

    let rate = if cli.rate != 0 {
        cli.rate
    } else {
        toml.load.rate.unwrap_or(0)
    };

    let ramp_up = if cli.ramp_up != Duration::ZERO {
        cli.ramp_up
    } else {
        toml.load.ramp_up.unwrap_or(Duration::ZERO)
    };

    let warmup = if cli.warmup != Duration::ZERO {
        cli.warmup
    } else {
        toml.load.warmup.unwrap_or(Duration::ZERO)
    };

    let timeout = if cli.timeout != Duration::from_secs(5) {
        cli.timeout
    } else {
        toml.target.timeout.unwrap_or(Duration::from_secs(5))
    };

    let connect_timeout = if cli.connect_timeout != Duration::from_secs(2) {
        cli.connect_timeout
    } else {
        toml.target.connect_timeout.unwrap_or(Duration::from_secs(2))
    };

    let insecure = cli.insecure || toml.target.insecure;

    Ok(LoadConfig {
        url,
        method,
        headers,
        body,
        concurrency,
        duration,
        rate,
        ramp_up,
        warmup,
        timeout,
        connect_timeout,
        insecure,
    })
}
