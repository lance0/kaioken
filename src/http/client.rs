use reqwest::Client;
use reqwest::redirect::Policy;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn create_client(
    concurrency: u32,
    timeout: Duration,
    connect_timeout: Duration,
    insecure: bool,
    http2: bool,
    cookie_jar: bool,
    follow_redirects: bool,
    disable_keepalive: bool,
    proxy: Option<&str>,
    client_cert: Option<&Path>,
    client_key: Option<&Path>,
    ca_cert: Option<&Path>,
    connect_to: Option<(&str, SocketAddr)>,
) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let mut builder = Client::builder()
        .connect_timeout(connect_timeout)
        .timeout(timeout)
        .tcp_nodelay(true)
        .gzip(true)
        .brotli(true)
        .user_agent(format!(
            "kaioken/{} (load-testing-tool)",
            env!("CARGO_PKG_VERSION")
        ))
        .danger_accept_invalid_certs(insecure)
        .cookie_store(cookie_jar);

    // Configure connection pooling / keepalive
    if disable_keepalive {
        builder = builder
            .pool_max_idle_per_host(0)
            .pool_idle_timeout(Duration::ZERO);
    } else {
        builder = builder
            .pool_max_idle_per_host(concurrency as usize)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60));
    }

    if http2 {
        builder = builder.http2_prior_knowledge();
    }

    if !follow_redirects {
        builder = builder.redirect(Policy::none());
    }

    // Configure proxy if specified
    if let Some(proxy_url) = proxy {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        builder = builder.proxy(proxy);
    }

    // Configure custom CA certificate
    if let Some(ca_path) = ca_cert {
        let ca_pem = std::fs::read(ca_path)?;
        let cert = reqwest::Certificate::from_pem(&ca_pem)?;
        builder = builder.add_root_certificate(cert);
    }

    // Configure client identity for mTLS (cert + key)
    if let (Some(cert_path), Some(key_path)) = (client_cert, client_key) {
        // Combine cert and key into single PEM for Identity
        let mut pem = std::fs::read(cert_path)?;
        pem.push(b'\n');
        pem.extend(std::fs::read(key_path)?);
        let identity = reqwest::Identity::from_pem(&pem)?;
        builder = builder.identity(identity);
    }

    // Configure DNS override (--connect-to)
    if let Some((host, addr)) = connect_to {
        builder = builder.resolve(host, addr);
    }

    Ok(builder.build()?)
}
