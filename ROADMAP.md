# Kaioken Roadmap

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

## Vision

Fast local load testing against HTTP endpoints with zero setup friction, real-time visibility, deterministic artifacts, and a memorable DBZ-themed UX. CI/CD ready with checks and thresholds.

## Competitive Positioning

| Feature | kaioken | k6 | Gatling | Locust |
|---------|:-------:|:--:|:-------:|:------:|
| **Real-time TUI** | ✅ Unique | ❌ | ❌ | ❌ |
| **Compare/Regression** | ✅ Unique | ❌ | ❌ | ❌ |
| **Zero Config** | ✅ | ❌ | ❌ | ❌ |
| **Constant Arrival Rate** | ✅ | ✅ | ✅ | ✅ |
| **CI/CD Thresholds** | ✅ | ✅ | ✅ | ❌ |
| **Request Chaining** | ✅ | ✅ | ✅ | ✅ |
| **WebSocket** | 🔜 v1.1 | ✅ | ✅ | ❌ |
| **gRPC** | 🔜 v1.2 | ✅ | ✅ | ❌ |
| **Language** | Rust | Go | Scala | Python |

---

## Completed Milestones

### v0.1 — Core ✓

- [x] CLI with clap (url, -c, -d, --timeout, -o, -H, -m, -b)
- [x] Engine: concurrent worker pool, unlimited throughput mode
- [x] HTTP client with reqwest (connection pooling, timeouts, TLS)
- [x] Stats: HDR histogram, status codes, error classification
- [x] TUI: power panel, latency bars, status codes, errors, sparkline
- [x] JSON export with full metrics
- [x] DBZ flavor (`--serious` to disable)
- [x] Headless mode (`--no-tui`, `--json`)
- [x] Safety warning for remote targets

### v0.2 — Load Control ✓

- [x] Rate limiting (`-r, --rate`) with token bucket algorithm
- [x] Ramp-up (`--ramp-up`) - gradually activate workers
- [x] Warmup (`--warmup`) - discard initial metrics, prime connections
- [x] TOML config file support (`-f, --config`)
- [x] CSV output format (`--format csv`)
- [x] Markdown output format (`--format md`)
- [x] Environment variable interpolation in config (`${VAR}`)

### v0.3 — Compare Mode ✓

- [x] `kaioken compare <baseline.json> <current.json>` subcommand
- [x] Side-by-side diff table (RPS, latency percentiles, error rate)
- [x] Regression detection with configurable thresholds
- [x] Exit code 3 on regression (for CI gating)
- [x] Config compatibility warnings

### v0.4 — Advanced Features ✓

- [x] `--max-requests` cap (stop after N requests)
- [x] Body from file (`--body-file`)
- [x] HTTP/2 support toggle (`--http2`)
- [x] Variable interpolation (`${REQUEST_ID}`, `${TIMESTAMP_MS}`)
- [x] DBZ theme cycle in TUI (`t` key) - 6 themes

### v0.5 — Polish & DX ✓

- [x] Weighted scenarios (`[[scenarios]]` in TOML)
- [x] `kaioken init` - generate starter config file
- [x] Shell completions (bash, zsh, fish)
- [x] `--dry-run` mode (validate config without running)
- [x] Man page generation (`kaioken man`)
- [x] HTML report export (`--format html`)

### v0.6 — Checks & Thresholds ✓

- [x] Thresholds in config (`[thresholds]` section)
- [x] Exit code 4 when thresholds fail
- [x] Threshold results in JSON output
- [x] Status checks (`[[checks]]` with `status == 200`, `status in [...]`)

### v0.7 — Load Profiles & Stages ✓

- [x] Stages (`[[stages]]` with duration and target)
- [x] Auto-calculated total duration from stages
- [x] Gradual worker ramp up/down between stages

### v0.8 — Request Chaining & Checks ✓

