use crate::types::WsErrorKind;
use futures_util::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Error as WsError, Message},
};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WsConnection {
    stream: WsStream,
    pub connect_time_us: u64,
}

impl WsConnection {
    pub fn new(stream: WsStream, connect_time_us: u64) -> Self {
        Self {
            stream,
            connect_time_us,
        }
    }

    pub async fn send(&mut self, message: &str) -> Result<(), WsErrorKind> {
        self.stream
            .send(Message::Text(message.into()))
            .await
            .map_err(|e| ws_error_to_kind(&e))
    }

    pub async fn receive(&mut self, timeout: Duration) -> Result<String, WsErrorKind> {
        let deadline = Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(WsErrorKind::Timeout);
            }

            match tokio::time::timeout(remaining, self.stream.next()).await {
                Ok(Some(Ok(msg))) => match msg {
                    Message::Text(text) => return Ok(text.to_string()),
                    Message::Binary(data) => return Ok(String::from_utf8_lossy(&data).to_string()),
                    Message::Close(_) => return Err(WsErrorKind::ConnectionClosed),
                    // Skip control frames, continue loop
                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
                },
                Ok(Some(Err(e))) => return Err(ws_error_to_kind(&e)),
                Ok(None) => return Err(WsErrorKind::ConnectionClosed),
                Err(_) => return Err(WsErrorKind::Timeout),
            }
        }
    }

    pub async fn close(mut self) -> Result<(), WsErrorKind> {
        self.stream
            .close(None)
            .await
            .map_err(|e| ws_error_to_kind(&e))
    }
}

/// Establish a new WebSocket connection
pub async fn connect(url: &str, timeout: Duration) -> Result<WsConnection, WsErrorKind> {
    let start = Instant::now();

    let result = tokio::time::timeout(timeout, connect_async(url)).await;

    match result {
        Ok(Ok((stream, _response))) => {
            let connect_time_us = start.elapsed().as_micros() as u64;
            Ok(WsConnection::new(stream, connect_time_us))
        }
        Ok(Err(e)) => Err(ws_error_to_kind(&e)),
        Err(_) => Err(WsErrorKind::Timeout),
    }
}

fn ws_error_to_kind(err: &WsError) -> WsErrorKind {
    match err {
        WsError::ConnectionClosed => WsErrorKind::ConnectionClosed,
        WsError::AlreadyClosed => WsErrorKind::ConnectionClosed,
        WsError::Io(io_err) => {
            let msg = io_err.to_string().to_lowercase();
            if msg.contains("refused") {
                WsErrorKind::ConnectFailed
            } else if msg.contains("tls") || msg.contains("certificate") {
                WsErrorKind::Tls
            } else {
                WsErrorKind::ConnectFailed
            }
        }
        WsError::Tls(_) => WsErrorKind::Tls,
        WsError::Protocol(_) => WsErrorKind::ProtocolError,
        WsError::Http(_) | WsError::HttpFormat(_) => WsErrorKind::HandshakeFailed,
        WsError::Url(_) => WsErrorKind::ConnectFailed,
        _ => WsErrorKind::Other,
    }
}
