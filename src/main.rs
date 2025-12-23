mod cli;
mod engine;
mod http;
mod output;
mod tui;
mod types;

use clap::Parser;
use cli::Cli;
use engine::Engine;
use output::{print_json, write_json};
use std::io::{self, Write};
use tui::App;
use types::LoadConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let cli = Cli::parse();

    let headers = cli.parse_headers()?;
    let method = cli.parse_method()?;

    if !cli.yes && !cli.is_localhost() && !cli.quiet && !cli.no_tui && !cli.json {
        eprintln!(
            "\n⚠️  WARNING: Target is remote ({})",
            extract_host(&cli.url).unwrap_or(&cli.url)
        );
        eprintln!("    High concurrency may impact production systems.");
        eprint!("    Press Enter to continue or Ctrl+C to abort... ");
        io::stderr().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;
    }

    let config = LoadConfig {
        url: cli.url.clone(),
        method,
        headers,
        body: cli.body.clone(),
        concurrency: cli.concurrency,
        duration: cli.duration,
        timeout: cli.timeout,
        connect_timeout: cli.connect_timeout,
        insecure: cli.insecure,
    };

    let engine = Engine::new(config.clone());
    let cancel_token = engine.cancel_token();
    let snapshot_rx = engine.snapshot_rx();
    let state_rx = engine.state_rx();

    let use_tui = !cli.no_tui && !cli.json;
    let output_json = cli.json;

    let tui_handle = if use_tui {
        let app = App::new(
            config.clone(),
            snapshot_rx.clone(),
            state_rx.clone(),
            cancel_token.clone(),
            cli.serious,
            cli.output.clone(),
        );

        Some(tokio::spawn(async move { app.run().await }))
    } else {
        None
    };

    let ctrl_c_token = cancel_token.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            ctrl_c_token.cancel();
        }
    });

    let stats = engine.run().await?;

    if let Some(handle) = tui_handle {
        let _ = handle.await;
    }

    let final_snapshot = snapshot_rx.borrow().clone();

    if output_json {
        print_json(&final_snapshot, &config).map_err(|e| format!("Failed to write JSON: {}", e))?;
    } else if !use_tui {
        print_summary(&final_snapshot, cli.serious);
    }

    if let Some(path) = &cli.output {
        if !use_tui || cli.json {
            write_json(&final_snapshot, &config, path)
                .map_err(|e| format!("Failed to write output file: {}", e))?;
        }
    }

    if stats.failed > 0 && stats.error_rate() > 0.5 {
        std::process::exit(1);
    }

    Ok(())
}

fn extract_host(url: &str) -> Option<&str> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    without_scheme.split('/').next()
}

fn print_summary(snapshot: &types::StatsSnapshot, serious: bool) {
    let title = if serious { "Load Test Results" } else { "KAIOKEN RESULTS" };

    println!("\n{}", "=".repeat(50));
    println!("{:^50}", title);
    println!("{}", "=".repeat(50));

    println!("\nThroughput:");
    println!("  Total Requests:  {:>12}", snapshot.total_requests);
    println!("  Successful:      {:>12}", snapshot.successful);
    println!("  Failed:          {:>12}", snapshot.failed);
    println!("  Requests/sec:    {:>12.2}", snapshot.requests_per_sec);
    println!("  Error Rate:      {:>11.2}%", snapshot.error_rate * 100.0);

    println!("\nLatency (ms):");
    println!("  Min:             {:>12.2}", snapshot.latency_min_us as f64 / 1000.0);
    println!("  Max:             {:>12.2}", snapshot.latency_max_us as f64 / 1000.0);
    println!("  Mean:            {:>12.2}", snapshot.latency_mean_us / 1000.0);
    println!("  p50:             {:>12.2}", snapshot.latency_p50_us as f64 / 1000.0);
    println!("  p90:             {:>12.2}", snapshot.latency_p90_us as f64 / 1000.0);
    println!("  p95:             {:>12.2}", snapshot.latency_p95_us as f64 / 1000.0);
    println!("  p99:             {:>12.2}", snapshot.latency_p99_us as f64 / 1000.0);
    println!("  p99.9:           {:>12.2}", snapshot.latency_p999_us as f64 / 1000.0);

    if !snapshot.status_codes.is_empty() {
        println!("\nStatus Codes:");
        let mut codes: Vec<_> = snapshot.status_codes.iter().collect();
        codes.sort_by_key(|(code, _)| *code);
        for (code, count) in codes {
            println!("  {}:              {:>12}", code, count);
        }
    }

    if !snapshot.errors.is_empty() {
        println!("\nErrors:");
        for (kind, count) in &snapshot.errors {
            println!("  {:15} {:>12}", format!("{}:", kind.as_str()), count);
        }
    }

    println!("\n{}", "=".repeat(50));
}
