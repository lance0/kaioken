use crate::types::{WsMessageResult, WsMode};
use crate::ws::client::WsConnection;
use std::time::{Duration, Instant};

/// Execute a WebSocket message exchange
pub async fn execute_ws_message(
    conn: &mut WsConnection,
    message: &str,
    mode: WsMode,
    timeout: Duration,
) -> WsMessageResult {
    let bytes_sent = message.len() as u64;
    let start = Instant::now();

    // Send the message
    if let Err(e) = conn.send(message).await {
        return WsMessageResult::error(e);
    }

    match mode {
        WsMode::Echo => {
            // Wait for response
            match conn.receive(timeout).await {
                Ok(response) => {
                    let latency_us = start.elapsed().as_micros() as u64;
                    let bytes_received = response.len() as u64;
                    WsMessageResult::success(latency_us, bytes_sent, bytes_received)
                        .with_response(response)
                }
                Err(e) => WsMessageResult::error(e),
            }
        }
        WsMode::FireAndForget => {
            // Don't wait for response
            let latency_us = start.elapsed().as_micros() as u64;
            WsMessageResult::success(latency_us, bytes_sent, 0)
        }
    }
}
