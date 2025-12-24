use crate::cli::CompareArgs;
use crate::output::json::JsonOutput;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CompareResult {
    pub baseline_file: String,
    pub current_file: String,
    pub metrics: Vec<MetricComparison>,
    pub regressions: Vec<Regression>,
    pub warnings: Vec<String>,
    pub has_regressions: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricComparison {
    pub name: String,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub delta_pct: f64,
    pub unit: String,
    pub improved: bool,
    pub regressed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Regression {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub delta_pct: f64,
    pub threshold_pct: f64,
}

pub fn compare_results(args: &CompareArgs) -> Result<CompareResult, String> {
    let baseline = load_json(&args.baseline)?;
    let current = load_json(&args.current)?;

    let mut metrics = Vec::new();
    let mut regressions = Vec::new();
    let mut warnings = Vec::new();

    // Determine load models
    let baseline_model = baseline
        .metadata
        .load
        .load_model
        .as_deref()
        .unwrap_or("closed");
    let current_model = current
        .metadata
        .load
        .load_model
        .as_deref()
        .unwrap_or("closed");
    let baseline_is_open = baseline_model == "open";
    let current_is_open = current_model == "open";

    // Print load model metadata
    eprintln!();
    if baseline_is_open {
        let rate = baseline.metadata.load.arrival_rate.unwrap_or(0);
        let max_vus = baseline.metadata.load.max_vus.unwrap_or(0);
        eprintln!(
            "Baseline:  Open (arrival rate)  target={}  max_vus={}",
            rate, max_vus
        );
    } else {
        let vus = baseline.metadata.load.concurrency;
        eprintln!("Baseline:  Closed (VU-driven)   vus={}", vus);
    }
    if current_is_open {
        let rate = current.metadata.load.arrival_rate.unwrap_or(0);
        let max_vus = current.metadata.load.max_vus.unwrap_or(0);
        eprintln!(
            "Candidate: Open (arrival rate)  target={}  max_vus={}",
            rate, max_vus
        );
    } else {
        let vus = current.metadata.load.concurrency;
        eprintln!("Candidate: Closed (VU-driven)   vus={}", vus);
    }
    eprintln!();

    // Fail if load models differ (unless --force)
    if baseline_is_open != current_is_open {
        if !args.force {
            return Err(format!(
                "Cannot compare {} vs {} runs. Use --force to compare anyway.",
                if baseline_is_open { "Open" } else { "Closed" },
                if current_is_open { "Open" } else { "Closed" }
            ));
        }
        warnings.push(format!(
            "Load models differ: {} vs {} (forced comparison)",
            if baseline_is_open { "Open" } else { "Closed" },
            if current_is_open { "Open" } else { "Closed" }
        ));
    }

    // Model-specific parameter validation
    if baseline_is_open && current_is_open {
        // Both open: check arrival rate parameters
        let base_rate = baseline.metadata.load.arrival_rate.unwrap_or(0);
        let curr_rate = current.metadata.load.arrival_rate.unwrap_or(0);
        if base_rate != curr_rate {
            warnings.push(format!(
                "Target RPS differs: {} vs {}",
                base_rate, curr_rate
            ));
        }
        let base_max = baseline.metadata.load.max_vus.unwrap_or(0);
        let curr_max = current.metadata.load.max_vus.unwrap_or(0);
        if base_max != curr_max {
            warnings.push(format!("Max VUs differs: {} vs {}", base_max, curr_max));
        }
    } else if !baseline_is_open && !current_is_open {
        // Both closed: check VU parameters
        if baseline.metadata.load.concurrency != current.metadata.load.concurrency {
            warnings.push(format!(
                "Concurrency differs: {} vs {}",
                baseline.metadata.load.concurrency, current.metadata.load.concurrency
            ));
        }
    }

    // Check config compatibility
    if baseline.metadata.target.url != current.metadata.target.url {
        warnings.push(format!(
            "URL differs: '{}' vs '{}'",
            baseline.metadata.target.url, current.metadata.target.url
        ));
    }
    if baseline.metadata.target.method != current.metadata.target.method {
        warnings.push(format!(
            "Method differs: {} vs {}",
            baseline.metadata.target.method, current.metadata.target.method
        ));
    }

    // RPS comparison (higher is better)
    let rps_cmp = compare_metric(
        "Requests/sec",
        baseline.summary.requests_per_sec,
        current.summary.requests_per_sec,
        "req/s",
        true, // higher is better
    );
    if rps_cmp.regressed && rps_cmp.delta_pct.abs() > args.threshold_rps {
        regressions.push(Regression {
            metric: "Requests/sec".to_string(),
            baseline: rps_cmp.baseline,
            current: rps_cmp.current,
            delta_pct: rps_cmp.delta_pct,
            threshold_pct: args.threshold_rps,
        });
    }
    metrics.push(rps_cmp);

    // Total requests
    metrics.push(compare_metric(
        "Total requests",
        baseline.summary.total_requests as f64,
        current.summary.total_requests as f64,
        "",
        true,
    ));

    // Error rate (lower is better)
    let err_cmp = compare_metric(
        "Error rate",
        baseline.summary.error_rate * 100.0,
        current.summary.error_rate * 100.0,
        "%",
        false, // lower is better
    );
    if err_cmp.regressed && current.summary.error_rate > 0.0 {
        let relative_change = if baseline.summary.error_rate > 0.0 {
            ((current.summary.error_rate - baseline.summary.error_rate)
                / baseline.summary.error_rate)
                * 100.0
        } else {
            100.0 // Any errors when baseline had none is bad
        };
        if relative_change > args.threshold_error_rate {
            regressions.push(Regression {
                metric: "Error rate".to_string(),
                baseline: err_cmp.baseline,
                current: err_cmp.current,
                delta_pct: relative_change,
                threshold_pct: args.threshold_error_rate,
            });
        }
    }
    metrics.push(err_cmp);

    // Latency percentiles (lower is better)
    let latency_metrics = [
        (
            "p50 latency",
            baseline.latency_us.p50,
            current.latency_us.p50,
            args.threshold_p99,
        ),
        (
            "p90 latency",
            baseline.latency_us.p90,
            current.latency_us.p90,
            args.threshold_p99,
        ),
        (
            "p95 latency",
            baseline.latency_us.p95,
            current.latency_us.p95,
            args.threshold_p99,
        ),
        (
            "p99 latency",
            baseline.latency_us.p99,
            current.latency_us.p99,
            args.threshold_p99,
        ),
        (
            "p99.9 latency",
            baseline.latency_us.p999,
            current.latency_us.p999,
            args.threshold_p999,
        ),
    ];

    for (name, base_us, curr_us, threshold) in latency_metrics {
        let base_ms = base_us as f64 / 1000.0;
        let curr_ms = curr_us as f64 / 1000.0;
        let cmp = compare_metric(name, base_ms, curr_ms, "ms", false);

        if cmp.regressed && cmp.delta_pct > threshold {
            regressions.push(Regression {
                metric: name.to_string(),
                baseline: base_ms,
                current: curr_ms,
                delta_pct: cmp.delta_pct,
                threshold_pct: threshold,
            });
        }
        metrics.push(cmp);
    }

    // Status codes
    let mut all_codes: Vec<u16> = baseline
        .status_codes
        .keys()
        .chain(current.status_codes.keys())
        .filter_map(|s| s.parse().ok())
        .collect();
    all_codes.sort();
    all_codes.dedup();

    for code in all_codes {
        let code_str = code.to_string();
        let base_count = baseline.status_codes.get(&code_str).copied().unwrap_or(0) as f64;
        let curr_count = current.status_codes.get(&code_str).copied().unwrap_or(0) as f64;
        if base_count > 0.0 || curr_count > 0.0 {
            metrics.push(compare_metric(
                &format!("Status {}", code),
                base_count,
                curr_count,
                "",
                code < 400, // 2xx/3xx higher is better, 4xx/5xx lower is better
            ));
        }
    }

    let has_regressions = !regressions.is_empty();

    Ok(CompareResult {
        baseline_file: args.baseline.display().to_string(),
        current_file: args.current.display().to_string(),
        metrics,
        regressions,
        warnings,
        has_regressions,
    })
}

fn compare_metric(
    name: &str,
    baseline: f64,
    current: f64,
    unit: &str,
    higher_is_better: bool,
) -> MetricComparison {
    let delta = current - baseline;
    let delta_pct = if baseline != 0.0 {
        (delta / baseline) * 100.0
    } else if current != 0.0 {
        100.0
    } else {
        0.0
    };

    let improved = if higher_is_better {
        delta > 0.0
    } else {
        delta < 0.0
    };

    let regressed = if higher_is_better {
        delta < 0.0
    } else {
        delta > 0.0
    };

    MetricComparison {
        name: name.to_string(),
        baseline,
        current,
        delta,
        delta_pct,
        unit: unit.to_string(),
        improved,
        regressed,
    }
}

fn load_json(path: &Path) -> Result<JsonOutput, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))
}
