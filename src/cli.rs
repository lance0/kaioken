use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;
use std::time::Duration;

fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}

fn parse_number_with_suffix(s: &str) -> Result<u64, String> {
    let s = s.trim().to_lowercase();

    if s.is_empty() {
        return Err("empty value".to_string());
    }

    let (num_str, multiplier) = if let Some(prefix) = s.strip_suffix('k') {
        (prefix, 1_000u64)
    } else if let Some(prefix) = s.strip_suffix('m') {
        (prefix, 1_000_000u64)
    } else {
        (s.as_str(), 1u64)
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;

    num.checked_mul(multiplier)
        .ok_or_else(|| format!("'{}' overflows u64", s))
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
    Run(Box<RunArgs>),

    /// Compare two load test results for regressions
    Compare(CompareArgs),

    /// Generate a starter config file
    Init(InitArgs),

    /// Import scenarios from external formats (HAR, Postman, OpenAPI)
    Import(ImportArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Generate man page
    Man,
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Run(Box::default())
    }
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Target URL to load test
    #[arg(required_unless_present_any = ["config", "rand_regex_url", "urls_from_file"])]
    pub url: Option<String>,

    /// Generate random URLs from regex pattern (e.g., 'https://api.example.com/users/[a-z]{3}[0-9]{2}')
    #[arg(long, value_name = "PATTERN", conflicts_with = "urls_from_file")]
    pub rand_regex_url: Option<String>,

    /// Read URLs from file (one per line, round-robin)
    #[arg(long, value_name = "FILE", conflicts_with = "rand_regex_url")]
    pub urls_from_file: Option<PathBuf>,

    /// Number of concurrent workers
    #[arg(short = 'c', long, default_value = "50")]
    pub concurrency: u32,

    /// Test duration (e.g., 10s, 1m, 30s)
    #[arg(short = 'd', long, default_value = "10s", value_parser = parse_duration)]
    pub duration: Duration,

    /// Max requests per second (0 = unlimited)
    #[arg(short = 'r', long, default_value = "0")]
    pub rate: u32,

    /// Requests per burst (enables burst mode)
    #[arg(long, value_name = "N", requires = "burst_delay", conflicts_with = "arrival_rate")]
    pub burst_rate: Option<u32>,

    /// Delay between bursts (e.g., 1s, 500ms)
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, requires = "burst_rate")]
    pub burst_delay: Option<Duration>,

    /// Ramp-up time to reach full concurrency (e.g., 5s)
    #[arg(long, default_value = "0s", value_parser = parse_duration)]
    pub ramp_up: Duration,

    /// Warmup period before measuring (e.g., 3s)
    #[arg(long, default_value = "0s", value_parser = parse_duration)]
    pub warmup: Duration,

    /// Think time between requests (e.g., 500ms)
    #[arg(long, value_parser = parse_duration)]
    pub think_time: Option<Duration>,

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
    #[arg(long, value_name = "FILE", conflicts_with = "body_lines_file")]
    pub body_file: Option<PathBuf>,

    /// Body lines from file (one per request, round-robin)
    #[arg(short = 'Z', long = "body-lines", value_name = "FILE", conflicts_with_all = ["body", "body_file"])]
    pub body_lines_file: Option<PathBuf>,

    /// Max requests to send (0 = unlimited). Supports k/m suffixes (e.g., 10k, 1m)
    #[arg(short = 'n', long, default_value = "0", value_parser = parse_number_with_suffix)]
    pub max_requests: u64,

    /// Use HTTP/2 (default: HTTP/1.1)
    #[arg(long)]
    pub http2: bool,

    /// Use HTTP/3 (QUIC) - requires --features http3
    #[cfg(feature = "http3")]
    #[arg(long)]
    pub http3: bool,

    /// gRPC service name (e.g., "helloworld.Greeter") - requires --features grpc
    #[cfg(feature = "grpc")]
    #[arg(long)]
    pub grpc_service: Option<String>,

    /// gRPC method name (e.g., "SayHello") - requires --features grpc
    #[cfg(feature = "grpc")]
    #[arg(long)]
    pub grpc_method: Option<String>,

    /// Enable cookie jar for automatic session handling
    #[arg(long)]
    pub cookie_jar: bool,

    /// Target arrival rate in requests/second (enables arrival rate mode)
    #[arg(long)]
    pub arrival_rate: Option<u32>,

    /// Maximum VUs for arrival rate mode (default: 100)
    #[arg(long, default_value = "100")]
    pub max_vus: u32,

    /// Disable latency correction (normally auto-enabled for arrival rate mode)
    #[arg(long)]
    pub no_latency_correction: bool,

    /// Disable following HTTP redirects
    #[arg(long)]
    pub no_follow_redirects: bool,

    /// Config file path (TOML)
    #[arg(short = 'f', long = "config")]
    pub config: Option<PathBuf>,

    /// Output file path for results
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Write summary snapshots to SQLite database
    #[arg(long, value_name = "PATH")]
    pub db_url: Option<PathBuf>,

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

    /// Abort immediately when any threshold fails
    #[arg(long)]
    pub fail_fast: bool,

    /// Send a single request and print full request/response dump
    #[arg(long)]
    pub debug: bool,

    /// Disable HTTP keepalive (new connection per request)
    #[arg(long)]
    pub disable_keepalive: bool,

    // WebSocket options
    /// WebSocket message send interval (e.g., 100ms)
    #[arg(long, default_value = "100ms", value_parser = parse_duration)]
    pub ws_message_interval: Duration,

    /// WebSocket fire-and-forget mode (don't wait for response)
    #[arg(long)]
    pub ws_fire_and_forget: bool,

    // Authentication and security options
    /// Basic authentication credentials (user:password)
    #[arg(short = 'a', long = "basic-auth", value_name = "USER:PASS")]
    pub basic_auth: Option<String>,

    /// HTTP/HTTPS/SOCKS5 proxy URL (e.g., http://proxy:8080, socks5://127.0.0.1:1080)
    #[arg(short = 'x', long)]
    pub proxy: Option<String>,

    /// Client certificate file path (PEM format) for mTLS
    #[arg(long, value_name = "FILE")]
    pub cert: Option<PathBuf>,

    /// Client private key file path (PEM format) for mTLS
    #[arg(long, value_name = "FILE")]
    pub key: Option<PathBuf>,

    /// CA certificate file path (PEM format) for custom root CA
    #[arg(long, value_name = "FILE")]
    pub cacert: Option<PathBuf>,

    /// Override host resolution (HOST:PORT:TARGET_HOST:TARGET_PORT)
    #[arg(long, value_name = "MAPPING")]
    pub connect_to: Option<String>,

    /// Multipart form field (name=value or name=@filepath for files)
    #[arg(short = 'F', long = "form", value_name = "FIELD")]
    pub form: Vec<String>,
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            url: None,
            rand_regex_url: None,
            urls_from_file: None,
            concurrency: 50,
            duration: Duration::from_secs(10),
            rate: 0,
            burst_rate: None,
            burst_delay: None,
            ramp_up: Duration::ZERO,
            warmup: Duration::ZERO,
            think_time: None,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            method: "GET".to_string(),
            headers: Vec::new(),
            body: None,
            body_file: None,
            body_lines_file: None,
            max_requests: 0,
            http2: false,
            #[cfg(feature = "http3")]
            http3: false,
            #[cfg(feature = "grpc")]
            grpc_service: None,
            #[cfg(feature = "grpc")]
            grpc_method: None,
            cookie_jar: false,
            arrival_rate: None,
            max_vus: 100,
            no_latency_correction: false,
            no_follow_redirects: false,
            config: None,
            output: None,
            db_url: None,
            format: "json".to_string(),
            no_tui: false,
            json: false,
            quiet: false,
            serious: false,
            insecure: false,
            yes: false,
            dry_run: false,
            fail_fast: false,
            debug: false,
            disable_keepalive: false,
            ws_message_interval: Duration::from_millis(100),
            ws_fire_and_forget: false,
            basic_auth: None,
            proxy: None,
            cert: None,
            key: None,
            cacert: None,
            connect_to: None,
            form: Vec::new(),
        }
    }
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

    /// Force comparison even if load models differ (open vs closed)
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct ImportArgs {
    /// Input file to import (HAR, Postman collection, or OpenAPI spec)
    pub input: PathBuf,

    /// Output file path (default: kaioken.toml)
    #[arg(short, long, default_value = "kaioken.toml")]
    pub output: PathBuf,

    /// Import format (auto-detected from extension if not specified)
    #[arg(short, long, value_enum)]
    pub format: Option<ImportFormat>,

    /// Filter requests by URL pattern (regex)
    #[arg(long)]
    pub filter: Option<String>,

    /// Overwrite existing output file
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ImportFormat {
    /// HAR (HTTP Archive) format from browser DevTools
    Har,
    /// Postman Collection v2.1
    Postman,
    /// OpenAPI 3.x specification
    Openapi,
}

impl RunArgs {
    pub fn parse_headers(&self) -> Result<Vec<(String, String)>, String> {
        self.headers
            .iter()
            .map(|h| {
                let parts: Vec<&str> = h.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(format!(
                        "Invalid header format: {}. Expected 'Name: Value'",
                        h
                    ));
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

pub fn generate_man_page() -> Result<(), std::io::Error> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut std::io::stdout())
}
