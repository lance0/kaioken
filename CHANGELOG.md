# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-12-24

### Added

- **Constant arrival rate** - Generate load at a fixed RPS regardless of response times:
  ```bash
  kaioken run --arrival-rate 100 --max-vus 50 -d 1m https://api.example.com
  ```
  ```toml
  [load]
  arrival_rate = 100  # 100 requests/second
  max_vus = 200       # Auto-scale VUs up to this limit
  duration = "5m"
  ```
- **Ramping arrival rate** - RPS-based stages with gradual transitions:
  ```toml
  [[stages]]
  duration = "1m"
  target_rate = 100   # Ramp to 100 RPS
  
  [[stages]]
  duration = "5m"
  target_rate = 500   # Ramp to 500 RPS
  
  [[stages]]
  duration = "1m"
  target_rate = 0     # Ramp down
  ```
- **Automatic VU scaling** - VUs spawn on-demand to maintain target rate
- **Dropped iteration tracking** - Metric for when max VUs can't sustain the rate
- **TUI arrival rate display** - Shows load model, target vs achieved RPS, VUs, dropped iterations
- New CLI flags: `--arrival-rate` and `--max-vus`
- JSON/HTML output includes `load_model`, `arrival_rate`, `max_vus`, `dropped_iterations`

### Changed

- Stage `target` field is now optional (use `target_rate` for RPS-based stages)
- Config validation prevents mixing VU-based and rate-based stages

## [0.9.0] - 2025-12-24

### Added

- **Check pass rate metric** - Track percentage of requests passing checks:
  ```toml
  [thresholds]
  check_pass_rate = "> 0.95"  # Fail if less than 95% pass
  ```
- **Checks in JSON output** - Per-check and overall pass rates included in results:
  ```json
  "checks": {
    "overall_pass_rate": 0.98,
    "results": {
      "status_ok": { "passed": 980, "total": 1000, "pass_rate": 0.98 }
    }
  }
  ```
- **Tags** - Label scenarios for filtering and organization:
  ```toml
  [[scenarios]]
  name = "get_users"
  url = "https://api.example.com/users"
  tags = { endpoint = "users", version = "v2" }
  ```
- Tags included in JSON output `scenarios` section
- **Cookie jar** - Automatic session handling with `--cookie-jar` flag or config:
  ```toml
  [target]
  cookie_jar = true
  ```
- Cookies persist across requests within each worker

## [0.8.0] - 2025-12-23

### Added

- **Response body checks** - Validate response content with body assertions:
  ```toml
  [[checks]]
  name = "has_success"
  condition = "body contains \"success\""
  
  [[checks]]
  name = "valid_json"
  condition = "body matches \"\\{.*\\}\""
  ```
- Check results displayed after test with pass/fail percentages
- **Request chaining** - Extract values from responses for subsequent requests:
  ```toml
  [[scenarios]]
  name = "login"
  url = "https://api.example.com/auth"
  method = "POST"
  body = '{"user": "test"}'
  
  [scenarios.extract]
  token = "json:$.access_token"
  
  [[scenarios]]
  name = "get_profile"
  url = "https://api.example.com/me"
  
  [scenarios.headers]
  Authorization = "Bearer ${token}"
  ```
- Extraction sources: `json:$.path`, `regex:pattern:group`, `body`
- Extracted values available as `${varname}` in URLs, headers, body
- Runtime variables (lowercase) preserved through config loading

### Changed

- Response body captured when checks or extractions are configured
- Environment variable interpolation skips lowercase variable names

## [0.7.1] - 2025-12-23

### Added

- **Think time** - Pause between requests to simulate realistic user behavior:
  - CLI: `--think-time 500ms`
  - Config: `think_time = "500ms"` in `[load]` section
- **Fail-fast mode** - Abort test immediately when any threshold breaches:
  - CLI: `--fail-fast`
  - Checks thresholds every second during the test
  - Exits with code 4 when threshold breaches detected

## [0.7.0] - 2025-12-23

### Added

- **Stages** - Multi-phase load profiles for realistic testing:
  ```toml
  [[stages]]
  duration = "30s"
  target = 50      # ramp to 50 workers
  
  [[stages]]
  duration = "2m"
  target = 50      # hold at 50
  
  [[stages]]
  duration = "30s"
  target = 0       # ramp down
  ```
- Stages automatically calculate total duration and max worker count
- `--dry-run` displays stage configuration with total duration and max workers
- StagesScheduler gradually ramps workers up/down between targets

### Changed

- Engine now supports both simple concurrency mode and stages mode
- Duration is automatically computed from stages when configured

## [0.6.0] - 2025-12-23

### Added

- **Thresholds** - CI/CD pass/fail criteria for metrics:
  ```toml
  [thresholds]
  p95_latency_ms = "< 500"
  p99_latency_ms = "< 1000"
  error_rate = "< 0.01"
  rps = "> 100"
  ```
- Exit code 4 when thresholds fail (distinct from errors/regressions)
- Threshold results in JSON output (`thresholds.passed`, `thresholds.results`)
- `--dry-run` now displays configured thresholds and checks
- **Check conditions** - validate response status (body checks coming soon):
  ```toml
  [[checks]]
  name = "status_ok"
  condition = "status == 200"
  
  [[checks]]
  name = "success_status"
  condition = "status in [200, 201, 204]"
  ```

### Changed

- Threshold evaluation happens before output generation
- JSON output includes threshold results when thresholds configured

## [0.5.1] - 2025-12-23

### Added

- `kaioken man` subcommand to generate man page (pipe to file or `man -l`)
- HTML report export with `--format html` - standalone shareable reports with:
  - Dark theme with gradient styling
  - Throughput metrics with RPS visualization
  - Latency percentile bars (p50-p999)
  - Status codes and errors breakdown
  - Timeline chart showing requests over time
  - Configuration summary
  - Mobile-responsive design

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
