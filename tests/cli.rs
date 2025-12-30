//! CLI integration tests for kaioken
//!
//! These tests verify the CLI interface works correctly.

use assert_cmd::Command;
use predicates::prelude::*;

fn kaioken() -> Command {
    Command::cargo_bin("kaioken").unwrap()
}

mod help_and_version {
    use super::*;

    #[test]
    fn help_displays_usage() {
        kaioken()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("load test"))
            .stdout(predicate::str::contains("run"))
            .stdout(predicate::str::contains("compare"));
    }

    #[test]
    fn version_displays_version() {
        kaioken()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn run_help_shows_options() {
        kaioken()
            .args(["run", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--concurrency"))
            .stdout(predicate::str::contains("--duration"))
            .stdout(predicate::str::contains("--arrival-rate"))
            .stdout(predicate::str::contains("--max-vus"));
    }

    #[test]
    fn compare_help_shows_options() {
        kaioken()
            .args(["compare", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--threshold-p99"))
            .stdout(predicate::str::contains("--force"));
    }

    #[test]
    fn init_help_shows_options() {
        kaioken()
            .args(["init", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--output"))
            .stdout(predicate::str::contains("--url"));
    }
}

mod run_validation {
    use super::*;

    #[test]
    fn run_without_url_fails() {
        kaioken()
            .arg("run")
            .assert()
            .failure()
            .stderr(predicate::str::contains("required"));
    }

    #[test]
    fn run_with_invalid_duration_fails() {
        kaioken()
            .args(["run", "https://example.com", "-d", "invalid"])
            .assert()
            .failure();
    }

    #[test]
    fn run_dry_run_validates_config() {
        kaioken()
            .args(["run", "https://example.com", "--dry-run", "-y"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn run_dry_run_shows_arrival_rate() {
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--dry-run",
                "-y",
                "--arrival-rate",
                "100",
                "--max-vus",
                "50",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Load Model:  Open (arrival rate)"))
            .stderr(predicate::str::contains("Target RPS:  100"))
            .stderr(predicate::str::contains("Max VUs:     50"));
    }

    #[test]
    fn run_dry_run_shows_closed_model() {
        kaioken()
            .args(["run", "https://example.com", "--dry-run", "-y", "-c", "25"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Load Model:  Closed (VU-driven)"))
            .stderr(predicate::str::contains("Concurrency: 25"));
    }
}

mod compare_validation {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn compare_missing_files_fails() {
        kaioken()
            .args(["compare", "nonexistent1.json", "nonexistent2.json"])
            .assert()
            .failure();
    }

    #[test]
    fn compare_invalid_json_fails() {
        let dir = tempdir().unwrap();
        let baseline = dir.path().join("baseline.json");
        let current = dir.path().join("current.json");

        fs::write(&baseline, "not valid json").unwrap();
        fs::write(&current, "also not valid").unwrap();

        kaioken()
            .args([
                "compare",
                baseline.to_str().unwrap(),
                current.to_str().unwrap(),
            ])
            .assert()
            .failure();
    }
}

mod init_command {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn init_creates_config_file() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.toml");

        kaioken()
            .args(["init", "-o", output.to_str().unwrap()])
            .assert()
            .success();

        assert!(output.exists());
        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("[target]"));
        assert!(content.contains("[load]"));
    }

    #[test]
    fn init_with_url_includes_url() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.toml");

        kaioken()
            .args([
                "init",
                "-o",
                output.to_str().unwrap(),
                "-u",
                "https://api.test.com/health",
            ])
            .assert()
            .success();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("https://api.test.com/health"));
    }

    #[test]
    fn init_refuses_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.toml");

        fs::write(&output, "existing content").unwrap();

        kaioken()
            .args(["init", "-o", output.to_str().unwrap()])
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
    }

    #[test]
    fn init_overwrites_with_force() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.toml");

        fs::write(&output, "existing content").unwrap();

        kaioken()
            .args(["init", "-o", output.to_str().unwrap(), "--force"])
            .assert()
            .success();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("[target]"));
    }
}

mod completions {
    use super::*;

    #[test]
    fn generates_bash_completions() {
        kaioken()
            .args(["completions", "bash"])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete"));
    }

    #[test]
    fn generates_zsh_completions() {
        kaioken()
            .args(["completions", "zsh"])
            .assert()
            .success()
            .stdout(predicate::str::contains("compdef"));
    }

    #[test]
    fn generates_fish_completions() {
        kaioken()
            .args(["completions", "fish"])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete"));
    }
}

mod man_page {
    use super::*;

    #[test]
    fn generates_man_page() {
        kaioken()
            .arg("man")
            .assert()
            .success()
            .stdout(predicate::str::contains(".TH"));
    }
}

mod websocket_cli {
    use super::*;

    #[test]
    fn ws_message_interval_flag_accepted() {
        kaioken()
            .args([
                "run",
                "ws://localhost:8080/ws",
                "--ws-message-interval",
                "50ms",
                "--dry-run",
                "-y",
            ])
            .assert()
            .success();
    }

    #[test]
    fn ws_fire_and_forget_flag_accepted() {
        kaioken()
            .args([
                "run",
                "ws://localhost:8080/ws",
                "--ws-fire-and-forget",
                "--dry-run",
                "-y",
            ])
            .assert()
            .success();
    }

    #[test]
    fn ws_combined_flags_accepted() {
        kaioken()
            .args([
                "run",
                "ws://localhost:8080/events",
                "--ws-message-interval",
                "10ms",
                "--ws-fire-and-forget",
                "-c",
                "20",
                "-d",
                "30s",
                "--dry-run",
                "-y",
            ])
            .assert()
            .success();
    }

    #[test]
    fn help_shows_websocket_options() {
        kaioken()
            .args(["run", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--ws-message-interval"))
            .stdout(predicate::str::contains("--ws-fire-and-forget"));
    }
}

mod import_command {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn import_help_shows_options() {
        kaioken()
            .args(["import", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("HAR"))
            .stdout(predicate::str::contains("--filter"));
    }

    #[test]
    fn import_har_creates_config() {
        let dir = tempdir().unwrap();
        let har_file = dir.path().join("test.har");
        let output_file = dir.path().join("output.toml");

        fs::write(
            &har_file,
            r#"{
                "log": {
                    "entries": [{
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/health",
                            "headers": []
                        }
                    }]
                }
            }"#,
        )
        .unwrap();

        kaioken()
            .args([
                "import",
                har_file.to_str().unwrap(),
                "-o",
                output_file.to_str().unwrap(),
            ])
            .assert()
            .success();

        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("[target]"));
        assert!(content.contains("https://api.example.com/health"));
    }

    #[test]
    fn import_missing_file_fails() {
        kaioken()
            .args(["import", "/nonexistent/file.har"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Failed to read"));
    }

    #[test]
    fn import_invalid_json_fails() {
        let dir = tempdir().unwrap();
        let har_file = dir.path().join("invalid.har");

        fs::write(&har_file, "{ invalid json }").unwrap();

        kaioken()
            .args(["import", har_file.to_str().unwrap()])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Failed to parse"));
    }

    #[test]
    fn import_with_filter() {
        let dir = tempdir().unwrap();
        let har_file = dir.path().join("multi.har");
        let output_file = dir.path().join("filtered.toml");

        fs::write(
            &har_file,
            r#"{
                "log": {
                    "entries": [
                        {"request": {"method": "GET", "url": "https://api.example.com/v1/users", "headers": []}},
                        {"request": {"method": "GET", "url": "https://api.example.com/v2/users", "headers": []}},
                        {"request": {"method": "GET", "url": "https://cdn.example.com/image.png", "headers": []}}
                    ]
                }
            }"#,
        )
        .unwrap();

        kaioken()
            .args([
                "import",
                har_file.to_str().unwrap(),
                "--filter",
                "v2",
                "-o",
                output_file.to_str().unwrap(),
            ])
            .assert()
            .success();

        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("/v2/users"));
        assert!(!content.contains("/v1/users"));
        assert!(!content.contains("cdn.example.com"));
    }
}

mod v1_3_features {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn help_shows_v1_3_flags() {
        kaioken()
            .args(["run", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--rand-regex-url"))
            .stdout(predicate::str::contains("--urls-from-file"))
            .stdout(predicate::str::contains("--body-lines"))
            .stdout(predicate::str::contains("--connect-to"))
            .stdout(predicate::str::contains("--db-url"))
            .stdout(predicate::str::contains("--burst-rate"))
            .stdout(predicate::str::contains("--burst-delay"));
    }

    #[test]
    fn rand_regex_url_provides_url() {
        // --rand-regex-url should satisfy the URL requirement
        kaioken()
            .args([
                "run",
                "--rand-regex-url",
                "https://example\\.com/users/[0-9]+",
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn urls_from_file_provides_url() {
        let dir = tempdir().unwrap();
        let urls_file = dir.path().join("urls.txt");
        fs::write(&urls_file, "https://example.com/1\nhttps://example.com/2\n").unwrap();

        kaioken()
            .args([
                "run",
                "--urls-from-file",
                urls_file.to_str().unwrap(),
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn urls_from_file_empty_fails() {
        let dir = tempdir().unwrap();
        let urls_file = dir.path().join("empty.txt");
        fs::write(&urls_file, "").unwrap();

        kaioken()
            .args([
                "run",
                "--urls-from-file",
                urls_file.to_str().unwrap(),
                "--dry-run",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("URL is required"));
    }

    #[test]
    fn body_lines_file_validates() {
        let dir = tempdir().unwrap();
        let body_file = dir.path().join("bodies.jsonl");
        fs::write(&body_file, "{\"id\":1}\n{\"id\":2}\n").unwrap();

        kaioken()
            .args([
                "run",
                "https://example.com",
                "-Z",
                body_file.to_str().unwrap(),
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn connect_to_flag_validates() {
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--connect-to",
                "example.com:127.0.0.1:8080",
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn connect_to_invalid_format_fails() {
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--connect-to",
                "invalid",
                "--dry-run",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid connect-to format"));
    }

    #[test]
    fn db_url_flag_accepted() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        kaioken()
            .args([
                "run",
                "https://example.com",
                "--db-url",
                db_path.to_str().unwrap(),
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn burst_mode_requires_both_flags() {
        // --burst-rate without --burst-delay should fail
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--burst-rate",
                "100",
                "--dry-run",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("burst-delay"));
    }

    #[test]
    fn burst_mode_validates() {
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--burst-rate",
                "100",
                "--burst-delay",
                "1s",
                "--dry-run",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains("Configuration validated"));
    }

    #[test]
    fn burst_mode_conflicts_with_arrival_rate() {
        kaioken()
            .args([
                "run",
                "https://example.com",
                "--burst-rate",
                "100",
                "--burst-delay",
                "1s",
                "--arrival-rate",
                "50",
                "--dry-run",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("cannot be used with"));
    }

    #[test]
    fn rand_regex_conflicts_with_urls_from_file() {
        let dir = tempdir().unwrap();
        let urls_file = dir.path().join("urls.txt");
        fs::write(&urls_file, "https://example.com/1\n").unwrap();

        kaioken()
            .args([
                "run",
                "--rand-regex-url",
                "https://example\\.com/[0-9]+",
                "--urls-from-file",
                urls_file.to_str().unwrap(),
                "--dry-run",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("cannot be used with"));
    }
}
