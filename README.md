# kaioken

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

[![Crates.io](https://img.shields.io/crates/v/kaioken.svg)](https://crates.io/crates/kaioken)
[![License](https://img.shields.io/crates/l/kaioken.svg)](https://github.com/lance0/kaioken#license)

## Features

- **Real-time TUI** - Live metrics with latency percentiles, RPS, status codes, and sparkline
- **Rate limiting** - Token bucket algorithm for controlled load (`-r 1000`)
- **Ramp-up** - Gradually activate workers over time (`--ramp-up 10s`)
- **Warmup** - Prime connections before measuring (`--warmup 5s`)
- **Config files** - TOML configuration with environment variable interpolation
- **Multiple outputs** - JSON, CSV, and Markdown formats
- **DBZ flavor** - Power levels from Farmer to OVER 9000 (toggleable with `--serious`)

## Installation

```bash
cargo install kaioken
```

## Quick Start

```bash
# Basic test
kaioken https://api.example.com/health

# With options
kaioken https://api.example.com/users \
  -c 100 \          # 100 concurrent workers
  -d 30s \          # 30 second duration
  -r 500 \          # 500 requests/sec max
  --warmup 5s       # 5 second warmup

# Headless mode for CI
kaioken https://api.example.com --no-tui --format json -o results.json
```

## TUI Preview

```
┌─────────────────────────────────────────────────────────────────┐
│  KAIOKEN x100    https://api.example.com/users    [00:15/00:30] │
├───────────────────────────────────┬─────────────────────────────┤
│  POWER LEVEL                      │  LATENCY (ms)               │
│                                   │                             │
│  Rolling RPS: 8,432    [VEGETA]   │  p50:   12    ████          │
│  Total:       126,480             │  p90:   45    ██████████    │
│  Errors:      23 (0.02%)          │  p95:   89    ████████████  │
│                                   │  p99:  230    █████████████ │
│  ▁▂▃▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇            │  p999: 567    ██████████████│
├───────────────────────────────────┼─────────────────────────────┤
│  STATUS CODES                     │  ERRORS                     │
│                                   │                             │
│  200  ████████████████████  84012 │  timeout     15             │
│  201  ██                      312 │  connect      5             │
│  500  ▏                        23 │  reset        3             │
└───────────────────────────────────┴─────────────────────────────┘
  [q]uit  [s]ave  [?]help                              Running...
```

## CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `<URL>` | required | Target URL (optional if using config file) |
| `-c, --concurrency` | 50 | Concurrent workers |
| `-d, --duration` | 10s | Test duration |
| `-r, --rate` | 0 | Max RPS (0 = unlimited) |
| `--ramp-up` | 0s | Time to reach full concurrency |
| `--warmup` | 0s | Warmup period (not measured) |
| `-m, --method` | GET | HTTP method |
| `-H, --header` | — | Header (repeatable) |
| `-b, --body` | — | Request body |
| `-f, --config` | — | TOML config file |
| `-o, --output` | — | Output file path |
| `--format` | json | Output format: json, csv, md |
| `--no-tui` | false | Headless mode |
| `--json` | false | Shorthand for `--no-tui --format json` |
| `--serious` | false | Disable DBZ flavor |
| `--insecure` | false | Skip TLS verification |

## Config File

```toml
[target]
url = "https://api.example.com/users"
method = "POST"
timeout = "5s"

[target.headers]
Authorization = "Bearer ${API_TOKEN}"
Content-Type = "application/json"

[load]
concurrency = 100
duration = "30s"
rate = 500
warmup = "5s"
```

Environment variables are interpolated with `${VAR}` or `${VAR:-default}`.

## Output Formats

### JSON
```bash
kaioken https://example.com --json
```

### CSV
```bash
kaioken https://example.com --no-tui --format csv
```

### Markdown
```bash
kaioken https://example.com --no-tui --format md -o report.md
```

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

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
