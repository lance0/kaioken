# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2024-12-23

### Changed

- Updated to Rust 2024 edition

## [0.1.0] - 2024-12-23

### Added

- CLI with clap: URL, concurrency, duration, timeout, headers, body, output options
- HTTP load testing engine with concurrent workers
- Real-time stats aggregation with HDR histogram
- Terminal UI (TUI) with ratatui featuring:
  - Power level panel with rolling RPS and sparkline
  - Latency percentiles (p50/p90/p95/p99/p999) with bar charts
  - Status codes distribution
  - Error breakdown by type
- JSON output export with full metrics
- DBZ flavor with power ranks (Farmer -> OVER 9000) and themed messages
- `--serious` flag to disable DBZ theming
- `--no-tui` and `--json` flags for headless/CI usage
- Safety warning for non-localhost targets
- Graceful shutdown with Ctrl+C
