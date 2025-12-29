use crate::engine::Stats;
use crate::types::StatsSnapshot;
use std::collections::HashMap;

pub fn create_snapshot(stats: &Stats) -> StatsSnapshot {
    create_snapshot_with_arrival_rate(stats, 0, 0, 0, 0)
}

pub fn create_snapshot_with_arrival_rate(
    stats: &Stats,
    dropped_iterations: u64,
    vus_active: u32,
    vus_max: u32,
    target_rate: u32,
) -> StatsSnapshot {
    // Get corrected latency metrics if available
    let latency_correction_enabled = stats.has_corrected_latency();
    let (corrected_min, corrected_max, corrected_mean) = if latency_correction_enabled {
        (
            Some(stats.corrected_latency_min()),
            Some(stats.corrected_latency_max()),
            Some(stats.corrected_latency_mean()),
        )
    } else {
        (None, None, None)
    };

    let corrected_percentiles = if latency_correction_enabled {
        (
            Some(stats.corrected_latency_percentile(50.0)),
            Some(stats.corrected_latency_percentile(75.0)),
            Some(stats.corrected_latency_percentile(90.0)),
            Some(stats.corrected_latency_percentile(95.0)),
            Some(stats.corrected_latency_percentile(99.0)),
            Some(stats.corrected_latency_percentile(99.9)),
        )
    } else {
        (None, None, None, None, None, None)
    };

    let (queue_mean, queue_p99) = if latency_correction_enabled {
        (
            Some(stats.queue_time_mean()),
            Some(stats.queue_time_percentile(99.0)),
        )
    } else {
        (None, None)
    };

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
        target_rate,

        // Latency correction metrics
        latency_correction_enabled,
        corrected_latency_min_us: corrected_min,
        corrected_latency_max_us: corrected_max,
        corrected_latency_mean_us: corrected_mean,
        corrected_latency_p50_us: corrected_percentiles.0,
        corrected_latency_p75_us: corrected_percentiles.1,
        corrected_latency_p90_us: corrected_percentiles.2,
        corrected_latency_p95_us: corrected_percentiles.3,
        corrected_latency_p99_us: corrected_percentiles.4,
        corrected_latency_p999_us: corrected_percentiles.5,
        queue_time_mean_us: queue_mean,
        queue_time_p99_us: queue_p99,
        total_queue_time_us: stats.total_queue_time_us,

        // WebSocket fields (default to zero for HTTP tests)
        is_websocket: false,
        ws_messages_sent: 0,
        ws_messages_received: 0,
        ws_bytes_sent: 0,
        ws_bytes_received: 0,
        ws_connections_active: 0,
        ws_connections_established: 0,
        ws_connection_errors: 0,
        ws_disconnects: 0,
        ws_messages_per_sec: 0.0,
        ws_rolling_mps: 0.0,
        ws_error_rate: 0.0,
        ws_errors: HashMap::new(),
        ws_latency_min_us: 0,
        ws_latency_max_us: 0,
        ws_latency_mean_us: 0.0,
        ws_latency_stddev_us: 0.0,
        ws_latency_p50_us: 0,
        ws_latency_p95_us: 0,
        ws_latency_p99_us: 0,
        ws_connect_time_mean_us: 0.0,
        ws_connect_time_p99_us: 0,
    }
}
