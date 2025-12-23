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
use engine::{evaluate_thresholds, print_threshold_results, Engine};
use output::{print_csv, print_html, print_json, print_markdown, write_csv, write_html, write_json, write_markdown};
use std::io::{self, Write};
use std::sync::atomic::Ordering;
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
        Commands::Init(args) => run_init(&args),
        Commands::Completions(args) => {
            cli::generate_completions(args.shell);
            Ok(0)
        }
        Commands::Man => {
            cli::generate_man_page().map_err(|e| format!("Failed to generate man page: {}", e))?;
            Ok(0)
        }
    }
}

fn run_init(args: &cli::InitArgs) -> Result<i32, String> {
    use std::fs;

    if args.output.exists() && !args.force {
        return Err(format!(
            "File '{}' already exists. Use --force to overwrite.",
            args.output.display()
        ));
    }

    let url = args.url.as_deref().unwrap_or("https://api.example.com/health");

    let config = format!(
        r#"# Kaioken Load Test Configuration
# https://github.com/lance0/kaioken

[target]
url = "{url}"
method = "GET"
timeout = "5s"
connect_timeout = "2s"
# http2 = false
# insecure = false

# Headers (uncomment and modify as needed)
# [target.headers]
# Authorization = "Bearer ${{API_TOKEN}}"
# Content-Type = "application/json"

# Request body (for POST/PUT/PATCH)
# body = '{{"key": "value"}}'
# body_file = "payload.json"

[load]
concurrency = 50
duration = "30s"
# max_requests = 0      # 0 = unlimited
# rate = 0              # requests/sec, 0 = unlimited
# ramp_up = "0s"        # time to reach full concurrency
# warmup = "0s"         # warmup period (not measured)

# Variable interpolation available in URL, headers, and body:
#   ${{REQUEST_ID}}    - unique ID per request
#   ${{TIMESTAMP_MS}}  - current epoch time in milliseconds

# Weighted scenarios (optional) - when defined, these override [target]
# Traffic is distributed based on weight (e.g., 7:2:1 ratio below)
#
# [[scenarios]]
# name = "get_users"
# url = "https://api.example.com/users"
# method = "GET"
# weight = 7
#
# [[scenarios]]
# name = "create_user"
# url = "https://api.example.com/users"
# method = "POST"
# body = '{{"name": "test"}}'
# weight = 2
#
# [[scenarios]]
# name = "health_check"
# url = "https://api.example.com/health"
# method = "GET"
# weight = 1
"#,
        url = url
    );

    fs::write(&args.output, config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    eprintln!("Created config file: {}", args.output.display());
    eprintln!("\nRun with: kaioken run -f {}", args.output.display());

    Ok(0)
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

    // Dry run - validate and exit
    if args.dry_run {
        eprintln!("Configuration validated successfully!\n");
        if config.scenarios.is_empty() {
            eprintln!("Target:      {}", config.url);
            eprintln!("Method:      {}", config.method);
        } else {
            eprintln!("Scenarios:   {} defined", config.scenarios.len());
            let total_weight: u32 = config.scenarios.iter().map(|s| s.weight).sum();
            for s in &config.scenarios {
                let pct = (s.weight as f64 / total_weight as f64) * 100.0;
                eprintln!(
                    "  - {} ({} {}) weight={} ({:.0}%)",
                    s.name, s.method, s.url, s.weight, pct
                );
            }
        }
        eprintln!("Concurrency: {}", config.concurrency);
        eprintln!("Duration:    {:?}", config.duration);
        if config.max_requests > 0 {
            eprintln!("Max Reqs:    {}", config.max_requests);
        }
        if config.rate > 0 {
            eprintln!("Rate Limit:  {} req/s", config.rate);
        }
        if !config.ramp_up.is_zero() {
            eprintln!("Ramp Up:     {:?}", config.ramp_up);
        }
        if !config.warmup.is_zero() {
            eprintln!("Warmup:      {:?}", config.warmup);
        }
        if let Some(think_time) = config.think_time {
            eprintln!("Think time:  {:?}", think_time);
        }
        if config.http2 {
            eprintln!("HTTP/2:      enabled");
        }
        if !config.headers.is_empty() {
            eprintln!("Headers:     {} custom", config.headers.len());
        }
        if config.body.is_some() {
            eprintln!("Body:        present");
        }
        if !config.thresholds.is_empty() {
            eprintln!("Thresholds:  {} defined", config.thresholds.len());
            for t in &config.thresholds {
                eprintln!(
                    "  - {} {} {}",
                    t.metric.as_str(),
                    t.operator.as_str(),
                    t.value
                );
            }
        }
        if !config.checks.is_empty() {
            eprintln!("Checks:      {} defined", config.checks.len());
            for c in &config.checks {
                eprintln!("  - {}", c.name);
            }
        }
        if !config.stages.is_empty() {
            let total: std::time::Duration = config.stages.iter().map(|s| s.duration).sum();
            let max_target = config.stages.iter().map(|s| s.target).max().unwrap_or(0);
            eprintln!("Stages:      {} defined (total: {:?}, max workers: {})", 
                config.stages.len(), total, max_target);
            for (i, s) in config.stages.iter().enumerate() {
                eprintln!("  {}. {:?} -> {} workers", i + 1, s.duration, s.target);
            }
        }
        return Ok(0);
    }

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
    let fail_fast_flag = engine.threshold_failed_flag();

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

    // Evaluate thresholds
    let threshold_results = evaluate_thresholds(&config.thresholds, &final_snapshot);
    let thresholds_passed = threshold_results.iter().all(|r| r.passed);
    let threshold_results_opt = if threshold_results.is_empty() {
        None
    } else {
        Some(threshold_results.as_slice())
    };

    // Print output to stdout if in headless mode
    if output_json {
        print_json(&final_snapshot, &config, threshold_results_opt)
            .map_err(|e| format!("Failed to write JSON: {}", e))?;
    } else if !use_tui {
        match format.as_str() {
            "csv" => print_csv(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write CSV: {}", e))?,
            "md" | "markdown" => print_markdown(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write Markdown: {}", e))?,
            "html" => print_html(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write HTML: {}", e))?,
            "json" => print_json(&final_snapshot, &config, threshold_results_opt)
                .map_err(|e| format!("Failed to write JSON: {}", e))?,
            _ => print_summary(&final_snapshot, args.serious),
        }
    }

    // Write to file if specified
    if let Some(path) = &args.output {
        let write_result = match format.as_str() {
            "csv" => write_csv(&final_snapshot, &config, path),
            "md" | "markdown" => write_markdown(&final_snapshot, &config, path),
            "html" => write_html(&final_snapshot, &config, path),
            _ => write_json(&final_snapshot, &config, path, threshold_results_opt),
        };
        write_result.map_err(|e| format!("Failed to write output file: {}", e))?;

        if !args.quiet && !use_tui {
            eprintln!("Results written to: {}", path);
        }
    }

    // Print threshold results to console (for non-JSON formats)
    if !threshold_results.is_empty() && !use_tui && !output_json && format != "json" {
        print_threshold_results(&threshold_results);
    }

    // Determine exit code
    let fail_fast_triggered = fail_fast_flag.load(Ordering::Relaxed);
    if !thresholds_passed || fail_fast_triggered {
        Ok(4) // Thresholds failed
    } else if stats.failed > 0 && stats.error_rate() > 0.5 {
        Ok(1) // High error rate
    } else {
        Ok(0) // Success
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
