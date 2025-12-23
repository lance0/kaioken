# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2025-12-23

### Added

- `kaioken init` subcommand to generate starter config file
- `kaioken completions <shell>` for bash/zsh/fish shell completions
- `--dry-run` flag to validate config without running
- Weighted scenarios support via `[[scenarios]]` in TOML config:
  - Define multiple endpoints with different methods, headers, body
  - Set weight for traffic distribution (e.g., 70% reads, 30% writes)
  - Variable interpolation works in all scenario fields

### Changed

- Init command generates config with scenario examples

## [0.4.0] - 2025-12-23

### Added

- `--max-requests` / `-n` flag to stop after N requests (useful for fixed workloads)
- `--body-file` flag to load request body from file
- `--http2` flag for HTTP/2 prior knowledge (h2c)
- Variable interpolation in URL, headers, and body:
  - `${REQUEST_ID}` - unique ID per request (worker_id * 1B + counter)
  - `${TIMESTAMP_MS}` - current epoch time in milliseconds
- DBZ-themed color schemes in TUI (press `t` to cycle):
  - Earth (default cyan/yellow)
  - Namek (green/turquoise)
  - Planet Vegeta (red/orange)
  - Time Chamber (steel blue/minimal)
  - Tournament (gold/purple)
  - Frieza Force (purple/pink)

### Changed

- Footer now shows current theme name and `[t]heme` hint

## [0.3.0] - 2025-12-23

### Added

- `kaioken compare <baseline.json> <current.json>` subcommand for regression detection
- Side-by-side comparison table with delta percentages
- Configurable regression thresholds (`--threshold-p99`, `--threshold-rps`, etc.)
- Exit code 3 when regressions detected (for CI gating)
- Config compatibility warnings (URL, method, concurrency differences)
- JSON output for compare results (`--json`)
- DBZ-flavored comparison output ("FUSION", "POWER", "DRAIN")

### Changed

- CLI restructured to use subcommands: `kaioken run <URL>` and `kaioken compare`
- **Breaking**: URL is now passed to `run` subcommand (e.g., `kaioken run https://...`)

## [0.2.1] - 2025-12-23

### Added

- README.md with full documentation
- Dual MIT/Apache-2.0 license

## [0.2.0] - 2025-12-23

### Added

- Rate limiting (`-r, --rate`) with token bucket algorithm for controlled RPS
- Ramp-up (`--ramp-up`) to gradually activate workers over time
- Warmup period (`--warmup`) to prime connections before measuring
- TOML config file support (`-f, --config`) with full feature parity
- Environment variable interpolation in config files (`${VAR}`, `${VAR:-default}`)
- CSV output format (`--format csv`)
- Markdown output format (`--format md`)
- TUI shows warmup phase indicator ("Charging..." / "Warmup")

### Changed

- URL argument now optional when using config file
- JSON output includes rate, ramp_up_secs, warmup_secs fields

## [0.1.1] - 2025-12-23

### Changed

- Updated to Rust 2024 edition

## [0.1.0] - 2025-12-23

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
