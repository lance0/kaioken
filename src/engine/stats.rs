use crate::types::{ErrorKind, RequestResult, TimelineBucket};
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Stats {
    histogram: Histogram<u64>,
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub bytes_received: u64,
    pub status_codes: HashMap<u16, u64>,
    pub errors: HashMap<ErrorKind, u64>,
    pub timeline: Vec<TimelineBucket>,
    start_time: Instant,
    last_second_requests: u64,
    last_second_time: Instant,
    rolling_window: Vec<(Instant, u64)>,
}

impl Stats {
    pub fn new(duration: Duration) -> Self {
        let histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3)
            .expect("Failed to create histogram");

        let timeline_capacity = duration.as_secs() as usize + 60;

        Self {
            histogram,
            total_requests: 0,
            successful: 0,
            failed: 0,
            bytes_received: 0,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            timeline: Vec::with_capacity(timeline_capacity),
            start_time: Instant::now(),
            last_second_requests: 0,
            last_second_time: Instant::now(),
            rolling_window: Vec::with_capacity(100),
        }
    }

    pub fn reset(&mut self) {
        self.histogram.reset();
        self.total_requests = 0;
        self.successful = 0;
        self.failed = 0;
        self.bytes_received = 0;
        self.status_codes.clear();
        self.errors.clear();
        self.timeline.clear();
        self.start_time = Instant::now();
        self.last_second_requests = 0;
        self.last_second_time = Instant::now();
        self.rolling_window.clear();
    }

    pub fn record(&mut self, result: &RequestResult) {
        self.total_requests += 1;
        self.bytes_received += result.bytes_received;

        let latency = result.latency_us.min(60_000_000);
        let _ = self.histogram.record(latency);

        if result.is_success() {
            self.successful += 1;
        } else {
            self.failed += 1;
        }

        if let Some(status) = result.status {
            *self.status_codes.entry(status).or_insert(0) += 1;
        }

        if let Some(kind) = result.error {
            *self.errors.entry(kind).or_insert(0) += 1;
        }

        let now = Instant::now();
        self.rolling_window.push((now, 1));
        self.rolling_window.retain(|(t, _)| now.duration_since(*t) < Duration::from_secs(1));

        self.update_timeline();
    }

    fn update_timeline(&mut self) {
        let elapsed_secs = self.start_time.elapsed().as_secs() as u32;

        if self.timeline.is_empty() || self.timeline.last().unwrap().elapsed_secs < elapsed_secs {
            if let Some(last) = self.timeline.last_mut() {
                last.requests = self.last_second_requests;
            }

            self.timeline.push(TimelineBucket {
                elapsed_secs,
                requests: 0,
                errors: 0,
            });
            self.last_second_requests = 0;
        }

        self.last_second_requests += 1;

        if let Some(bucket) = self.timeline.last_mut() {
            bucket.requests = self.last_second_requests;
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn rolling_rps(&self) -> f64 {
        self.rolling_window.iter().map(|(_, c)| *c as f64).sum()
    }

    pub fn requests_per_sec(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_requests as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.failed as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    pub fn latency_min(&self) -> u64 {
        self.histogram.min()
    }

    pub fn latency_max(&self) -> u64 {
        self.histogram.max()
    }

    pub fn latency_mean(&self) -> f64 {
        self.histogram.mean()
    }

    pub fn latency_stddev(&self) -> f64 {
        self.histogram.stdev()
    }

    pub fn latency_percentile(&self, p: f64) -> u64 {
        self.histogram.value_at_percentile(p)
    }
}
