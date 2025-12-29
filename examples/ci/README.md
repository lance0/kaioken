# CI/CD Integration Examples

Ready-to-use templates for integrating kaioken load tests into your CI/CD pipeline.

## Quick Start

### GitHub Actions

Copy `github-actions.yml` to `.github/workflows/load-test.yml` in your repository:

```bash
mkdir -p .github/workflows
cp examples/ci/github-actions.yml .github/workflows/load-test.yml
```

Set your target URL as a repository secret:
- Go to Settings → Secrets → Actions
- Add `LOAD_TEST_URL` with your API endpoint

### GitLab CI

Include the template in your `.gitlab-ci.yml`:

```yaml
include:
  - local: 'examples/ci/gitlab-ci.yml'

variables:
  TARGET_URL: "https://api.example.com/health"
```

Or copy the contents directly into your `.gitlab-ci.yml`.

## Features

Both templates include:

- **Automatic load testing** on push/PR to main branch
- **Threshold checking** - fail pipeline if latency or error rate exceeds limits
- **Regression detection** - compare against baseline from main branch
- **Artifact storage** - JSON results saved for 30 days
- **Manual triggers** - run with custom duration/concurrency

## Using Config Files

For more control, create a `kaioken.toml` in your repo:

```toml
[target]
url = "https://api.example.com/health"
method = "GET"
timeout = "5s"

[load]
concurrency = 50
duration = "30s"
warmup = "5s"

[thresholds]
p99_latency_ms = "< 500"    # Fail if p99 > 500ms
error_rate = "< 0.01"        # Fail if error rate > 1%

[[checks]]
name = "status is 200"
condition = "status == 200"
```

Then run with:
```bash
kaioken run -f kaioken.toml --no-tui -o results.json -y
```

Exit codes:
- `0` - Success
- `1` - Error (config issues, high error rate)
- `4` - Thresholds failed

## Regression Detection

The `kaioken compare` command detects performance regressions:

```bash
kaioken compare baseline.json current.json \
  --threshold-p99 10 \         # 10% p99 regression allowed
  --threshold-error-rate 50    # 50% error rate increase allowed
```

Exit code `3` indicates regression detected.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LOAD_TEST_URL` | Target API endpoint | `https://httpbin.org/get` |
| `DURATION` | Test duration | `30s` |
| `CONCURRENCY` | Concurrent users | `50` |

## Tips

1. **Start conservative** - Begin with shorter durations (30s) and lower concurrency (10-20)
2. **Use warmup** - Add `--warmup 5s` to prime connections before measuring
3. **Separate environments** - Run load tests against staging, not production
4. **Monitor during tests** - Check your APM/metrics during load tests
5. **Store baselines** - Keep successful results as baselines for comparison
