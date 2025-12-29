//! HTTP/3 client using h3 + quinn

use bytes::Buf;
use h3::client::SendRequest;
use h3_quinn::OpenStreams;
use quinn::{ClientConfig, Endpoint};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::types::{ErrorKind, RequestResult};

/// HTTP/3 client wrapper
pub struct Http3Client {
    endpoint: Endpoint,
    #[allow(dead_code)]
    server_name: String,
}

/// HTTP/3 response data
#[allow(dead_code)]
pub struct Http3Response {
    pub status: u16,
    pub body: Vec<u8>,
    pub latency_us: u64,
}

impl Http3Client {
    /// Create a new HTTP/3 client
    pub fn new(insecure: bool) -> Result<Self, String> {
        let mut crypto = rustls::ClientConfig::builder()
            .with_root_certificates(Self::root_certs()?)
            .with_no_client_auth();

        if insecure {
            crypto
                .dangerous()
                .set_certificate_verifier(Arc::new(InsecureVerifier));
        }

        crypto.alpn_protocols = vec![b"h3".to_vec()];

        let client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
                .map_err(|e| format!("Failed to create QUIC config: {}", e))?,
        ));

        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| format!("Failed to create endpoint: {}", e))?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            server_name: String::new(),
        })
    }

    fn root_certs() -> Result<rustls::RootCertStore, String> {
        let mut roots = rustls::RootCertStore::empty();
        let result = rustls_native_certs::load_native_certs();

        // Check for errors (but don't fail if some certs loaded)
        if result.certs.is_empty() && !result.errors.is_empty() {
            return Err(format!("Failed to load native certs: {:?}", result.errors));
        }

        for cert in result.certs {
            roots.add(cert).ok();
        }
        Ok(roots)
    }

    /// Connect to a server and return a send request handle
    pub async fn connect(
        &self,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<SendRequest<OpenStreams, bytes::Bytes>, String> {
        let connection = self
            .endpoint
            .connect(addr, server_name)
            .map_err(|e| format!("Failed to connect: {}", e))?
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        let (mut driver, send_request) = h3::client::new(h3_quinn::Connection::new(connection))
            .await
            .map_err(|e| format!("H3 handshake failed: {}", e))?;

        // Spawn the connection driver
        tokio::spawn(async move {
            // poll_close drives the connection and returns when closed
            let err = futures_util::future::poll_fn(|cx| driver.poll_close(cx)).await;
            tracing::debug!("H3 connection closed: {:?}", err);
        });

        Ok(send_request)
    }
}

/// Execute an HTTP/3 request and return the result
#[allow(dead_code)]
pub async fn execute_http3_request(
    client: &Http3Client,
    addr: SocketAddr,
    server_name: &str,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Option<&str>,
    timeout: Duration,
) -> RequestResult {
    let start = Instant::now();

    let result = tokio::time::timeout(timeout, async {
        // Connect
        let mut send_request = client.connect(addr, server_name).await?;

        // Build request
        let mut req = http::Request::builder()
            .method(method)
            .uri(path)
            .header(":authority", server_name);

        for (name, value) in headers {
            req = req.header(name.as_str(), value.as_str());
        }

        let req = req
            .body(())
            .map_err(|e| format!("Failed to build request: {}", e))?;

        // Send request
        let mut stream = send_request
            .send_request(req)
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        // Send body if present
        if let Some(body_data) = body {
            stream
                .send_data(bytes::Bytes::from(body_data.to_string()))
                .await
                .map_err(|e| format!("Failed to send body: {}", e))?;
        }

        stream
            .finish()
            .await
            .map_err(|e| format!("Failed to finish stream: {}", e))?;

        // Receive response
        let response = stream
            .recv_response()
            .await
            .map_err(|e| format!("Failed to receive response: {}", e))?;

        let status = response.status().as_u16();

        // Read response body
        let mut body = Vec::new();
        while let Some(chunk) = stream
            .recv_data()
            .await
            .map_err(|e| format!("Failed to receive data: {}", e))?
        {
            body.extend_from_slice(chunk.chunk());
        }

        Ok::<_, String>((status, body))
    })
    .await;

    let latency_us = start.elapsed().as_micros() as u64;

    match result {
        Ok(Ok((status, body))) => RequestResult {
            status: Some(status),
            latency_us,
            bytes_received: body.len() as u64,
            error: None,
            body: Some(String::from_utf8_lossy(&body).to_string()),
            scheduled_at_us: None,
            started_at_us: None,
            queue_time_us: None,
        },
        Ok(Err(_e)) => RequestResult {
            status: None,
            latency_us,
            bytes_received: 0,
            error: Some(ErrorKind::Other),
            body: None,
            scheduled_at_us: None,
            started_at_us: None,
            queue_time_us: None,
        },
        Err(_) => RequestResult {
            status: None,
            latency_us,
            bytes_received: 0,
            error: Some(ErrorKind::Timeout),
            body: None,
            scheduled_at_us: None,
            started_at_us: None,
            queue_time_us: None,
        },
    }
}

/// Insecure certificate verifier for testing
#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
