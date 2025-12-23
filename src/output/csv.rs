use crate::types::{LoadConfig, StatsSnapshot};
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub fn write_csv(snapshot: &StatsSnapshot, config: &LoadConfig, path: &str) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_csv_content(&mut writer, snapshot, config)
}

pub fn print_csv(snapshot: &StatsSnapshot, config: &LoadConfig) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    write_csv_content(&mut writer, snapshot, config)
}

fn write_csv_content<W: Write>(
    writer: &mut W,
    snapshot: &StatsSnapshot,
    config: &LoadConfig,
) -> io::Result<()> {
    // Header
    writeln!(
        writer,
        "metric,value"
    )?;

    // Metadata
    writeln!(writer, "url,\"{}\"", config.url)?;
    writeln!(writer, "method,{}", config.method)?;
    writeln!(writer, "concurrency,{}", config.concurrency)?;
    writeln!(writer, "duration_secs,{}", snapshot.elapsed.as_secs())?;

    // Summary
    writeln!(writer, "total_requests,{}", snapshot.total_requests)?;
    writeln!(writer, "successful,{}", snapshot.successful)?;
    writeln!(writer, "failed,{}", snapshot.failed)?;
    writeln!(writer, "requests_per_sec,{:.2}", snapshot.requests_per_sec)?;
    writeln!(writer, "error_rate,{:.6}", snapshot.error_rate)?;

    // Latency (ms)
    writeln!(writer, "latency_min_ms,{:.2}", snapshot.latency_min_us as f64 / 1000.0)?;
    writeln!(writer, "latency_max_ms,{:.2}", snapshot.latency_max_us as f64 / 1000.0)?;
    writeln!(writer, "latency_mean_ms,{:.2}", snapshot.latency_mean_us / 1000.0)?;
    writeln!(writer, "latency_p50_ms,{:.2}", snapshot.latency_p50_us as f64 / 1000.0)?;
    writeln!(writer, "latency_p90_ms,{:.2}", snapshot.latency_p90_us as f64 / 1000.0)?;
    writeln!(writer, "latency_p95_ms,{:.2}", snapshot.latency_p95_us as f64 / 1000.0)?;
    writeln!(writer, "latency_p99_ms,{:.2}", snapshot.latency_p99_us as f64 / 1000.0)?;
    writeln!(writer, "latency_p999_ms,{:.2}", snapshot.latency_p999_us as f64 / 1000.0)?;

    // Status codes
    let mut codes: Vec<_> = snapshot.status_codes.iter().collect();
    codes.sort_by_key(|(code, _)| *code);
    for (code, count) in codes {
        writeln!(writer, "status_{},{}", code, count)?;
    }

    // Errors
    for (kind, count) in &snapshot.errors {
        writeln!(writer, "error_{},{}", kind.as_str(), count)?;
    }

    writer.flush()
}
