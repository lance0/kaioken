use crate::types::{LoadConfig, StatsSnapshot};
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub fn write_markdown(snapshot: &StatsSnapshot, config: &LoadConfig, path: &str) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_markdown_content(&mut writer, snapshot, config)
}

pub fn print_markdown(snapshot: &StatsSnapshot, config: &LoadConfig) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    write_markdown_content(&mut writer, snapshot, config)
}

fn write_markdown_content<W: Write>(
    writer: &mut W,
    snapshot: &StatsSnapshot,
    config: &LoadConfig,
) -> io::Result<()> {
    writeln!(writer, "# Load Test Results")?;
    writeln!(writer)?;

    // Configuration
    writeln!(writer, "## Configuration")?;
    writeln!(writer)?;
    writeln!(writer, "| Parameter | Value |")?;
    writeln!(writer, "|-----------|-------|")?;
    writeln!(writer, "| URL | `{}` |", config.url)?;
    writeln!(writer, "| Method | {} |", config.method)?;
    writeln!(writer, "| Concurrency | {} |", config.concurrency)?;
    writeln!(writer, "| Duration | {}s |", snapshot.elapsed.as_secs())?;
    if config.rate > 0 {
        writeln!(writer, "| Rate Limit | {} req/s |", config.rate)?;
    }
    writeln!(writer)?;

    // Summary
    writeln!(writer, "## Summary")?;
    writeln!(writer)?;
    writeln!(writer, "| Metric | Value |")?;
    writeln!(writer, "|--------|-------|")?;
    writeln!(writer, "| Total Requests | {} |", snapshot.total_requests)?;
    writeln!(writer, "| Successful | {} |", snapshot.successful)?;
    writeln!(writer, "| Failed | {} |", snapshot.failed)?;
    writeln!(writer, "| Requests/sec | {:.2} |", snapshot.requests_per_sec)?;
    writeln!(writer, "| Error Rate | {:.2}% |", snapshot.error_rate * 100.0)?;
    writeln!(writer)?;

    // Latency
    writeln!(writer, "## Latency")?;
    writeln!(writer)?;
    writeln!(writer, "| Percentile | Latency (ms) |")?;
    writeln!(writer, "|------------|--------------|")?;
    writeln!(writer, "| Min | {:.2} |", snapshot.latency_min_us as f64 / 1000.0)?;
    writeln!(writer, "| p50 | {:.2} |", snapshot.latency_p50_us as f64 / 1000.0)?;
    writeln!(writer, "| p90 | {:.2} |", snapshot.latency_p90_us as f64 / 1000.0)?;
    writeln!(writer, "| p95 | {:.2} |", snapshot.latency_p95_us as f64 / 1000.0)?;
    writeln!(writer, "| p99 | {:.2} |", snapshot.latency_p99_us as f64 / 1000.0)?;
    writeln!(writer, "| p99.9 | {:.2} |", snapshot.latency_p999_us as f64 / 1000.0)?;
    writeln!(writer, "| Max | {:.2} |", snapshot.latency_max_us as f64 / 1000.0)?;
    writeln!(writer)?;

    // Status Codes
    if !snapshot.status_codes.is_empty() {
        writeln!(writer, "## Status Codes")?;
        writeln!(writer)?;
        writeln!(writer, "| Code | Count |")?;
        writeln!(writer, "|------|-------|")?;
        let mut codes: Vec<_> = snapshot.status_codes.iter().collect();
        codes.sort_by_key(|(code, _)| *code);
        for (code, count) in codes {
            writeln!(writer, "| {} | {} |", code, count)?;
        }
        writeln!(writer)?;
    }

    // Errors
    if !snapshot.errors.is_empty() {
        writeln!(writer, "## Errors")?;
        writeln!(writer)?;
        writeln!(writer, "| Type | Count |")?;
        writeln!(writer, "|------|-------|")?;
        for (kind, count) in &snapshot.errors {
            writeln!(writer, "| {} | {} |", kind.as_str(), count)?;
        }
        writeln!(writer)?;
    }

    writer.flush()
}
