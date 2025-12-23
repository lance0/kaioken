mod cli;
mod compare;
mod config;
mod engine;
mod http;
mod output;
mod tui;
mod types;

use clap::Parser;
use cli::{Cli, Commands, RunArgs};
use compare::{compare_results, print_comparison};
use config::{load_config, merge_config};
use engine::Engine;
use output::{print_csv, print_json, print_markdown, write_csv, write_json, write_markdown};
use std::io::{self, Write};
use tui::App;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let exit_code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

async fn run() -> Result<i32, String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(args) => run_load_test(&args).await,
        Commands::Compare(args) => run_compare(&args),
    }
}

fn run_compare(args: &cli::CompareArgs) -> Result<i32, String> {
    let result = compare_results(args)?;

    if args.json {
        compare::display::print_comparison_json(&result)?;
    } else {
        print_comparison(&result, args.serious);
    }

    if result.has_regressions {
        Ok(3) // Exit code 3 for regressions
    } else {
        Ok(0)
    }
}

async fn run_load_test(args: &RunArgs) -> Result<i32, String> {
    // Load TOML config if specified
    let toml_config = if let Some(ref path) = args.config {
        Some(load_config(path)?)
    } else {
        None
    };

    // Merge CLI args with config file
    let config = merge_config(args, toml_config)?;

    // Safety warning for remote targets
    if !args.yes && !is_localhost(&config.url) && !args.quiet && !args.no_tui && !args.json {
        eprintln!(
            "\n⚠️  WARNING: Target is remote ({})",
            extract_host(&config.url).unwrap_or(&config.url)
        );
        eprintln!("    High concurrency may impact production systems.");
        eprint!("    Press Enter to continue or Ctrl+C to abort... ");
        io::stderr().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;
    }

    let engine = Engine::new(config.clone());
    let cancel_token = engine.cancel_token();
    let snapshot_rx = engine.snapshot_rx();
    let state_rx = engine.state_rx();
    let phase_rx = engine.phase_rx();

    let use_tui = !args.no_tui && !args.json;
    let output_json = args.json;
    let format = args.format.to_lowercase();

    let tui_handle = if use_tui {
        let app = App::new(
            config.clone(),
            snapshot_rx.clone(),
            state_rx.clone(),
            phase_rx,
            cancel_token.clone(),
            args.serious,
            args.output.clone(),
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

    // Print output to stdout if in headless mode
    if output_json {
        print_json(&final_snapshot, &config).map_err(|e| format!("Failed to write JSON: {}", e))?;
    } else if !use_tui {
        match format.as_str() {
            "csv" => print_csv(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write CSV: {}", e))?,
            "md" | "markdown" => print_markdown(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write Markdown: {}", e))?,
            "json" => print_json(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write JSON: {}", e))?,
            _ => print_summary(&final_snapshot, args.serious),
        }
    }

    // Write to file if specified
    if let Some(path) = &args.output {
        let write_result = match format.as_str() {
            "csv" => write_csv(&final_snapshot, &config, path),
            "md" | "markdown" => write_markdown(&final_snapshot, &config, path),
            _ => write_json(&final_snapshot, &config, path),
        };
        write_result.map_err(|e| format!("Failed to write output file: {}", e))?;

        if !args.quiet && !use_tui {
            eprintln!("Results written to: {}", path);
        }
    }

    if stats.failed > 0 && stats.error_rate() > 0.5 {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn is_localhost(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    url_lower.contains("localhost")
        || url_lower.contains("127.0.0.1")
        || url_lower.contains("[::1]")
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
