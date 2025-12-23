# Kaioken Roadmap

A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor.

## Vision

Fast local load testing against HTTP endpoints with zero setup friction, real-time visibility, deterministic artifacts, and a memorable DBZ-themed UX.

---

## Milestones

### v0.1 — Core (Week 1)

- [x] CLI: url, concurrency, duration, timeout, output flags
- [x] Engine: worker pool, unlimited mode
- [x] Aggregator: histogram, status codes, error counts, rolling RPS
- [x] TUI: power panel, latency percentiles, status table
- [x] JSON export
- [x] DBZ flavor (toggleable with `--serious`)

### v0.2 — Full Load Control (Week 2)

- [ ] Headers, method, body support
- [ ] Rate cap (token bucket)
- [ ] Ramp-up scheduling
- [ ] Warmup period
- [ ] Timeline sparkline in TUI
- [ ] TOML config file support
- [ ] CSV/Markdown output formats

### v0.3 — Compare Mode (Week 3)

- [ ] `kaioken compare` subcommand
- [ ] Compare TUI view with diff visualization
- [ ] Regression detection (p99, error rate, RPS thresholds)
- [ ] Config comparability warnings

### v0.4 — Polish (Week 4)

- [ ] Weighted scenarios (multi-endpoint testing)
- [ ] HTTP/2 toggle
- [ ] Body read with configurable cap
- [ ] Statistical significance in compare mode
- [ ] `--max-requests` cap option
- [ ] Variable interpolation (`${ENV_VAR}`, `${REQUEST_ID}`)

---

## Future Considerations

Beyond v0.4, potential directions include:

- **Distributed load generation** — Coordinated multi-node testing
- **Keep-alive metrics** — Connection reuse rate tracking
- **Request IDs** — Optional per-request tracing
- **DNS re-resolution** — Periodic re-resolve for DNS-based load balancing
- **HTTP/2 stream metrics** — Detailed multiplexing visibility

---

## Contributing

Contributions welcome! Please open an issue to discuss significant changes before submitting PRs.
