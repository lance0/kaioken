use crate::engine::Stats;
use crate::types::StatsSnapshot;
use std::collections::HashMap;

pub fn create_snapshot(stats: &Stats) -> StatsSnapshot {
    create_snapshot_with_arrival_rate(stats, 0, 0, 0)
}

pub fn create_snapshot_with_arrival_rate(
    stats: &Stats,
    dropped_iterations: u64,
    vus_active: u32,
    vus_max: u32,
) -> StatsSnapshot {
    StatsSnapshot {
        elapsed: stats.elapsed(),
        total_requests: stats.total_requests,
        successful: stats.successful,
        failed: stats.failed,
        bytes_received: stats.bytes_received,

        rolling_rps: stats.rolling_rps(),
        requests_per_sec: stats.requests_per_sec(),
        error_rate: stats.error_rate(),

        latency_min_us: stats.latency_min(),
        latency_max_us: stats.latency_max(),
        latency_mean_us: stats.latency_mean(),
        latency_stddev_us: stats.latency_stddev(),
        latency_p50_us: stats.latency_percentile(50.0),
        latency_p75_us: stats.latency_percentile(75.0),
        latency_p90_us: stats.latency_percentile(90.0),
        latency_p95_us: stats.latency_percentile(95.0),
        latency_p99_us: stats.latency_percentile(99.0),
        latency_p999_us: stats.latency_percentile(99.9),

        status_codes: stats.status_codes.clone(),
        errors: stats.errors.clone(),
        timeline: stats.timeline.clone(),
        check_stats: HashMap::new(),
        overall_check_pass_rate: None,
        dropped_iterations,
        vus_active,
        vus_max,
    }
}
