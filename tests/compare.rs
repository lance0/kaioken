//! Compare mode integration tests
//!
//! These tests verify the regression comparison functionality.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn kaioken() -> Command {
    Command::cargo_bin("kaioken").unwrap()
}

fn create_test_results(
    total_requests: u64,
    rps: f64,
    error_rate: f64,
    p99_us: u64,
    load_model: Option<&str>,
    arrival_rate: Option<u32>,
) -> String {
    let load_model_str = load_model.unwrap_or("closed");
    let arrival_rate_field = arrival_rate
        .map(|r| format!(r#""arrival_rate": {}, "#, r))
        .unwrap_or_default();
    let max_vus_field = arrival_rate
        .map(|_| r#""max_vus": 50, "#)
        .unwrap_or_default();

    format!(
        r#"{{
    "metadata": {{
        "tool": "kaioken",
        "version": "1.0.0",
        "started_at": "2025-01-01T00:00:00Z",
        "ended_at": "2025-01-01T00:00:30Z",
        "duration_secs": 30,
        "target": {{
            "url": "https://example.com/api",
            "method": "GET",
            "headers": []
        }},
        "load": {{
            "concurrency": 50,
            "rate": 0,
            "ramp_up_secs": 0,
            "warmup_secs": 0,
            "timeout_ms": 5000,
            "load_model": "{}",
            {}{}
            "dummy": 0
        }},
        "env": {{
            "hostname": "test",
            "os": "linux",
            "cpus": 4
        }}
    }},
    "summary": {{
        "total_requests": {},
        "successful": {},
        "failed": {},
        "error_rate": {},
        "requests_per_sec": {},
        "bytes_received": 1000000
    }},
    "latency_us": {{
        "min": 1000,
        "max": 100000,
        "mean": 5000.0,
        "stddev": 2000.0,
        "p50": 4000,
        "p75": 6000,
        "p90": 8000,
        "p95": 10000,
        "p99": {},
        "p999": {}
    }},
    "status_codes": {{
        "200": {}
    }},
    "errors": {{}},
    "timeline": []
}}"#,
        load_model_str,
        arrival_rate_field,
        max_vus_field,
        total_requests,
        ((1.0 - error_rate) * total_requests as f64) as u64,
        (error_rate * total_requests as f64) as u64,
        error_rate,
        rps,
        p99_us,
        p99_us + 10000,
        ((1.0 - error_rate) * total_requests as f64) as u64
    )
}

mod basic_comparison {
    use super::*;

    #[test]
    fn compare_identical_results_no_regression() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        let results = create_test_results(1000, 100.0, 0.01, 10000, None, None);
        fs::write(&baseline, &results).unwrap();
        fs::write(&current, &results).unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("stable"));
    }

    #[test]
    fn compare_improved_results() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 15000, None, None),
        )
        .unwrap();
        fs::write(
            &current,
            create_test_results(1000, 120.0, 0.005, 10000, None, None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .success();
    }

    #[test]
    fn compare_regression_exits_with_code_3() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, None, None),
        )
        .unwrap();
        // 50% worse p99 latency
        fs::write(
            &current,
            create_test_results(1000, 100.0, 0.01, 15000, None, None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
                "--threshold-p99",
                "10",
            ])
            .assert()
            .code(3)
            .stdout(predicate::str::contains("REGRESSION"));
    }

    #[test]
    fn compare_json_output() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        let results = create_test_results(1000, 100.0, 0.01, 10000, None, None);
        fs::write(&baseline, &results).unwrap();
        fs::write(&current, &results).unwrap();

        let output = kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert!(json["metrics"].as_array().is_some());
        assert_eq!(json["has_regressions"].as_bool().unwrap(), false);
    }
}

mod load_model_validation {
    use super::*;

    #[test]
    fn compare_open_vs_closed_fails_without_force() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, Some("open"), Some(100)),
        )
        .unwrap();
        fs::write(
            &current,
            create_test_results(1000, 100.0, 0.01, 10000, Some("closed"), None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .code(5)
            .stderr(predicate::str::contains("Cannot compare"))
            .stderr(predicate::str::contains("--force"));
    }

    #[test]
    fn compare_open_vs_closed_succeeds_with_force() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, Some("open"), Some(100)),
        )
        .unwrap();
        fs::write(
            &current,
            create_test_results(1000, 100.0, 0.01, 10000, Some("closed"), None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
                "--force",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("forced comparison"));
    }

    #[test]
    fn compare_prints_load_model_metadata() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, Some("open"), Some(100)),
        )
        .unwrap();
        fs::write(
            &current,
            create_test_results(1000, 100.0, 0.01, 10000, Some("open"), Some(100)),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Baseline:"))
            .stderr(predicate::str::contains("Open"))
            .stderr(predicate::str::contains("Candidate:"));
    }

    #[test]
    fn compare_closed_models_shows_vus() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        let results = create_test_results(1000, 100.0, 0.01, 10000, Some("closed"), None);
        fs::write(&baseline, &results).unwrap();
        fs::write(&current, &results).unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Closed"))
            .stderr(predicate::str::contains("vus="));
    }
}

mod threshold_options {
    use super::*;

    #[test]
    fn custom_p99_threshold() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, None, None),
        )
        .unwrap();
        // 20% worse p99, but threshold is 25%
        fs::write(
            &current,
            create_test_results(1000, 100.0, 0.01, 12000, None, None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
                "--threshold-p99",
                "25",
            ])
            .assert()
            .success();
    }

    #[test]
    fn custom_rps_threshold() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(
            &baseline,
            create_test_results(1000, 100.0, 0.01, 10000, None, None),
        )
        .unwrap();
        // 15% worse RPS, but threshold is 20%
        fs::write(
            &current,
            create_test_results(1000, 85.0, 0.01, 10000, None, None),
        )
        .unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
                "--threshold-rps",
                "20",
            ])
            .assert()
            .success();
    }
}

mod warnings {
    use super::*;

    #[test]
    fn warns_on_concurrency_difference() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        let mut baseline_json = create_test_results(1000, 100.0, 0.01, 10000, None, None);
        baseline_json = baseline_json.replace(r#""concurrency": 50"#, r#""concurrency": 50"#);

        let mut current_json = create_test_results(1000, 100.0, 0.01, 10000, None, None);
        current_json = current_json.replace(r#""concurrency": 50"#, r#""concurrency": 100"#);

        fs::write(&baseline, baseline_json).unwrap();
        fs::write(&current, current_json).unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("Concurrency differs"));
    }
}
