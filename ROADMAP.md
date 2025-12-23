# Kaioken Roadmap

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

## Vision

Fast local load testing against HTTP endpoints with zero setup friction, real-time visibility, deterministic artifacts, and a memorable DBZ-themed UX.

---

## Milestones

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
- [x] DBZ theme cycle in TUI (`t` key) - Earth, Namek, Planet Vegeta, Time Chamber, Tournament, Frieza Force

### v0.5 — Polish & DX

- [ ] Weighted scenarios (`[[scenarios]]` in TOML)
- [ ] `kaioken init` - generate starter config file
- [ ] Shell completions (bash, zsh, fish)
- [ ] Man page generation
- [ ] Improved error messages with suggestions
- [ ] `--dry-run` mode (validate config without running)
- [ ] Statistical significance in compare (multi-run baselines)

---

## Future Considerations

- **Distributed mode** — Coordinated multi-node load generation
- **Lua/Rhai scripting** — Dynamic request generation
- **WebSocket testing** — Connection upgrade and message load
- **gRPC support** — Protocol buffer payloads
- **Keep-alive metrics** — Connection reuse tracking
- **DNS re-resolution** — For DNS-based load balancing testing
- **Prometheus metrics endpoint** — Real-time scraping during runs
- **HTML report export** — Shareable standalone reports

---

## Non-Goals (v0.x)

- Browser automation / JavaScript execution
- Distributed coordination (single-node focus)
- "Pure server latency" (includes client overhead by design)
- Comprehensive TLS/cert testing matrix

---

## Contributing

Contributions welcome! Please open an issue to discuss significant changes before submitting PRs.
