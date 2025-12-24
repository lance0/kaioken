//! Load test integration tests using wiremock
//!
//! These tests verify the load testing functionality works correctly
//! against a mock HTTP server.

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn kaioken() -> Command {
    Command::cargo_bin("kaioken").unwrap()
}

async fn setup_mock_server() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":"ok"}"#))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"status":"ok"}"#)
                .set_delay(std::time::Duration::from_millis(100)),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(500).set_body_string(r#"{"error":"internal"}"#))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/users"))
        .respond_with(
            ResponseTemplate::new(201).set_body_string(r#"{"id":1,"name":"test"}"#),
        )
        .mount(&server)
        .await;

    server
}

#[tokio::test]
async fn basic_load_test_succeeds() {
    let server = setup_mock_server().await;
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "2",
            "-d",
            "1s",
            "--no-tui",
            "-y",
        ])
        .assert()
        .success();
}

#[tokio::test]
async fn load_test_json_output() {
    let server = setup_mock_server().await;
    let dir = tempdir().unwrap();
    let output = dir.path().join("results.json");
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "2",
            "-d",
            "1s",
            "--no-tui",
            "-y",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output.exists());
    let content = fs::read_to_string(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(json["metadata"]["target"]["url"].as_str().is_some());
    assert!(json["summary"]["total_requests"].as_u64().unwrap() > 0);
    assert!(json["latency_us"]["p50"].as_u64().is_some());
}

#[tokio::test]
async fn load_test_csv_output() {
    let server = setup_mock_server().await;
    let dir = tempdir().unwrap();
    let output = dir.path().join("results.csv");
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "2",
            "-d",
            "1s",
            "--no-tui",
            "-y",
            "-o",
            output.to_str().unwrap(),
            "--format",
            "csv",
        ])
        .assert()
        .success();

    assert!(output.exists());
    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("metric,value"));
    assert!(content.contains("total_requests"));
}

#[tokio::test]
async fn load_test_with_headers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/auth"))
        .and(wiremock::matchers::header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let url = format!("{}/auth", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "1",
            "-d",
            "1s",
            "--no-tui",
            "-y",
            "-H",
            "Authorization: Bearer test-token",
        ])
        .assert()
        .success();
}

#[tokio::test]
async fn load_test_post_with_body() {
    let server = setup_mock_server().await;
    let url = format!("{}/users", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "1",
            "-d",
            "1s",
            "--no-tui",
            "-y",
            "-m",
            "POST",
            "-b",
            r#"{"name":"test"}"#,
            "-H",
            "Content-Type: application/json",
        ])
        .assert()
        .success();
}

#[tokio::test]
async fn load_test_max_requests() {
    let server = setup_mock_server().await;
    let dir = tempdir().unwrap();
    let output = dir.path().join("results.json");
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "2",
            "-n",
            "10",
            "--no-tui",
            "-y",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let total = json["summary"]["total_requests"].as_u64().unwrap();

    assert!(total >= 10, "Expected at least 10 requests, got {}", total);
}

#[tokio::test]
async fn load_test_rate_limiting() {
    let server = setup_mock_server().await;
    let dir = tempdir().unwrap();
    let output = dir.path().join("results.json");
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "-c",
            "10",
            "-d",
            "2s",
            "-r",
            "5",
            "--no-tui",
            "-y",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let rps = json["summary"]["requests_per_sec"].as_f64().unwrap();

    // Rate should be close to 5, with some tolerance
    assert!(rps <= 7.0, "RPS {} should be rate-limited to ~5", rps);
}

#[tokio::test]
async fn load_test_json_stdout() {
    let server = setup_mock_server().await;
    let url = format!("{}/health", server.uri());

    let output = kaioken()
        .args([
            "run",
            &url,
            "-c",
            "1",
            "-d",
            "1s",
            "--json",
            "-y",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json["summary"]["total_requests"].as_u64().is_some());
}

#[tokio::test]
async fn load_test_arrival_rate_mode() {
    let server = setup_mock_server().await;
    let dir = tempdir().unwrap();
    let output = dir.path().join("results.json");
    let url = format!("{}/health", server.uri());

    kaioken()
        .args([
            "run",
            &url,
            "--arrival-rate",
            "10",
            "--max-vus",
            "5",
            "-d",
            "2s",
            "--no-tui",
            "-y",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify arrival rate mode was used
    assert_eq!(
        json["metadata"]["load"]["load_model"].as_str().unwrap(),
        "open"
    );
    assert_eq!(
        json["metadata"]["load"]["arrival_rate"].as_u64().unwrap(),
        10
    );
    assert_eq!(
        json["metadata"]["load"]["max_vus"].as_u64().unwrap(),
        5
    );

    // Verify arrival rate summary is present
    assert!(json["summary"]["arrival_rate"]["target_rps"].as_u64().is_some());
    assert!(json["summary"]["arrival_rate"]["achieved_rps"].as_f64().is_some());
}
