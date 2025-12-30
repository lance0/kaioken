mod cli;
mod compare;
mod config;
mod engine;
#[cfg(feature = "grpc")]
mod grpc;
mod http;
#[cfg(feature = "http3")]
mod http3;
mod import;
mod output;
mod tui;
mod types;
mod ws;

use clap::Parser;
use cli::{Cli, Commands, RunArgs};
use compare::{compare_results, print_comparison};
use config::{load_config, merge_config};
use engine::{Engine, evaluate_thresholds, print_threshold_results};
use output::{
    print_csv, print_html, print_json, print_markdown, write_csv, write_html, write_json,
    write_markdown,
};
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
        Commands::Import(args) => {
            import::run_import(&args)?;
            Ok(0)
        }
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

    let url = args
        .url
        .as_deref()
        .unwrap_or("https://api.example.com/health");

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

    fs::write(&args.output, config).map_err(|e| format!("Failed to write config file: {}", e))?;

    eprintln!("Created config file: {}", args.output.display());
    eprintln!("\nRun with: kaioken run -f {}", args.output.display());

    Ok(0)
}

fn run_compare(args: &cli::CompareArgs) -> Result<i32, String> {
    let result = match compare_results(args) {
        Ok(r) => r,
        Err(e) if e.contains("Cannot compare") && e.contains("vs") => {
            eprintln!("Error: {}", e);
            return Ok(5); // Exit code 5 for load model mismatch
        }
        Err(e) => return Err(e),
    };

    if args.json {
        compare::display::print_comparison_json(&result)?;
    } else {
        // Use serious mode if explicitly requested OR if not a TTY (CI environment)
        let serious = args.serious || !std::io::IsTerminal::is_terminal(&std::io::stdout());
        print_comparison(&result, serious);
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

    // Debug mode - send single request and exit
    if args.debug {
        return run_debug_request(&config).await;
    }

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
        // Show load model info
        if config.arrival_rate.is_some() || config.stages.iter().any(|s| s.target_rate.is_some()) {
            eprintln!("Load Model:  Open (arrival rate)");
            if let Some(rate) = config.arrival_rate {
                eprintln!("Target RPS:  {}", rate);
            }
            eprintln!("Max VUs:     {}", config.max_vus.unwrap_or(100));
        } else {
            eprintln!("Load Model:  Closed (VU-driven)");
            eprintln!("Concurrency: {}", config.concurrency);
        }
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
            let max_target = config
                .stages
                .iter()
                .filter_map(|s| s.target)
                .max()
                .unwrap_or(0);
            let max_rate = config.stages.iter().filter_map(|s| s.target_rate).max();
            if let Some(rate) = max_rate {
                eprintln!(
                    "Stages:      {} defined (total: {:?}, max rate: {} RPS)",
                    config.stages.len(),
                    total,
                    rate
                );
            } else {
                eprintln!(
                    "Stages:      {} defined (total: {:?}, max workers: {})",
                    config.stages.len(),
                    total,
                    max_target
                );
            }
            for (i, s) in config.stages.iter().enumerate() {
                if let Some(target) = s.target {
                    eprintln!("  {}. {:?} -> {} workers", i + 1, s.duration, target);
                } else if let Some(rate) = s.target_rate {
                    eprintln!("  {}. {:?} -> {} RPS", i + 1, s.duration, rate);
                }
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
    let check_stats_ref = engine.check_stats_ref();

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

    let mut final_snapshot = snapshot_rx.borrow().clone();

    // Merge check stats into snapshot for threshold evaluation
    let check_stats = check_stats_ref.lock().unwrap().clone();
    if !check_stats.is_empty() {
        let (total_passed, total_checks): (u64, u64) = check_stats
            .values()
            .fold((0, 0), |(p, t), (passed, total)| (p + passed, t + total));
        if total_checks > 0 {
            final_snapshot.overall_check_pass_rate =
                Some(total_passed as f64 / total_checks as f64);
        }
    }

    // Evaluate thresholds
    let threshold_results = evaluate_thresholds(&config.thresholds, &final_snapshot);
    let thresholds_passed = threshold_results.iter().all(|r| r.passed);
    let threshold_results_opt = if threshold_results.is_empty() {
        None
    } else {
        Some(threshold_results.as_slice())
    };

    // Prepare check_stats option for JSON output
    let check_stats_opt = if check_stats.is_empty() {
        None
    } else {
        Some(&check_stats)
    };

    // Print output to stdout if in headless mode
    if output_json {
        print_json(
            &final_snapshot,
            &config,
            threshold_results_opt,
            check_stats_opt,
        )
        .map_err(|e| format!("Failed to write JSON: {}", e))?;
    } else if !use_tui {
        match format.as_str() {
            "csv" => print_csv(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write CSV: {}", e))?,
            "md" | "markdown" => print_markdown(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write Markdown: {}", e))?,
            "html" => print_html(&final_snapshot, &config)
                .map_err(|e| format!("Failed to write HTML: {}", e))?,
            "json" => print_json(
                &final_snapshot,
                &config,
                threshold_results_opt,
                check_stats_opt,
            )
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
            _ => write_json(
                &final_snapshot,
                &config,
                path,
                threshold_results_opt,
                check_stats_opt,
            ),
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

    // Print check results (check_stats already obtained above)
    if !check_stats.is_empty() && !use_tui && !output_json && format != "json" {
        print_check_results(&check_stats);
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

async fn run_debug_request(config: &types::LoadConfig) -> Result<i32, String> {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use std::time::Instant;

    let separator = "=".repeat(80);

    println!("{}", separator);
    println!("{:^80}", "DEBUG: Single Request");
    println!("{}", separator);

    // Determine URL and method (use first scenario if available)
    let (url, method, body, headers) = if !config.scenarios.is_empty() {
        let s = &config.scenarios[0];
        (
            s.url.clone(),
            s.method.clone(),
            s.body.clone(),
            s.headers.clone(),
        )
    } else {
        (
            config.url.clone(),
            config.method.clone(),
            config.body.clone(),
            config.headers.clone(),
        )
    };

    // Print request details
    println!("\nRequest:");
    println!("  {} {}", method, url);

    if !headers.is_empty() {
        println!("  Headers:");
        for (k, v) in &headers {
            // Mask sensitive headers
            let display_value = if k.to_lowercase() == "authorization" {
                if v.len() > 15 {
                    format!("{}***", &v[..12])
                } else {
                    "***".to_string()
                }
            } else {
                v.clone()
            };
            println!("    {}: {}", k, display_value);
        }
    }

    if let Some(ref b) = body {
        println!("  Body:");
        // Pretty print if JSON, otherwise show as-is
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(b) {
            if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                for line in pretty.lines() {
                    println!("    {}", line);
                }
            } else {
                println!("    {}", b);
            }
        } else {
            // Truncate if too long
            if b.len() > 500 {
                println!("    {}... ({} bytes total)", &b[..500], b.len());
            } else {
                println!("    {}", b);
            }
        }
    }

    // Build client
    let client = http::create_client(
        1,
        config.timeout,
        config.connect_timeout,
        config.insecure,
        config.http2,
        config.cookie_jar,
        config.follow_redirects,
        config.disable_keepalive,
        config.proxy.as_deref(),
        config.client_cert.as_deref(),
        config.client_key.as_deref(),
        config.ca_cert.as_deref(),
        config.connect_to.as_ref().map(|(h, a)| (h.as_str(), *a)),
    )
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build request
    let mut request = client.request(method.clone(), &url);

    // Add headers
    let mut header_map = HeaderMap::new();
    for (k, v) in &headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::try_from(k.as_str()),
            HeaderValue::from_str(v.as_str()),
        ) {
            header_map.insert(name, value);
        }
    }
    request = request.headers(header_map);

    // Add body
    if let Some(ref b) = body {
        request = request.body(b.clone());
    }

    // Send request and measure time
    println!("\nSending request...");
    let start = Instant::now();
    let result = request.send().await;
    let latency = start.elapsed();

    println!("\n{}", "-".repeat(80));
    println!("Response:");

    match result {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();
            let content_length = headers
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            println!(
                "  Status: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("")
            );
            println!("  Latency: {:.2}ms", latency.as_secs_f64() * 1000.0);

            if let Some(len) = content_length {
                println!("  Content-Length: {} bytes", len);
            }

            // Print response headers
            if !headers.is_empty() {
                println!("  Headers:");
                for (name, value) in headers.iter() {
                    if let Ok(v) = value.to_str() {
                        println!("    {}: {}", name.as_str(), v);
                    }
                }
            }

            // Get body
            match response.text().await {
                Ok(body_text) => {
                    if !body_text.is_empty() {
                        println!("\nBody:");
                        // Try to pretty-print JSON
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                            if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                                for line in pretty.lines().take(100) {
                                    println!("  {}", line);
                                }
                                let line_count = pretty.lines().count();
                                if line_count > 100 {
                                    println!("  ... ({} more lines)", line_count - 100);
                                }
                            } else {
                                print_body_text(&body_text);
                            }
                        } else {
                            print_body_text(&body_text);
                        }
                    }
                }
                Err(e) => {
                    println!("\n  (Failed to read body: {})", e);
                }
            }

            println!("\n{}", separator);

            if status.is_success() {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        Err(e) => {
            println!("  Error: {}", e);

            // Provide helpful suggestions based on error type
            if e.is_timeout() {
                println!("\n  Suggestion: Request timed out. Try increasing --timeout");
            } else if e.is_connect() {
                println!(
                    "\n  Suggestion: Connection failed. Check if the server is running and accessible"
                );
            } else if e.is_builder() {
                println!("\n  Suggestion: Invalid request configuration. Check URL and headers");
            }

            println!("\n{}", separator);
            Ok(1)
        }
    }
}

fn print_body_text(body: &str) {
    const MAX_BODY_LINES: usize = 50;
    const MAX_LINE_LEN: usize = 200;

    let lines: Vec<&str> = body.lines().collect();
    for line in lines.iter().take(MAX_BODY_LINES) {
        if line.len() > MAX_LINE_LEN {
            println!("  {}...", &line[..MAX_LINE_LEN]);
        } else {
            println!("  {}", line);
        }
    }
    if lines.len() > MAX_BODY_LINES {
        println!("  ... ({} more lines)", lines.len() - MAX_BODY_LINES);
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
    let title = if serious {
        "Load Test Results"
    } else {
        "KAIOKEN RESULTS"
    };

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
    println!(
        "  Min:             {:>12.2}",
        snapshot.latency_min_us as f64 / 1000.0
    );
    println!(
        "  Max:             {:>12.2}",
        snapshot.latency_max_us as f64 / 1000.0
    );
    println!(
        "  Mean:            {:>12.2}",
        snapshot.latency_mean_us / 1000.0
    );
    println!(
        "  p50:             {:>12.2}",
        snapshot.latency_p50_us as f64 / 1000.0
    );
    println!(
        "  p90:             {:>12.2}",
        snapshot.latency_p90_us as f64 / 1000.0
    );
    println!(
        "  p95:             {:>12.2}",
        snapshot.latency_p95_us as f64 / 1000.0
    );
    println!(
        "  p99:             {:>12.2}",
        snapshot.latency_p99_us as f64 / 1000.0
    );
    println!(
        "  p99.9:           {:>12.2}",
        snapshot.latency_p999_us as f64 / 1000.0
    );

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
            let suggestion = kind.suggestion();
            if suggestion.is_empty() {
                println!("  {:15} {:>12}", format!("{}:", kind.as_str()), count);
            } else {
                println!(
                    "  {:15} {:>12}  ({})",
                    format!("{}:", kind.as_str()),
                    count,
                    suggestion
                );
            }
        }
    }

    println!("\n{}", "=".repeat(50));
}

fn print_check_results(check_stats: &std::collections::HashMap<String, (u64, u64)>) {
    println!("\n{}", "=".repeat(60));
    println!("CHECKS");
    println!("{}", "=".repeat(60));

    let mut checks: Vec<_> = check_stats.iter().collect();
    checks.sort_by_key(|(name, _)| name.as_str());

    for (name, (passed, total)) in checks {
        let rate = if *total > 0 {
            (*passed as f64 / *total as f64) * 100.0
        } else {
            0.0
        };
        let status = if rate >= 100.0 {
            "\x1b[32m✓\x1b[0m"
        } else if rate >= 90.0 {
            "\x1b[33m⚠\x1b[0m"
        } else {
            "\x1b[31m✗\x1b[0m"
        };
        println!(
            "  {} {} - {}/{} ({:.1}%)",
            status, name, passed, total, rate
        );
    }
}
