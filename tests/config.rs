//! Configuration parsing tests
//!
//! These tests verify TOML config parsing works correctly.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn kaioken() -> Command {
    Command::cargo_bin("kaioken").unwrap()
}

mod basic_config {
    use super::*;

    #[test]
    fn minimal_config_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn config_with_all_options_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"
method = "POST"
timeout = "10s"
connect_timeout = "3s"
http2 = true
insecure = false
cookie_jar = true

[target.headers]
Authorization = "Bearer token"
Content-Type = "application/json"

[load]
concurrency = 100
duration = "1m"
rate = 500
ramp_up = "10s"
warmup = "5s"
think_time = "100ms"

[thresholds]
p95_latency_ms = "< 500"
error_rate = "< 0.01"

[[checks]]
name = "status_ok"
condition = "status == 200"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Thresholds"))
            .stderr(predicate::str::contains("Checks"));
    }
}

mod stages_config {
    use super::*;

    #[test]
    fn vu_based_stages_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[[stages]]
duration = "30s"
target = 50

[[stages]]
duration = "2m"
target = 50

[[stages]]
duration = "30s"
target = 0
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Stages"))
            .stderr(predicate::str::contains("3 defined"));
    }

    #[test]
    fn rate_based_stages_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
max_vus = 200

[[stages]]
duration = "1m"
target_rate = 100

[[stages]]
duration = "5m"
target_rate = 500

[[stages]]
duration = "1m"
target_rate = 0
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Stages"))
            .stderr(predicate::str::contains("RPS"));
    }

    #[test]
    fn mixed_stages_fail_validation() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[[stages]]
duration = "30s"
target = 50

[[stages]]
duration = "30s"
target_rate = 100
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Cannot mix"));
    }
}

mod scenarios_config {
    use super::*;

    #[test]
    fn weighted_scenarios_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "10s"

[[scenarios]]
name = "get_users"
url = "https://example.com/users"
method = "GET"
weight = 7

[[scenarios]]
name = "create_user"
url = "https://example.com/users"
method = "POST"
body = '{"name": "test"}'
weight = 3
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Scenarios"))
            .stderr(predicate::str::contains("get_users"));
    }

    #[test]
    fn scenarios_with_tags_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "10s"

[[scenarios]]
name = "api_v2"
url = "https://example.com/api/v2/users"
method = "GET"
weight = 1

[scenarios.tags]
version = "v2"
endpoint = "users"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success();
    }
}

mod arrival_rate_config {
    use super::*;

    #[test]
    fn arrival_rate_config_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
arrival_rate = 100
max_vus = 200
duration = "5m"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn arrival_rate_conflicts_with_concurrency() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 50
arrival_rate = 100
duration = "5m"
"#,
        )
        .unwrap();

        // Should warn or handle the conflict gracefully
        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success(); // arrival_rate takes precedence
    }
}

mod thresholds_config {
    use super::*;

    #[test]
    fn all_threshold_metrics_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"

[thresholds]
p50_latency_ms = "< 100"
p75_latency_ms = "< 200"
p90_latency_ms = "< 300"
p95_latency_ms = "< 400"
p99_latency_ms = "< 500"
p999_latency_ms = "< 1000"
mean_latency_ms = "< 200"
max_latency_ms = "< 2000"
error_rate = "< 0.01"
rps = "> 100"
check_pass_rate = "> 0.95"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("11 defined"));
    }

    #[test]
    fn invalid_threshold_metric_fails() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"

[thresholds]
invalid_metric = "< 100"
"#,
        )
        .unwrap();

        // Unknown threshold metrics must fail with helpful error listing valid ones
        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown field"))
            .stderr(predicate::str::contains("p95_latency_ms"));
    }
}

mod checks_config {
    use super::*;

    #[test]
    fn status_checks_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"

[[checks]]
name = "is_200"
condition = "status == 200"

[[checks]]
name = "is_success"
condition = "status in [200, 201, 204]"

[[checks]]
name = "not_error"
condition = "status < 400"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Checks"))
            .stderr(predicate::str::contains("3 defined"));
    }

    #[test]
    fn body_checks_validate() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"

[[checks]]
name = "has_success"
condition = 'body contains "success"'

[[checks]]
name = "valid_json"
condition = 'body matches "\{.*\}"'
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success();
    }
}

mod scenario_without_target {
    use super::*;

    #[test]
    fn scenario_only_config_fails_with_explicit_message() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[load]
concurrency = 10
duration = "10s"

[[scenarios]]
name = "get_users"
url = "https://example.com/users"
method = "GET"
weight = 1
"#,
        )
        .unwrap();

        // Should fail with explicit message about needing [target]
        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("[target]"))
            .stderr(predicate::str::contains("[[scenarios]]"));
    }
}

mod extraction_config {
    use super::*;

    #[test]
    fn json_extraction_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "https://example.com/api"

[load]
concurrency = 10
duration = "5s"

[[scenarios]]
name = "login"
url = "https://example.com/auth"
method = "POST"
body = '{"user": "test"}'
weight = 0

[scenarios.extract]
token = "json:$.access_token"

[[scenarios]]
name = "profile"
url = "https://example.com/me"
method = "GET"
weight = 1

[scenarios.headers]
Authorization = "Bearer ${token}"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success();
    }
}

mod env_var_interpolation {
    use super::*;

    #[test]
    fn env_var_with_default_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "${API_URL:-https://example.com/api}"

[load]
concurrency = 10
duration = "5s"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("https://example.com/api"));
    }
}

mod websocket_config {
    use super::*;

    #[test]
    fn websocket_url_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "ws://localhost:8080/ws"

[load]
concurrency = 10
duration = "5s"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("ws://localhost:8080/ws"));
    }

    #[test]
    fn websocket_wss_url_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "wss://example.com/socket"

[load]
concurrency = 5
duration = "10s"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("wss://example.com/socket"));
    }

    #[test]
    fn websocket_config_section_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "ws://localhost:8080/ws"

[load]
concurrency = 10
duration = "5s"

[websocket]
message_interval = "50ms"
mode = "echo"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success();
    }

    #[test]
    fn websocket_fire_and_forget_mode_validates() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.toml");

        fs::write(
            &config,
            r#"
[target]
url = "ws://localhost:8080/events"

[load]
concurrency = 20
duration = "30s"

[websocket]
message_interval = "10ms"
mode = "fire_and_forget"
"#,
        )
        .unwrap();

        kaioken()
            .args(["run", "-f", config.to_str().unwrap(), "--dry-run", "-y"])
            .assert()
            .success();
    }
}
