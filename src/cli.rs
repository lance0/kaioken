use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;
use std::time::Duration;

fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}

#[derive(Parser, Debug)]
#[command(
    name = "kaioken",
    author,
    version,
    about = "A Rust-based HTTP load testing tool with real-time terminal UI and DBZ flavor",
    long_about = "kaioken runs controlled HTTP load tests with real-time TUI visualization.\n\n\
                  Power up your API testing - DBZ style!"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a load test (default if no subcommand specified)
    #[command(name = "run")]
    Run(RunArgs),

    /// Compare two load test results for regressions
    Compare(CompareArgs),

    /// Generate a starter config file
    Init(InitArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Run(RunArgs::default())
    }
}

#[derive(Parser, Debug, Default)]
pub struct RunArgs {
    /// Target URL to load test
    #[arg(required_unless_present = "config")]
    pub url: Option<String>,

    /// Number of concurrent workers
    #[arg(short = 'c', long, default_value = "50")]
    pub concurrency: u32,

    /// Test duration (e.g., 10s, 1m, 30s)
    #[arg(short = 'd', long, default_value = "10s", value_parser = parse_duration)]
    pub duration: Duration,

    /// Max requests per second (0 = unlimited)
    #[arg(short = 'r', long, default_value = "0")]
    pub rate: u32,

    /// Ramp-up time to reach full concurrency (e.g., 5s)
    #[arg(long, default_value = "0s", value_parser = parse_duration)]
    pub ramp_up: Duration,

    /// Warmup period before measuring (e.g., 3s)
    #[arg(long, default_value = "0s", value_parser = parse_duration)]
    pub warmup: Duration,

    /// Request timeout (e.g., 5s)
    #[arg(long, default_value = "5s", value_parser = parse_duration)]
    pub timeout: Duration,

    /// Connection timeout (e.g., 2s)
    #[arg(long, default_value = "2s", value_parser = parse_duration)]
    pub connect_timeout: Duration,

    /// HTTP method
    #[arg(short = 'm', long, default_value = "GET")]
    pub method: String,

    /// HTTP headers (can be specified multiple times)
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub headers: Vec<String>,

    /// Request body
    #[arg(short = 'b', long)]
    pub body: Option<String>,

    /// Request body from file
    #[arg(long, value_name = "FILE")]
    pub body_file: Option<PathBuf>,

    /// Max requests to send (0 = unlimited, use duration)
    #[arg(short = 'n', long, default_value = "0")]
    pub max_requests: u64,

    /// Use HTTP/2 (default: HTTP/1.1)
    #[arg(long)]
    pub http2: bool,

    /// Config file path (TOML)
    #[arg(short = 'f', long = "config")]
    pub config: Option<PathBuf>,

    /// Output file path for results
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Output format (json, csv, md)
    #[arg(long, default_value = "json")]
    pub format: String,

    /// Disable TUI, print summary only
    #[arg(long)]
    pub no_tui: bool,

    /// Shorthand for --no-tui --format json (outputs JSON to stdout)
    #[arg(long)]
    pub json: bool,

    /// Suppress non-essential output (for CI)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Disable DBZ flavor (serious mode)
    #[arg(long)]
    pub serious: bool,

    /// Skip TLS certificate verification
    #[arg(long)]
    pub insecure: bool,

    /// Skip confirmation for remote targets
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Validate config and exit without running
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct CompareArgs {
    /// Baseline results file (JSON)
    pub baseline: PathBuf,

    /// Current results file (JSON) to compare against baseline
    pub current: PathBuf,

    /// p99 latency regression threshold (percentage, default: 10)
    #[arg(long, default_value = "10.0")]
    pub threshold_p99: f64,

    /// p999 latency regression threshold (percentage, default: 15)
    #[arg(long, default_value = "15.0")]
    pub threshold_p999: f64,

    /// Error rate regression threshold (percentage, default: 50)
    #[arg(long, default_value = "50.0")]
    pub threshold_error_rate: f64,

    /// RPS regression threshold (percentage, default: 10)
    #[arg(long, default_value = "10.0")]
    pub threshold_rps: f64,

    /// Disable DBZ flavor (serious mode)
    #[arg(long)]
    pub serious: bool,

    /// Output as JSON instead of table
    #[arg(long)]
    pub json: bool,
}

impl RunArgs {
    pub fn parse_headers(&self) -> Result<Vec<(String, String)>, String> {
        self.headers
            .iter()
            .map(|h| {
                let parts: Vec<&str> = h.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid header format: {}. Expected 'Name: Value'", h));
                }
                Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
            })
            .collect()
    }
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output file path (default: kaioken.toml)
    #[arg(short, long, default_value = "kaioken.toml")]
    pub output: PathBuf,

    /// Target URL to include in config
    #[arg(short, long)]
    pub url: Option<String>,

    /// Overwrite existing file
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "kaioken", &mut std::io::stdout());
}
