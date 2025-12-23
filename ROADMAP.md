# Kaioken Roadmap

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

## Vision

Fast local load testing against HTTP endpoints with zero setup friction, real-time visibility, deterministic artifacts, and a memorable DBZ-themed UX. CI/CD ready with checks and thresholds.

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

---

## Upcoming Milestones

### Deferred Items

Items moved from earlier milestones:

- [ ] **Check pass rate metric** - Track % of requests passing checks
- [ ] **Constant arrival rate** - Fixed RPS regardless of response time
- [ ] **Ramping arrival rate** - RPS-based stages (not worker-based)
- [ ] **Cookie jar** - Automatic session handling
- [ ] **Redirect control** - `follow_redirects = false`
- [ ] **Request groups** - Logical grouping for metrics

### v0.9 — Observability & Integration

Enterprise-grade monitoring and reporting.

- [ ] **Tags** - Label requests for filtering
  ```toml
  [[scenarios]]
  tags = { endpoint = "users", version = "v2" }
  ```
- [ ] **Prometheus metrics endpoint** - Real-time scraping during runs
- [ ] **InfluxDB export** - Time-series metrics output
- [ ] **Custom metrics** - User-defined counters/gauges
- [ ] **Improved error messages** - Suggestions for common mistakes

### v1.0 — Production Ready

Stability, documentation, and ecosystem.

- [ ] **Comprehensive test suite** - Unit, integration, e2e tests
- [ ] **Performance benchmarks** - kaioken vs wrk/vegeta/k6
- [ ] **User guide documentation** - Full docs site
- [ ] **Plugin system** - Custom output formats, checks
- [ ] **Statistical significance** - Multi-run baseline comparison

---

## Future Considerations (Post v1.0)

**Protocol Support:**
- WebSocket testing — Connection upgrade and message load
- gRPC support — Protocol buffer payloads
- GraphQL — Query-aware load testing

**Advanced Features:**
- Distributed mode — Coordinated multi-node load generation
- Lua/Rhai scripting — Dynamic request generation
- File uploads — multipart/form-data support
- Proxy support — HTTP/SOCKS proxy

**Metrics & Analysis:**
- Keep-alive metrics — Connection reuse tracking
- DNS re-resolution — For DNS-based load balancing
- Flame graphs — CPU profiling integration
- Anomaly detection — AI-powered regression detection

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
| 4 | Thresholds failed (v0.6+) |

---

## Contributing

Contributions welcome! Please open an issue to discuss significant changes before submitting PRs.
