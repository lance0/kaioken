use crate::types::{WsErrorKind, WsMessageResult, WsMode};
use crate::ws::{WsConnection, connect, execute_ws_message};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub struct WsWorker {
    id: u32,
    url: String,
    message: String,
    mode: WsMode,
    message_interval: Duration,
    timeout: Duration,
    result_tx: mpsc::Sender<WsMessageResult>,
    cancel_token: CancellationToken,
}

impl WsWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        url: String,
        message: String,
        mode: WsMode,
        message_interval: Duration,
        timeout: Duration,
        result_tx: mpsc::Sender<WsMessageResult>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            id,
            url,
            message,
            mode,
            message_interval,
            timeout,
            result_tx,
            cancel_token,
        }
    }

    pub async fn run(self) {
        tracing::debug!("WsWorker {} starting", self.id);

        let mut connection: Option<WsConnection> = None;
        let mut message_counter: u64 = 0;
        let base_message_id = (self.id as u64) * 1_000_000_000;

        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            // Ensure we have a connection
            if connection.is_none() {
                match connect(&self.url, self.timeout).await {
                    Ok(conn) => {
                        tracing::debug!("WsWorker {} connected", self.id);
                        // Record the connection time with the first message
                        connection = Some(conn);
                    }
                    Err(e) => {
                        // Send connection error result
                        let result = WsMessageResult::error(e);
                        if self.result_tx.send(result).await.is_err() {
                            break;
                        }
                        // Wait before retry
                        tokio::select! {
                            _ = sleep(Duration::from_secs(1)) => {}
                            _ = self.cancel_token.cancelled() => break,
                        }
                        continue;
                    }
                }
            }

            let conn = connection.as_mut().unwrap();
            let is_first_message = message_counter == 0;

            message_counter += 1;
            let _message_id = base_message_id + message_counter;
            let timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);

            // Interpolate message
            let message = self
                .message
                .replace("${MESSAGE_ID}", &message_counter.to_string())
                .replace("${TIMESTAMP_MS}", &timestamp_ms.to_string());

            let start = Instant::now();
            let mut result = execute_ws_message(conn, &message, self.mode, self.timeout).await;

            // Include connect time on first message
            if is_first_message {
                result = result.with_connect_time(conn.connect_time_us);
            }

            // Check for connection loss
            let connection_lost = matches!(
                result.error,
                Some(WsErrorKind::ConnectionClosed) | Some(WsErrorKind::SendFailed)
            );

            if self.result_tx.send(result).await.is_err() {
                break;
            }

            if connection_lost {
                tracing::debug!("WsWorker {} connection lost, will reconnect", self.id);
                connection = None;
                continue;
            }

            // Wait for next message interval
            let elapsed = start.elapsed();
            if elapsed < self.message_interval {
                let remaining = self.message_interval - elapsed;
                tokio::select! {
                    _ = sleep(remaining) => {}
                    _ = self.cancel_token.cancelled() => break,
                }
            }
        }

        // Close connection gracefully
        if let Some(conn) = connection {
            let _ = conn.close().await;
        }

        tracing::debug!("WsWorker {} stopped", self.id);
    }
}
