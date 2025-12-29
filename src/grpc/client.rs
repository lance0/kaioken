//! gRPC client implementation using tonic

use crate::grpc::types::{GrpcConfig, GrpcError, GrpcMethod, GrpcResult};
use bytes::{Buf, BufMut, Bytes};
use std::time::Instant;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};

/// gRPC client wrapper
pub struct GrpcClient {
    channel: Channel,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub async fn new(address: &str, tls: bool, _insecure: bool) -> Result<Self, GrpcError> {
        let scheme = if tls { "https" } else { "http" };
        let uri = format!("{}://{}", scheme, address);

        let endpoint = Endpoint::from_shared(uri)
            .map_err(|e| GrpcError::Connect(format!("Invalid address: {}", e)))?;

        let endpoint = if tls {
            endpoint
                .tls_config(tonic::transport::ClientTlsConfig::new().with_enabled_roots())
                .map_err(|e| GrpcError::Connect(format!("TLS config error: {}", e)))?
        } else {
            endpoint
        };

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| GrpcError::Connect(format!("Connection failed: {}", e)))?;

        Ok(Self { channel })
    }

    /// Get the channel for making requests
    #[allow(dead_code)]
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }
}

/// Raw bytes codec for dynamic protobuf
#[derive(Debug, Clone, Default)]
pub struct RawCodec;

impl Codec for RawCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = RawEncoder;
    type Decoder = RawDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        RawEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        RawDecoder
    }
}

#[derive(Debug, Clone, Default)]
pub struct RawEncoder;

impl Encoder for RawEncoder {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put_slice(&item);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct RawDecoder;

impl Decoder for RawDecoder {
    type Item = Bytes;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let chunk = src.chunk();
        if chunk.is_empty() {
            return Ok(None);
        }
        let bytes = Bytes::copy_from_slice(chunk);
        src.advance(chunk.len());
        Ok(Some(bytes))
    }
}

/// Execute a gRPC request and return the result
pub async fn execute_grpc_request(config: &GrpcConfig) -> GrpcResult {
    let start = Instant::now();

    let result = execute_grpc_internal(config).await;

    let latency_us = start.elapsed().as_micros() as u64;

    match result {
        Ok(mut grpc_result) => {
            grpc_result.latency_us = latency_us;
            grpc_result
        }
        Err(e) => GrpcResult {
            latency_us,
            status_code: -1,
            status_message: None,
            responses: Vec::new(),
            response_count: 0,
            bytes_received: 0,
            error: Some(e),
        },
    }
}

async fn execute_grpc_internal(config: &GrpcConfig) -> Result<GrpcResult, GrpcError> {
    // Build the endpoint
    let scheme = if config.tls { "https" } else { "http" };
    let uri = format!("{}://{}", scheme, config.address);

    let endpoint = Endpoint::from_shared(uri)
        .map_err(|e| GrpcError::Connect(format!("Invalid address: {}", e)))?
        .timeout(config.timeout);

    let endpoint = if config.tls {
        endpoint
            .tls_config(tonic::transport::ClientTlsConfig::new().with_enabled_roots())
            .map_err(|e| GrpcError::Connect(format!("TLS config error: {}", e)))?
    } else {
        endpoint
    };

    let channel = endpoint
        .connect()
        .await
        .map_err(|e| GrpcError::Connect(format!("Connection failed: {}", e)))?;

    // Build the path: /package.Service/Method
    let path = format!("/{}/{}", config.service, config.method);

    match config.method_type {
        GrpcMethod::Unary => execute_unary(channel, &path, config).await,
        GrpcMethod::ServerStream => execute_server_stream(channel, &path, config).await,
        GrpcMethod::ClientStream => {
            // For MVP, treat as unary
            execute_unary(channel, &path, config).await
        }
        GrpcMethod::BidiStream => {
            // For MVP, treat as unary
            execute_unary(channel, &path, config).await
        }
    }
}

