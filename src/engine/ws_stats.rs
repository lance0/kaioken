use crate::types::{WsErrorKind, WsMessageResult};
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct WsStats {
    // Message latency histogram (RTT for echo mode)
    message_histogram: Histogram<u64>,
    // Connection time histogram
    connect_histogram: Histogram<u64>,

    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,

    pub connections_established: u64,
    pub connection_errors: u64,
    pub disconnects: u64,

    pub errors: HashMap<WsErrorKind, u64>,

    start_time: Instant,
    rolling_window: Vec<(Instant, u64)>,
}

#[allow(dead_code)]
impl WsStats {
    pub fn new() -> Self {
        let message_histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3)
            .expect("Failed to create message histogram");
        let connect_histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3)
            .expect("Failed to create connect histogram");

        Self {
            message_histogram,
            connect_histogram,
            total_messages_sent: 0,
            total_messages_received: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            connections_established: 0,
            connection_errors: 0,
            disconnects: 0,
            errors: HashMap::new(),
            start_time: Instant::now(),
            rolling_window: Vec::with_capacity(100),
        }
    }

    pub fn reset(&mut self) {
        self.message_histogram.reset();
        self.connect_histogram.reset();
        self.total_messages_sent = 0;
        self.total_messages_received = 0;
        self.total_bytes_sent = 0;
        self.total_bytes_received = 0;
        self.connections_established = 0;
        self.connection_errors = 0;
        self.disconnects = 0;
        self.errors.clear();
        self.start_time = Instant::now();
        self.rolling_window.clear();
    }

    pub fn record_message(&mut self, result: &WsMessageResult) {
        self.total_messages_sent += 1;
        self.total_bytes_sent += result.bytes_sent;

        if result.is_success() {
            let latency = result.message_latency_us.min(60_000_000);
            let _ = self.message_histogram.record(latency);

            if result.bytes_received > 0 {
                self.total_messages_received += 1;
                self.total_bytes_received += result.bytes_received;
            }
        }

        if let Some(connect_time) = result.connect_time_us {
            let connect_clamped = connect_time.min(60_000_000);
            let _ = self.connect_histogram.record(connect_clamped);
            self.connections_established += 1;
        }

        if let Some(kind) = result.error {
            *self.errors.entry(kind).or_insert(0) += 1;

            if matches!(kind, WsErrorKind::ConnectionClosed) {
                self.disconnects += 1;
            }
            if matches!(
                kind,
                WsErrorKind::ConnectFailed | WsErrorKind::HandshakeFailed | WsErrorKind::Tls
            ) {
                self.connection_errors += 1;
            }
        }

        let now = Instant::now();
        self.rolling_window.push((now, 1));
        self.rolling_window
            .retain(|(t, _)| now.duration_since(*t) < Duration::from_secs(1));
    }

    pub fn record_connection(&mut self, connect_time_us: u64) {
        let connect_clamped = connect_time_us.min(60_000_000);
        let _ = self.connect_histogram.record(connect_clamped);
        self.connections_established += 1;
    }

    pub fn record_disconnect(&mut self) {
        self.disconnects += 1;
    }

    pub fn record_connection_error(&mut self, kind: WsErrorKind) {
        self.connection_errors += 1;
        *self.errors.entry(kind).or_insert(0) += 1;
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn rolling_messages_per_sec(&self) -> f64 {
        self.rolling_window.iter().map(|(_, c)| *c as f64).sum()
    }

    pub fn messages_per_sec(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_messages_sent as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_messages_sent > 0 {
            let total_errors: u64 = self.errors.values().sum();
            total_errors as f64 / self.total_messages_sent as f64
        } else {
            0.0
        }
    }

    // Message latency metrics
    pub fn message_latency_min(&self) -> u64 {
        self.message_histogram.min()
    }

    pub fn message_latency_max(&self) -> u64 {
        self.message_histogram.max()
    }

    pub fn message_latency_mean(&self) -> f64 {
        self.message_histogram.mean()
    }

    pub fn message_latency_stddev(&self) -> f64 {
        self.message_histogram.stdev()
    }

    pub fn message_latency_percentile(&self, p: f64) -> u64 {
        self.message_histogram.value_at_percentile(p)
    }

    // Connection time metrics
    pub fn connect_time_min(&self) -> u64 {
        self.connect_histogram.min()
    }

    pub fn connect_time_max(&self) -> u64 {
        self.connect_histogram.max()
    }

    pub fn connect_time_mean(&self) -> f64 {
        self.connect_histogram.mean()
    }

    pub fn connect_time_percentile(&self, p: f64) -> u64 {
        self.connect_histogram.value_at_percentile(p)
    }
}

impl Default for WsStats {
    fn default() -> Self {
        Self::new()
    }
}
