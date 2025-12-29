//! gRPC-specific types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// gRPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// gRPC service address (e.g., "localhost:50051")
    pub address: String,

    /// Service name (e.g., "helloworld.Greeter")
    pub service: String,

    /// Method name (e.g., "SayHello")
    pub method: String,

    /// Request body (raw bytes or protobuf-encoded data)
    pub request: Vec<u8>,

    /// Request timeout
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Use TLS
    #[serde(default)]
    pub tls: bool,

    /// Skip certificate verification
    #[serde(default)]
    pub insecure: bool,

    /// Method type (unary, server_stream, client_stream, bidi_stream)
    #[serde(default)]
    pub method_type: GrpcMethod,

    /// Metadata (headers) to include
    #[serde(default)]
    pub metadata: Vec<(String, String)>,
}

fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// gRPC method type
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrpcMethod {
    /// Unary call: single request, single response
    #[default]
    Unary,
    /// Server streaming: single request, stream of responses
    ServerStream,
    /// Client streaming: stream of requests, single response
    ClientStream,
    /// Bidirectional streaming: stream of requests and responses
    BidiStream,
}

/// Result of a gRPC call
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GrpcResult {
    /// Latency in microseconds
    pub latency_us: u64,

    /// gRPC status code (0 = OK)
    pub status_code: i32,

    /// gRPC status message
    pub status_message: Option<String>,

    /// Response message(s) as JSON
    pub responses: Vec<String>,

    /// Number of response messages received
    pub response_count: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Error if any
    pub error: Option<GrpcError>,
}

/// gRPC-specific errors
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum GrpcError {
    /// Connection failed
    Connect(String),
    /// Timeout
    Timeout,
    /// Invalid request encoding
    Encoding(String),
    /// Server returned error status
    Status(i32, String),
    /// Stream error
    Stream(String),
    /// Other error
    Other(String),
}

#[allow(dead_code)]
impl GrpcError {
    pub fn as_str(&self) -> &'static str {
        match self {
            GrpcError::Connect(_) => "connect",
            GrpcError::Timeout => "timeout",
            GrpcError::Encoding(_) => "encoding",
            GrpcError::Status(_, _) => "status",
            GrpcError::Stream(_) => "stream",
            GrpcError::Other(_) => "other",
        }
    }
}

impl std::fmt::Display for GrpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrpcError::Connect(msg) => write!(f, "connect error: {}", msg),
            GrpcError::Timeout => write!(f, "timeout"),
            GrpcError::Encoding(msg) => write!(f, "encoding error: {}", msg),
            GrpcError::Status(code, msg) => write!(f, "status {}: {}", code, msg),
            GrpcError::Stream(msg) => write!(f, "stream error: {}", msg),
            GrpcError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            address: "localhost:50051".to_string(),
            service: String::new(),
            method: String::new(),
            request: Vec::new(),
            timeout: default_timeout(),
            tls: false,
            insecure: false,
            method_type: GrpcMethod::Unary,
            metadata: Vec::new(),
        }
    }
}
