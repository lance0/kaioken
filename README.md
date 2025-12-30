# kaioken

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

[![Crates.io](https://img.shields.io/crates/v/kaioken.svg)](https://crates.io/crates/kaioken)
[![License](https://img.shields.io/crates/l/kaioken.svg)](https://github.com/lance0/kaioken#license)

## Features

- **Real-time TUI** - Live metrics with latency percentiles, RPS, status codes
- **Constant arrival rate** - Fixed RPS load generation with automatic VU scaling
- **Latency correction** - Avoid coordinated omission for accurate percentiles
- **Thresholds** - CI/CD pass/fail criteria (p95 < 500ms, error_rate < 0.01, check_pass_rate > 0.95)
- **Checks** - Response validation (status codes, body content, regex) with pass rate tracking
- **Request chaining** - Extract values from responses for subsequent requests
- **Stages** - Multi-phase load profiles (ramp up → hold → ramp down)
- **Weighted scenarios** - Multi-endpoint testing with traffic distribution and tags
- **Cookie jar** - Automatic session handling across requests
- **Rate limiting** - Token bucket algorithm for controlled load
- **Ramp-up & warmup** - Gradual worker activation and connection priming
- **Compare mode** - Regression detection with CI-friendly exit codes
- **Multiple outputs** - JSON, CSV, Markdown, and HTML reports
- **Variable interpolation** - Dynamic `${REQUEST_ID}`, `${TIMESTAMP_MS}`, and extracted values
- **HTTP/2 support** - Optional h2 prior knowledge mode
- **Proxy support** - HTTP, HTTPS, and SOCKS5 proxies
- **Basic auth** - Simple user:password authentication
- **Multipart forms** - File uploads with curl-like `-F` syntax
- **Client certificates** - mTLS authentication for enterprise APIs
- **Debug mode** - Single request with full request/response dump
- **Random regex URLs** - Generate dynamic URLs from regex patterns
- **Burst mode** - Spike testing with N requests, delay, repeat
- **SQLite logging** - Export snapshots to SQLite for analysis
- **DNS override** - Route requests to different hosts (--connect-to)
- **Prometheus export** - Real-time metrics for Grafana dashboards
- **DBZ themes** - 6 color schemes (press `t` to cycle)

## vs Other Tools

| Feature | kaioken | k6 | oha | wrk | Gatling |
|---------|:-------:|:--:|:---:|:---:|:-------:|
| **Real-time TUI** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Zero config** | ✅ | ❌ | ✅ | ✅ | ❌ |
| **Compare mode** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Latency correction** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **HTML reports** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Checks/thresholds** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Stages** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Arrival rate** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Request chaining** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Weighted scenarios** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Cookie jar** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **HTTP/2** | ✅ | ✅ | ✅ | ❌ | ✅ |
| **HTTP/3** | ✅* | ❌ | ✅ | ❌ | ❌ |
| **WebSocket** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **gRPC** | ✅* | ✅ | ❌ | ❌ | ✅ |
| **Proxy** | ✅ | ✅ | ✅ | ❌ | ✅ |
| **Basic auth** | ✅ | ✅ | ✅ | ❌ | ✅ |
| **Multipart upload** | ✅ | ✅ | ✅ | ❌ | ✅ |
| **Client certs (mTLS)** | ✅ | ✅ | ✅ | ❌ | ✅ |
| **Random regex URLs** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Burst mode** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **SQLite logging** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Prometheus export** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Config file** | TOML | JS | ❌ | Lua | Scala |
| **Language** | Rust | Go | Rust | C | Scala |

*\* Experimental feature*

**kaioken strengths:** Real-time visibility, regression detection, CI/CD thresholds, load stages, request chaining, latency correction, memorable UX

## Installation

### Pre-built binaries (recommended)

Download from [GitHub Releases](https://github.com/lance0/kaioken/releases):

```bash
# Linux x86_64
curl -LO https://github.com/lance0/kaioken/releases/latest/download/kaioken-linux-x86_64.tar.gz
tar xzf kaioken-linux-x86_64.tar.gz
sudo mv kaioken /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/lance0/kaioken/releases/latest/download/kaioken-macos-aarch64.tar.gz
tar xzf kaioken-macos-aarch64.tar.gz
sudo mv kaioken /usr/local/bin/

# macOS (Intel)
curl -LO https://github.com/lance0/kaioken/releases/latest/download/kaioken-macos-x86_64.tar.gz
tar xzf kaioken-macos-x86_64.tar.gz
sudo mv kaioken /usr/local/bin/
```

### Homebrew (macOS/Linux)

```bash
brew tap lance0/kaioken
brew install kaioken
```

### Cargo (from source)

```bash
cargo install kaioken

# With HTTP/3 support (experimental)
cargo install kaioken --features http3

# With gRPC support (experimental)
cargo install kaioken --features grpc

# With all features
cargo install kaioken --features "http3 grpc"
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

## 5-Minute Tutorial

This tutorial walks you through testing an API endpoint, from basic test to CI/CD integration.

### Step 1: Run Your First Test

```bash
# Test an endpoint with 50 concurrent users for 10 seconds
kaioken run https://httpbin.org/get
```

You'll see a real-time TUI showing requests/sec, latency percentiles, and status codes. Press `q` to quit early or wait for completion.

### Step 2: Create a Config File

For repeatable tests, create a config file:

```bash
kaioken init --url https://api.example.com/users -o api-test.toml
```

Edit `api-test.toml` to customize:

```toml
[target]
url = "https://api.example.com/users"
method = "GET"

[target.headers]
Authorization = "Bearer ${API_TOKEN}"  # Uses environment variable

[load]
concurrency = 100
duration = "30s"
ramp_up = "5s"     # Gradually add workers
warmup = "3s"      # Exclude from metrics
```

Run with: `kaioken run -f api-test.toml`

### Step 3: Add Response Validation

Ensure your API returns correct responses:

```toml
[[checks]]
name = "status is 200"
status = 200

[[checks]]
name = "response has users"
body_contains = "users"

[[checks]]
name = "response time OK"
max_latency = "500ms"
```

The TUI shows check pass rates in real-time.

### Step 4: Set CI/CD Thresholds

Fail the test if performance degrades:

```toml
[thresholds]
p95_latency = "< 500ms"     # 95th percentile under 500ms
p99_latency = "< 1s"        # 99th percentile under 1s
error_rate = "< 0.01"       # Less than 1% errors
check_pass_rate = "> 0.95"  # 95% of checks pass
rps = "> 100"               # At least 100 req/s
```

Exit code is non-zero if thresholds are breached.

### Step 5: Compare Results for Regressions

Save results and compare against baselines:

```bash
# Save baseline
kaioken run -f api-test.toml -o baseline.json

# After code changes, compare
kaioken run -f api-test.toml -o current.json
kaioken compare baseline.json current.json
```

`compare` exits with code 3 if regressions detected—perfect for CI gates.

### Step 6: Run in CI/CD

```yaml
# .github/workflows/load-test.yml
- name: Load test
  run: |
    kaioken run -f api-test.toml -o results.json --no-tui
    kaioken compare baseline.json results.json
  env:
    API_TOKEN: ${{ secrets.API_TOKEN }}
```

### What's Next?

- **Arrival rate mode**: Fixed RPS with `--arrival-rate 100`
- **Stages**: Ramp up/down with `[[stages]]`
- **Request chaining**: Extract tokens with `[extraction]`
- **Weighted scenarios**: Multi-endpoint with `[[scenarios]]`

See the [examples/](examples/) folder for complete configurations.

## TUI Preview

![kaioken TUI](kaioken.png)

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
| `-n, --max-requests` | 0 | Stop after N requests (0 = unlimited, supports k/m suffixes) |
| `-r, --rate` | 0 | Max RPS (0 = unlimited) |
| `--ramp-up` | 0s | Time to reach full concurrency |
| `--warmup` | 0s | Warmup period (not measured) |
| `--think-time` | — | Pause between requests (e.g., 500ms) |
| `--arrival-rate` | 0 | Target RPS (enables arrival rate mode) |
| `--max-vus` | 100 | Max VUs for arrival rate mode |
| `--no-latency-correction` | false | Disable latency correction |
| `--no-follow-redirects` | false | Don't follow HTTP redirects |
| `-m, --method` | GET | HTTP method |
| `-H, --header` | — | Header (repeatable) |
| `-b, --body` | — | Request body |
| `--body-file` | — | Load body from file |
| `--http2` | false | Use HTTP/2 prior knowledge |
| `--cookie-jar` | false | Enable cookie jar for session handling |
| `-f, --config` | — | TOML config file |
| `-o, --output` | — | Output file path |
| `--format` | json | Output format: json, csv, md, html |
| `--no-tui` | false | Headless mode |
| `--json` | false | Shorthand for `--no-tui --format json` |
| `--dry-run` | false | Validate config and exit |
| `--debug` | false | Send single request, print full dump |
| `--fail-fast` | false | Abort immediately on threshold breach |
| `--serious` | false | Disable DBZ flavor |
| `--insecure` | false | Skip TLS verification |
| `--disable-keepalive` | false | Disable connection reuse |
| `-y, --yes` | false | Skip remote target confirmation |
| `-x, --proxy` | — | Proxy URL (http/https/socks5) |
| `-a, --basic-auth` | — | Basic auth credentials (user:pass) |
| `-F, --form` | — | Multipart form field (repeatable) |
| `--cert` | — | Client certificate (PEM) for mTLS |
| `--key` | — | Client private key (PEM) for mTLS |
| `--cacert` | — | CA certificate (PEM) for custom CA |
| `--rand-regex-url` | — | Generate URLs from regex pattern |
| `--urls-from-file` | — | Read URLs from file (round-robin) |
| `-Z, --body-lines` | — | Body lines from file (round-robin) |
| `--connect-to` | — | DNS override (HOST:TARGET_IP:TARGET_PORT) |
| `--db-url` | — | SQLite database for snapshot logging |
| `--burst-rate` | — | Requests per burst (enables burst mode) |
| `--burst-delay` | — | Delay between bursts (e.g., 1s) |
| `--prometheus-pushgateway` | — | Push metrics to Prometheus Pushgateway URL |
| `--prometheus-port` | — | Expose /metrics endpoint on this port |
| `--http3` | false | Use HTTP/3 (QUIC) - experimental |
| `--grpc-service` | — | gRPC service name (experimental) |
| `--grpc-method` | — | gRPC method name (experimental) |

### `kaioken compare`

```
kaioken compare <BASELINE> <CURRENT> [OPTIONS]
```

Compare two JSON result files for regressions. Prints load model metadata and validates compatibility.

| Flag | Default | Description |
|------|---------|-------------|
| `--threshold-p99` | 10.0 | p99 latency regression threshold (%) |
| `--threshold-p999` | 15.0 | p999 latency regression threshold (%) |
| `--threshold-error-rate` | 50.0 | Error rate regression threshold (%) |
| `--threshold-rps` | 10.0 | RPS regression threshold (%) |
| `--force` | false | Allow comparing different load models (open vs closed) |
| `--json` | false | Output as JSON |

Exit codes: 0 (success), 3 (regressions), 5 (load model mismatch without --force)

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

### `kaioken man`

```
kaioken man > kaioken.1
man -l kaioken.1
```

Generate man page in roff format.

### `kaioken import`

```
kaioken import <FILE> [OPTIONS]
```

Convert HAR (HTTP Archive) files from browser DevTools to kaioken config.

| Flag | Default | Description |
|------|---------|-------------|
| `<FILE>` | — | HAR file to import |
| `-o, --output` | kaioken.toml | Output file path |
| `--filter` | — | URL regex filter (e.g., "api/v2") |

```bash
# Import from Chrome DevTools HAR export
kaioken import recording.har -o load-test.toml

# Filter by URL pattern
kaioken import api.har --filter "api/v2" -o filtered.toml
```

The importer:
- Auto-detects format from file extension
- Preserves headers, body, and method from HAR entries
- Creates weighted scenarios from duplicate requests
- Filters browser-specific headers (cookies, sec-*, etc.)

## Config File

```toml
[target]
url = "https://api.example.com/users"
method = "POST"
timeout = "5s"
connect_timeout = "2s"
# http2 = false
# insecure = false
# cookie_jar = false  # Enable for session handling
# follow_redirects = true  # Set false to not follow redirects
# disable_keepalive = false  # Disable connection reuse

# Authentication & security
# proxy = "http://proxy:8080"  # HTTP/HTTPS/SOCKS5 proxy
# basic_auth = "user:password"  # Basic authentication
# cert = "/path/to/client.crt"  # Client certificate (mTLS)
# key = "/path/to/client.key"   # Client private key (mTLS)
# cacert = "/path/to/ca.crt"    # Custom CA certificate

# Multipart form data (alternative to body)
# form_data = ["field=value", "file=@/path/to/upload.txt"]

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
# think_time = "500ms"

# Arrival rate mode (alternative to concurrency)
# arrival_rate = 100  # Fixed 100 RPS
# max_vus = 200       # Cap on concurrent VUs
```

Environment variables: `${VAR}` or `${VAR:-default}`

## Constant Arrival Rate

Generate load at a fixed RPS regardless of response times. VUs scale automatically.

```bash
# CLI: 100 RPS with up to 50 VUs
kaioken run --arrival-rate 100 --max-vus 50 -d 1m https://api.example.com
```

```toml
[load]
arrival_rate = 100  # Target: 100 requests/second
max_vus = 200       # Max concurrent VUs (auto-scales)
duration = "5m"
```

### Ramping Arrival Rate (Stages)

Use `target_rate` in stages for RPS-based load profiles:

```toml
[load]
max_vus = 200

[[stages]]
duration = "1m"
target_rate = 50    # Ramp up to 50 RPS

[[stages]]
duration = "5m"
target_rate = 200   # Ramp to 200 RPS

[[stages]]
duration = "1m"
target_rate = 0     # Ramp down
```

**How it works:**
- Iterations spawn at the target rate (e.g., 100/sec = one every 10ms)
- If responses are slow, more VUs are allocated (up to `max_vus`)
- If all VUs are busy, iterations are **dropped** and tracked
- Dropped iterations indicate the system can't sustain the target rate

**vs Rate Limiting (`--rate`):**
- `--rate` limits an existing pool of workers (caps RPS from above)
- `--arrival-rate` maintains a constant RPS (spawns work from below)

## Latency Correction

When using arrival rate mode, latency correction is automatically enabled to avoid the [coordinated omission problem](https://www.scylladb.com/2021/04/22/on-coordinated-omission/).

When the server slows down, requests queue waiting for available VUs. Without correction, this queue time inflates latency percentiles. With correction:

- **Queue time** is tracked separately (time waiting for a VU)
- **Corrected latency** = total latency - queue time (actual server response time)
- TUI shows `[corrected]` indicator when active
- JSON output includes both `corrected_latency_us` and `queue_time_us`

Disable with `--no-latency-correction` if you want wall-clock latency instead.

## Thresholds

Define pass/fail criteria for CI/CD pipelines:

```toml
[thresholds]
p95_latency_ms = "< 500"
p99_latency_ms = "< 1000"
error_rate = "< 0.01"
rps = "> 100"
check_pass_rate = "> 0.95"  # 95% of checks must pass
```

Available metrics:
- `p50_latency_ms`, `p75_latency_ms`, `p90_latency_ms`, `p95_latency_ms`, `p99_latency_ms`, `p999_latency_ms`
- `mean_latency_ms`, `max_latency_ms`
- `error_rate` (0.0 - 1.0)
- `rps` (requests per second)
- `check_pass_rate` (0.0 - 1.0) - percentage of checks passing

Operators: `<`, `<=`, `>`, `>=`, `==`

Exit codes:
- `0` - Success
- `1` - Error (high error rate, config issues)
- `3` - Regressions detected (compare mode)
- `4` - Thresholds failed
- `5` - Load model mismatch in compare (without --force)

## Checks

Validate response status codes and body content:

```toml
[[checks]]
name = "status_ok"
condition = "status == 200"

[[checks]]
name = "success_codes"
condition = "status in [200, 201, 204]"

[[checks]]
name = "has_data"
condition = "body contains \"success\""

[[checks]]
name = "valid_json"
condition = "body matches \"\\{.*\\}\""
```

Check results are displayed after the test with pass/fail percentages.

## Request Chaining

Extract values from responses and use in subsequent requests:

```toml
[[scenarios]]
name = "login"
url = "https://api.example.com/auth"
method = "POST"
body = '{"user": "test", "pass": "secret"}'
weight = 0  # weight=0 means dependency only

[scenarios.extract]
token = "json:$.access_token"
session_id = "header:X-Session-Id"

[[scenarios]]
name = "get_profile"
url = "https://api.example.com/me"
method = "GET"
weight = 10

[scenarios.headers]
Authorization = "Bearer ${token}"
```

Extraction sources:
- `json:$.path.to.value` - JSONPath extraction
- `regex:pattern:group` - Regex capture group
- `body` - Entire response body

Extracted values are available as `${varname}` in URLs, headers, and body.

## Stages

Define multi-phase load profiles (ramp up, hold, ramp down):

```toml
[target]
url = "https://api.example.com/health"

[[stages]]
duration = "30s"
target = 50      # ramp to 50 workers

[[stages]]
duration = "2m"
target = 50      # hold at 50

[[stages]]
duration = "30s"
target = 0       # ramp down to 0
```

When stages are configured:
- Total duration is calculated automatically
- Max worker count is determined from highest target
- Workers ramp up/down gradually within each stage

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
tags = { endpoint = "users", version = "v2" }

[[scenarios]]
name = "create_user"
url = "https://api.example.com/users"
method = "POST"
body = '{"name": "test-${REQUEST_ID}"}'
weight = 2  # 20% of traffic
tags = { endpoint = "users", operation = "write" }

[[scenarios]]
name = "health_check"
url = "https://api.example.com/health"
method = "GET"
weight = 1  # 10% of traffic
```

Tags are optional metadata for organizing and filtering scenarios in output.

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

## WebSocket Testing

Test WebSocket endpoints with echo or fire-and-forget modes:

```bash
# Echo mode (default) - measure RTT
kaioken run ws://localhost:8080/ws -c 100 -d 30s -b '{"type":"ping"}'

# Fire-and-forget - measure throughput
kaioken run ws://localhost:8080/events -c 50 --ws-fire-and-forget
```

TOML config:
```toml
[target]
url = "wss://api.example.com/ws"

[websocket]
message_interval = "100ms"
mode = "echo"  # or "fire_and_forget"
```

## Proxy Support

Route requests through HTTP, HTTPS, or SOCKS5 proxies:

```bash
# HTTP proxy
kaioken run https://api.example.com -x http://proxy:8080

# SOCKS5 proxy
kaioken run https://api.example.com -x socks5://127.0.0.1:1080

# Authenticated proxy
kaioken run https://api.example.com -x http://user:pass@proxy:8080
```

TOML config:
```toml
[target]
url = "https://api.example.com"
proxy = "http://proxy:8080"
```

## Basic Authentication

Authenticate with username and password:

```bash
kaioken run https://api.example.com -a admin:secret
```

TOML config:
```toml
[target]
url = "https://api.example.com"
basic_auth = "admin:secret"
```

## Multipart Form Upload

Upload files and form data using curl-like syntax:

```bash
# Text field
kaioken run https://api.example.com -F "name=value"

# File upload
kaioken run https://api.example.com -F "file=@/path/to/upload.txt"

# File with custom filename and MIME type
kaioken run https://api.example.com -F "doc=@report.pdf;filename=final.pdf;type=application/pdf"

# Multiple fields
kaioken run https://api.example.com -m POST \
  -F "user=test" \
  -F "avatar=@photo.jpg"
```

TOML config:
```toml
[target]
url = "https://api.example.com"
method = "POST"
form_data = ["field1=value1", "file=@/path/to/upload.txt"]
```

Note: `--form` and `--body` are mutually exclusive.

## Client Certificates (mTLS)

Authenticate with client certificates for mutual TLS:

```bash
# Client certificate + key
kaioken run https://secure.example.com --cert client.crt --key client.key

# With custom CA (for self-signed server certs)
kaioken run https://secure.example.com \
  --cert client.crt --key client.key --cacert ca.crt
```

TOML config:
```toml
[target]
url = "https://secure.example.com"
cert = "/path/to/client.crt"
key = "/path/to/client.key"
cacert = "/path/to/ca.crt"  # optional
```

Note: `--cert` and `--key` must be used together. Certificates must be in PEM format.

## Debug Mode

Send a single request and print full request/response details before running a load test:

```bash
# Simple GET
kaioken run https://api.example.com/health --debug

# POST with headers and body
kaioken run https://api.example.com/users \
  -m POST \
  -H "Content-Type: application/json" \
  -b '{"name":"test"}' \
  --debug
```

Output shows:
- Request: method, URL, headers, body (JSON pretty-printed)
- Response: status, latency, headers, body
- Errors: with actionable suggestions

## Disable Keepalive

Disable HTTP connection reuse to measure connection establishment overhead:

```bash
kaioken run https://api.example.com -c 10 -d 30s --disable-keepalive
```

TOML config:
```toml
[target]
url = "https://api.example.com"
disable_keepalive = true
```

Each request creates a new TCP connection. Useful for measuring TLS handshake and connection overhead.

## HTTP/3 (Experimental)

Build with HTTP/3 support and use QUIC transport:

```bash
cargo install kaioken --features http3

kaioken run https://quic.example.com --http3
```

Requires the target server to support HTTP/3.

**Limitations:** HTTP/3 mode uses simple constant-VU execution. Options like
`--arrival-rate`, `--rate`, `--think-time`, `--ramp-up`, and `[[scenarios]]`
are ignored. Use standard HTTP mode for these features.

## gRPC (Experimental)

Build with gRPC support to load test gRPC services:

```bash
cargo install kaioken --features grpc

# Unary call with inline body
kaioken run https://localhost:50051 \
  --grpc-service "helloworld.Greeter" \
  --grpc-method "SayHello" \
  -b 'raw protobuf bytes here' \
  -c 50 -d 30s

# Or load binary protobuf from file
kaioken run https://localhost:50051 \
  --grpc-service "helloworld.Greeter" \
  --grpc-method "SayHello" \
  --body-file request.bin \
  -c 50 -d 30s
```

Supports unary calls and server streaming. The request body should be **raw protobuf-encoded bytes**. Use `--body-file` to load binary protobuf data from a file. JSON-to-protobuf conversion is not currently supported.

**Limitations:** gRPC mode uses simple constant-VU execution. Options like
`--arrival-rate`, `--rate`, `--think-time`, `--ramp-up`, and `[[scenarios]]` are ignored.
The `--insecure` flag is not supported; use `http://` URLs for unencrypted connections.

## Prometheus Metrics Export

Export real-time metrics to Prometheus for Grafana dashboards. Two modes available:

### Push to Pushgateway

Push metrics every 100ms to a Prometheus Pushgateway:

```bash
# Start Pushgateway (Docker)
docker run -d -p 9091:9091 prom/pushgateway

# Run load test with metrics push
kaioken run https://api.example.com -c 50 -d 60s \
  --prometheus-pushgateway http://localhost:9091

# View metrics
curl http://localhost:9091/metrics | grep kaioken
```

### Expose /metrics Endpoint

Serve a Prometheus-compatible HTTP endpoint for scraping:

```bash
# Run load test with metrics endpoint
kaioken run https://api.example.com -c 50 -d 60s --prometheus-port 9090

# Scrape metrics (in another terminal)
curl http://localhost:9090/metrics
```

### TOML Config

```toml
[load]
# Option 1: Push to Pushgateway
prometheus_pushgateway = "http://localhost:9091"

# Option 2: Expose endpoint (mutually exclusive with pushgateway)
# prometheus_port = 9090
```

### Available Metrics

All metrics are prefixed with `kaioken_` and include labels `job="kaioken"` and `instance="<target_url>"`:

| Metric | Type | Description |
|--------|------|-------------|
| `kaioken_requests_total` | Counter | Total requests made |
| `kaioken_requests_success_total` | Counter | Successful requests |
| `kaioken_requests_failed_total` | Counter | Failed requests |
| `kaioken_rps` | Gauge | Current requests per second |
| `kaioken_error_rate` | Gauge | Current error rate (0.0-1.0) |
| `kaioken_latency_p50_ms` | Gauge | 50th percentile latency |
| `kaioken_latency_p95_ms` | Gauge | 95th percentile latency |
| `kaioken_latency_p99_ms` | Gauge | 99th percentile latency |
| `kaioken_latency_p999_ms` | Gauge | 99.9th percentile latency |
| `kaioken_vus_active` | Gauge | Active virtual users |
| `kaioken_vus_max` | Gauge | Maximum virtual users |
| `kaioken_bytes_received_total` | Counter | Total bytes received |
| `kaioken_dropped_iterations_total` | Counter | Dropped iterations (arrival rate) |

### Grafana Queries

```promql
# RPS over time
kaioken_rps{job="kaioken"}

# P99 latency in milliseconds
kaioken_latency_p99_ms{job="kaioken"}

# Error rate as percentage
kaioken_error_rate{job="kaioken"} * 100

# Active VUs
kaioken_vus_active{job="kaioken"}

# Total requests (counter)
rate(kaioken_requests_total{job="kaioken"}[1m])
```

## CI Integration

```yaml
# GitHub Actions example with thresholds
- name: Load test with thresholds
  run: |
    cat > test.toml << EOF
    [target]
    url = "https://api.example.com/health"
    
    [load]
    concurrency = 50
    duration = "30s"
    
    [thresholds]
    p95_latency_ms = "< 500"
    error_rate = "< 0.01"
    EOF
    
    kaioken run -f test.toml --no-tui -o results.json -y
    # Exits with code 4 if thresholds fail

- name: Check for regressions (optional)
  run: |
    kaioken compare baseline.json results.json \
      --threshold-p99 15 --threshold-rps 10
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

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
