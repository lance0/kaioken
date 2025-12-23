use clap::Parser;
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
    /// Target URL to load test
    #[arg(required = true)]
    pub url: String,

    /// Number of concurrent workers
    #[arg(short = 'c', long, default_value = "50")]
    pub concurrency: u32,

    /// Test duration (e.g., 10s, 1m, 30s)
    #[arg(short = 'd', long, default_value = "10s", value_parser = parse_duration)]
    pub duration: Duration,

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
}

impl Cli {
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

    pub fn parse_method(&self) -> Result<reqwest::Method, String> {
        self.method
            .to_uppercase()
            .parse()
            .map_err(|_| format!("Invalid HTTP method: {}", self.method))
    }

    pub fn is_localhost(&self) -> bool {
        let url_lower = self.url.to_lowercase();
        url_lower.contains("localhost")
            || url_lower.contains("127.0.0.1")
            || url_lower.contains("[::1]")
    }
}