/// Execute a unary gRPC call using tonic::client::Grpc
async fn execute_unary(
    channel: Channel,
    path: &str,
    config: &GrpcConfig,
) -> Result<GrpcResult, GrpcError> {
    use tonic::client::Grpc;

    let mut client = Grpc::new(channel);

    // Convert JSON request to bytes
    // For a full implementation, this would use prost-reflect or similar
    // For now, we send raw bytes (proto-encoded data must be provided)
    let request_bytes = Bytes::from(config.request.clone());

    // Build the request with metadata
    let mut request = Request::new(request_bytes);

    for (key, value) in &config.metadata {
        if let Ok(key) = key.parse::<tonic::metadata::MetadataKey<tonic::metadata::Ascii>>() {
            if let Ok(value) = value.parse() {
                request.metadata_mut().insert(key, value);
            }
        }
    }

    // Parse the path
    let path: tonic::codegen::http::uri::PathAndQuery = path
        .parse()
        .map_err(|e| GrpcError::Other(format!("Invalid path: {}", e)))?;

    // Make the unary call
    client
        .ready()
        .await
        .map_err(|e| GrpcError::Connect(format!("Service not ready: {}", e)))?;

    let response = client.unary(request, path, RawCodec).await;

    match response {
        Ok(response) => {
            let bytes = response.into_inner();
            let bytes_len = bytes.len() as u64;

            // Try to convert response to string for display
            let response_str = String::from_utf8_lossy(&bytes).to_string();

            Ok(GrpcResult {
                latency_us: 0, // Will be set by caller
                status_code: 0,
                status_message: Some("OK".to_string()),
                responses: vec![response_str],
                response_count: 1,
                bytes_received: bytes_len,
                error: None,
            })
        }
        Err(status) => Ok(GrpcResult {
            latency_us: 0,
            status_code: status.code() as i32,
            status_message: Some(status.message().to_string()),
            responses: Vec::new(),
            response_count: 0,
            bytes_received: 0,
            error: Some(GrpcError::Status(
                status.code() as i32,
                status.message().to_string(),
            )),
        }),
    }
}

/// Execute a server streaming gRPC call
async fn execute_server_stream(
    channel: Channel,
    path: &str,
    config: &GrpcConfig,
) -> Result<GrpcResult, GrpcError> {
    use futures_util::StreamExt;
    use tonic::client::Grpc;

    let mut client = Grpc::new(channel);

    let request_bytes = Bytes::from(config.request.clone());
    let mut request = Request::new(request_bytes);

    for (key, value) in &config.metadata {
        if let Ok(key) = key.parse::<tonic::metadata::MetadataKey<tonic::metadata::Ascii>>() {
            if let Ok(value) = value.parse() {
                request.metadata_mut().insert(key, value);
            }
        }
    }

    let path: tonic::codegen::http::uri::PathAndQuery = path
        .parse()
        .map_err(|e| GrpcError::Other(format!("Invalid path: {}", e)))?;

    client
        .ready()
        .await
        .map_err(|e| GrpcError::Connect(format!("Service not ready: {}", e)))?;

    // Make the server streaming call
    let response = client.server_streaming(request, path, RawCodec).await;

    match response {
        Ok(response) => {
            let mut stream = response.into_inner();
            let mut responses = Vec::new();
            let mut bytes_received = 0u64;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(bytes) => {
                        bytes_received += bytes.len() as u64;
                        responses.push(String::from_utf8_lossy(&bytes).to_string());
                    }
                    Err(status) => {
                        let count = responses.len() as u64;
                        return Ok(GrpcResult {
                            latency_us: 0,
                            status_code: status.code() as i32,
                            status_message: Some(status.message().to_string()),
                            responses,
                            response_count: count,
                            bytes_received,
                            error: Some(GrpcError::Stream(status.message().to_string())),
                        });
                    }
                }
            }

            Ok(GrpcResult {
                latency_us: 0,
                status_code: 0,
                status_message: Some("OK".to_string()),
                response_count: responses.len() as u64,
                responses,
                bytes_received,
                error: None,
            })
        }
        Err(status) => Ok(GrpcResult {
            latency_us: 0,
            status_code: status.code() as i32,
            status_message: Some(status.message().to_string()),
            responses: Vec::new(),
            response_count: 0,
            bytes_received: 0,
            error: Some(GrpcError::Status(
                status.code() as i32,
                status.message().to_string(),
            )),
        }),
    }
}
