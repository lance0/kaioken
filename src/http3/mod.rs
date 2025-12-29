//! HTTP/3 client implementation (feature-gated)
//!
//! Enable with: cargo build --features http3
//!
//! HTTP/3 uses QUIC as the transport layer, providing:
//! - Reduced connection establishment latency (0-RTT)
//! - Multiplexing without head-of-line blocking
//! - Better performance on lossy networks

mod client;

pub use client::{Http3Client, Http3Response, execute_http3_request};