- [x] Response body checks (`body contains`, `body matches`)
- [x] Response data extraction (`json:`, `regex:`, `body`)
- [x] Variable interpolation with extracted values (`${varname}`)

### v0.9 — Tags, Checks & Sessions ✓

- [x] **Check pass rate metric** - Track % of requests passing checks
- [x] **Check pass rate threshold** - `check_pass_rate = "> 0.95"` for CI/CD
- [x] **Tags** - Label scenarios for filtering in output
- [x] **Cookie jar** - Automatic session handling (`--cookie-jar`)
- [x] **Checks in JSON output** - Per-check and overall pass rates

---

## Upcoming Milestones

### v1.0 — Constant & Ramping Arrival Rate ✓

- [x] **Constant arrival rate executor** - Fixed RPS regardless of response time
  ```toml
  [load]
  arrival_rate = 100  # 100 requests/second, VUs scale automatically
  max_vus = 500       # Cap on concurrent VUs
  duration = "5m"
  ```
- [x] **Ramping arrival rate** - RPS-based stages with gradual transitions
  ```toml
  [[stages]]
  duration = "1m"
  target_rate = 100   # Ramp to 100 RPS
  
  [[stages]]
  duration = "5m"
  target_rate = 500   # Hold at 500 RPS
  ```
- [x] **Dropped iteration tracking** - Metric when VUs can't keep up with rate
- [x] **Auto VU scaling** - Dynamically spawn/retire VUs to maintain rate
- [x] CLI flags: `--arrival-rate` and `--max-vus`
- [x] TUI display for VUs active/max and dropped iterations

### v1.1 — WebSocket Support

Essential for modern real-time applications.

- [ ] **WebSocket connections** - `ws://` and `wss://` protocol support
- [ ] **Message send/receive** - Bidirectional messaging load tests
- [ ] **Connection lifecycle** - Open, message, close patterns
- [ ] **WebSocket checks** - Validate message content

### v1.2 — gRPC Support

Critical for microservices architectures.

- [ ] **gRPC unary calls** - Request/response pattern
- [ ] **Protobuf support** - Load .proto files or reflection
- [ ] **gRPC streaming** - Client, server, and bidirectional streams
- [ ] **gRPC checks** - Status codes, response validation

### v1.3 — Production Polish

- [ ] **Comprehensive test suite** - Unit, integration, e2e tests
- [ ] **Performance benchmarks** - kaioken vs wrk/vegeta/k6
- [ ] **Improved error messages** - Suggestions for common mistakes
- [ ] **Redirect control** - `follow_redirects = false`

---

## Future Considerations (Post v1.x)

**Observability:**
- Prometheus metrics endpoint — Real-time scraping during runs
- InfluxDB/OpenTelemetry export — Time-series metrics output
- Custom metrics — User-defined counters/gauges

**Protocol Support:**
- GraphQL — Query-aware load testing with introspection
- MQTT — IoT protocol testing
- Kafka — Message queue load testing

**Advanced Features:**
- Distributed mode — Coordinated multi-node load generation
- Lua/Rhai scripting — Dynamic request generation
- File uploads — multipart/form-data support
- Proxy support — HTTP/SOCKS proxy
- Statistical significance — Multi-run baseline comparison

**Metrics & Analysis:**
- Keep-alive metrics — Connection reuse tracking
- DNS re-resolution — For DNS-based load balancing
- Flame graphs — CPU profiling integration
- AI-powered anomaly detection — Smart regression detection

---

## Non-Goals (v1.x)

- Browser automation / JavaScript execution
- Distributed coordination requiring external infrastructure
- "Pure server latency" measurement (includes client overhead by design)
- Comprehensive TLS/cert testing matrix
- GUI application (TUI is the interface)

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (high error rate, config issues) |
| 3 | Regressions detected (compare mode) |
| 4 | Thresholds failed |
| 5 | Load model mismatch in compare (without --force) |

---

## Contributing

Contributions welcome! Please open an issue to discuss significant changes before submitting PRs.
