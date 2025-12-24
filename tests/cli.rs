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
            .args([
                "run",
                "https://example.com",
                "--dry-run",
                "-y",
                "-c",
                "25",
            ])
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
            .args(["compare", baseline.to_str().unwrap(), current.to_str().unwrap()])
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
