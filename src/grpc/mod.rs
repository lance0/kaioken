//! gRPC client implementation (feature-gated)
//!
//! Enable with: cargo build --features grpc
//!
//! Supports:
//! - Unary calls (request/response)
//! - Server streaming
//! - Client streaming
//! - Bidirectional streaming
//!
//! Uses dynamic protobuf encoding for flexibility without .proto files.

mod client;
mod types;

#[allow(unused_imports)]
pub use client::{GrpcClient, execute_grpc_request};
#[allow(unused_imports)]
pub use types::{GrpcConfig, GrpcError, GrpcMethod, GrpcResult};
