# kaioken

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

[![Crates.io](https://img.shields.io/crates/v/kaioken.svg)](https://crates.io/crates/kaioken)
[![License](https://img.shields.io/crates/l/kaioken.svg)](https://github.com/lance0/kaioken#license)

## Features

- **Real-time TUI** - Live metrics with latency percentiles, RPS, status codes
- **Weighted scenarios** - Multi-endpoint testing with traffic distribution
- **Rate limiting** - Token bucket algorithm for controlled load
- **Ramp-up & warmup** - Gradual worker activation and connection priming
- **Compare mode** - Regression detection with CI-friendly exit codes
- **Multiple outputs** - JSON, CSV, and Markdown formats
- **Variable interpolation** - Dynamic `${REQUEST_ID}` and `${TIMESTAMP_MS}`
- **HTTP/2 support** - Optional h2 prior knowledge mode
- **DBZ themes** - 6 color schemes (press `t` to cycle)

## Installation

```bash
cargo install kaioken
```

## Quick Start

```bash
# Basic test
kaioken run https://api.example.com/health

# With options
kaioken run https://api.example.com/users \
  -c 100 -d 30s -r 500 --warmup 5s

# Fixed number of requests
kaioken run https://api.example.com -n 10000

# Generate starter config
kaioken init --url https://api.example.com

# Validate config without running
kaioken run -f config.toml --dry-run

# Compare two runs for regressions
kaioken compare baseline.json current.json

# Shell completions
kaioken completions bash >> ~/.bashrc
```

## TUI Preview

```
┌───────────────────────────────────────────────────────────────┐
│ KAIOKEN x100   https://api.example.com/users    [00:15/00:30] │
├───────────────────────────────┬───────────────────────────────┤
│ POWER LEVEL                   │ LATENCY (ms)                  │
│                               │                               │
│ Rolling RPS: 8,432  [VEGETA]  │ p50:   12ms  ████             │
│ Total:       126,480          │ p90:   45ms  ████████         │
│ Errors:      23 (0.02%)       │ p95:   89ms  ██████████       │
│                               │ p99:  230ms  █████████████    │
│ ▁▂▃▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇          │ p999: 567ms  ██████████████   │
├───────────────────────────────┼───────────────────────────────┤
│ STATUS CODES                  │ ERRORS                        │
│                               │                               │
│ 200  ████████████████  84012  │ timeout      15               │
│ 201  ██                  312  │ connect       5               │
│ 500  ▏                    23  │ reset         3               │
└───────────────────────────────┴───────────────────────────────┘
[Earth]  [q]uit  [s]ave  [t]heme                      Running...
```

Press `t` to cycle themes: Earth → Namek → Planet Vegeta → Time Chamber → Tournament → Frieza Force

## CLI Reference

### `kaioken run`

```
kaioken run [OPTIONS] [URL]
```

| Flag | Default | Description |
|------|---------|-------------|
| `[URL]` | — | Target URL (required unless using `-f`) |
| `-c, --concurrency` | 50 | Concurrent workers |
| `-d, --duration` | 10s | Test duration |
| `-n, --max-requests` | 0 | Stop after N requests (0 = unlimited) |
| `-r, --rate` | 0 | Max RPS (0 = unlimited) |
| `--ramp-up` | 0s | Time to reach full concurrency |
| `--warmup` | 0s | Warmup period (not measured) |
| `-m, --method` | GET | HTTP method |
| `-H, --header` | — | Header (repeatable) |
| `-b, --body` | — | Request body |
| `--body-file` | — | Load body from file |
| `--http2` | false | Use HTTP/2 prior knowledge |
| `-f, --config` | — | TOML config file |
| `-o, --output` | — | Output file path |
| `--format` | json | Output format: json, csv, md |
| `--no-tui` | false | Headless mode |
| `--json` | false | Shorthand for `--no-tui --format json` |
| `--dry-run` | false | Validate config and exit |
| `--serious` | false | Disable DBZ flavor |
| `--insecure` | false | Skip TLS verification |
| `-y, --yes` | false | Skip remote target confirmation |

### `kaioken compare`

```
kaioken compare <BASELINE> <CURRENT> [OPTIONS]
```

Compare two JSON result files for regressions. Exits with code 3 if regressions detected.

| Flag | Default | Description |
|------|---------|-------------|
| `--threshold-p99` | 10.0 | p99 latency regression threshold (%) |
| `--threshold-p999` | 15.0 | p999 latency regression threshold (%) |
| `--threshold-error-rate` | 50.0 | Error rate regression threshold (%) |
| `--threshold-rps` | 10.0 | RPS regression threshold (%) |
| `--json` | false | Output as JSON |

### `kaioken init`

```
kaioken init [OPTIONS]
```

Generate a starter config file with documented options.

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output` | kaioken.toml | Output file path |
| `-u, --url` | — | Target URL to include |
| `--force` | false | Overwrite existing file |

### `kaioken completions`

```
kaioken completions <SHELL>
```

Generate shell completions. Supported: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

## Config File

```toml
[target]
url = "https://api.example.com/users"
method = "POST"
timeout = "5s"
connect_timeout = "2s"
# http2 = false
# insecure = false

[target.headers]
Authorization = "Bearer ${API_TOKEN}"
Content-Type = "application/json"

# body = '{"key": "value"}'
# body_file = "payload.json"

[load]
concurrency = 100
duration = "30s"
# max_requests = 0
# rate = 500
# ramp_up = "5s"
# warmup = "3s"
```

Environment variables: `${VAR}` or `${VAR:-default}`

## Weighted Scenarios

Test multiple endpoints with different traffic ratios:

```toml
[load]
concurrency = 100
duration = "60s"

[[scenarios]]
name = "list_users"
url = "https://api.example.com/users"
method = "GET"
weight = 7  # 70% of traffic

[[scenarios]]
name = "create_user"
url = "https://api.example.com/users"
method = "POST"
body = '{"name": "test-${REQUEST_ID}"}'
weight = 2  # 20% of traffic

[[scenarios]]
name = "health_check"
url = "https://api.example.com/health"
method = "GET"
weight = 1  # 10% of traffic
```

Validate with `--dry-run`:
```
$ kaioken run -f config.toml --dry-run
Configuration validated successfully!

Scenarios:   3 defined
  - list_users (GET .../users) weight=7 (70%)
  - create_user (POST .../users) weight=2 (20%)
  - health_check (GET .../health) weight=1 (10%)
Concurrency: 100
Duration:    60s
```

## Variable Interpolation

Available in URL, headers, and body:

| Variable | Description |
|----------|-------------|
| `${REQUEST_ID}` | Unique ID per request (worker_id * 1B + counter) |
| `${TIMESTAMP_MS}` | Current epoch time in milliseconds |

Example:
```bash
kaioken run 'https://api.example.com/items/${REQUEST_ID}' \
  -H 'X-Request-ID: ${REQUEST_ID}' \
  -b '{"ts": ${TIMESTAMP_MS}}'
```

## CI Integration

```yaml
# GitHub Actions example
- name: Load test
  run: |
    kaioken run https://api.example.com \
      -c 50 -d 30s --no-tui --json -o results.json

- name: Check for regressions
  run: |
    kaioken compare baseline.json results.json \
      --threshold-p99 15 --threshold-rps 10
```

Exit codes:
- `0` - Success
- `1` - Error (high error rate)
- `3` - Regressions detected (compare mode)

## Power Levels

| RPS | Rank |
|-----|------|
| 0-100 | Farmer |
| 101-500 | Krillin |
| 501-1,000 | Piccolo |
| 1,001-5,000 | Vegeta |
| 5,001-9,000 | Goku |
| 9,001+ | OVER 9000 |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
