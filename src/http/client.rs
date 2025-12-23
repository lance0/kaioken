use reqwest::Client;
use std::time::Duration;

pub fn create_client(
    concurrency: u32,
    timeout: Duration,
    connect_timeout: Duration,
    insecure: bool,
) -> Result<Client, reqwest::Error> {
    Client::builder()
        .pool_max_idle_per_host(concurrency as usize)
        .pool_idle_timeout(Duration::from_secs(30))
        .connect_timeout(connect_timeout)
        .timeout(timeout)
        .tcp_keepalive(Duration::from_secs(60))
        .tcp_nodelay(true)
        .gzip(true)
        .brotli(true)
        .user_agent(format!("kaioken/{} (load-testing-tool)", env!("CARGO_PKG_VERSION")))
        .danger_accept_invalid_certs(insecure)
        .build()
}
