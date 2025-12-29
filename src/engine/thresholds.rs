use crate::types::{StatsSnapshot, Threshold, ThresholdMetric, ThresholdResult};

pub fn evaluate_thresholds(
    thresholds: &[Threshold],
    snapshot: &StatsSnapshot,
) -> Vec<ThresholdResult> {
    thresholds
        .iter()
        .map(|t| evaluate_threshold(t, snapshot))
        .collect()
}

fn evaluate_threshold(threshold: &Threshold, snapshot: &StatsSnapshot) -> ThresholdResult {
    let actual = get_metric_value(&threshold.metric, snapshot);
    let passed = threshold.operator.evaluate(actual, threshold.value);

    ThresholdResult {
        metric: threshold.metric.as_str().to_string(),
        condition: format!(
            "{} {} {}",
            threshold.metric.as_str(),
            threshold.operator.as_str(),
            threshold.value
        ),
        actual,
        passed,
    }
}

fn get_metric_value(metric: &ThresholdMetric, snapshot: &StatsSnapshot) -> f64 {
    match metric {
        ThresholdMetric::P50LatencyMs => snapshot.latency_p50_us as f64 / 1000.0,
        ThresholdMetric::P75LatencyMs => snapshot.latency_p75_us as f64 / 1000.0,
        ThresholdMetric::P90LatencyMs => snapshot.latency_p90_us as f64 / 1000.0,
        ThresholdMetric::P95LatencyMs => snapshot.latency_p95_us as f64 / 1000.0,
        ThresholdMetric::P99LatencyMs => snapshot.latency_p99_us as f64 / 1000.0,
        ThresholdMetric::P999LatencyMs => snapshot.latency_p999_us as f64 / 1000.0,
        ThresholdMetric::MeanLatencyMs => snapshot.latency_mean_us / 1000.0,
        ThresholdMetric::MaxLatencyMs => snapshot.latency_max_us as f64 / 1000.0,
        ThresholdMetric::ErrorRate => snapshot.error_rate,
        ThresholdMetric::Rps => snapshot.requests_per_sec,
        ThresholdMetric::CheckPassRate => snapshot.overall_check_pass_rate.unwrap_or(1.0),
    }
}

pub fn print_threshold_results(results: &[ThresholdResult]) {
    if results.is_empty() {
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("THRESHOLDS");
    println!("{}", "=".repeat(60));

    let any_failed = results.iter().any(|r| !r.passed);

    for result in results {
        let status = if result.passed {
            "\x1b[32m✓ PASS\x1b[0m"
        } else {
            "\x1b[31m✗ FAIL\x1b[0m"
        };

        let actual_str = format_metric_value(&result.metric, result.actual);
        println!("  {} {} (actual: {})", status, result.condition, actual_str);
    }

    println!();
    if any_failed {
        println!("\x1b[31mThresholds failed! Exiting with code 4.\x1b[0m");
    } else {
        println!("\x1b[32mAll thresholds passed.\x1b[0m");
    }
}

fn format_metric_value(metric: &str, value: f64) -> String {
    if metric.contains("latency") {
        format!("{:.2}ms", value)
    } else if metric == "error_rate" || metric == "check_pass_rate" {
        format!("{:.4}", value)
    } else {
        format!("{:.2}", value)
    }
}
