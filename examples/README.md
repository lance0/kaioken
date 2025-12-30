# kaioken Examples

Real-world load testing configurations to get you started.

## Examples

| File | Use Case | Key Features |
|------|----------|--------------|
| [rest-api.toml](rest-api.toml) | REST API endpoint | Auth headers, checks, thresholds, ramp-up |
| [e-commerce.toml](e-commerce.toml) | Multi-endpoint flow | Weighted scenarios, tags, session handling |
| [auth-flow.toml](auth-flow.toml) | Login + authenticated requests | Request chaining, token extraction |
| [capacity-test.toml](capacity-test.toml) | Find system limits | Arrival rate stages, SQLite logging |
| [spike-test.toml](spike-test.toml) | Traffic bursts | Burst mode, resilience testing |

## Quick Start

```bash
# Run an example (replace api.example.com with your URL)
kaioken run -f examples/rest-api.toml --dry-run  # Validate first
kaioken run -f examples/rest-api.toml            # Run test

# Set required environment variables
export API_TOKEN="your-token"
export TEST_PASSWORD="your-password"
```

## CI Templates

See [ci/](ci/) for GitHub Actions and GitLab CI examples.
