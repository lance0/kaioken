use crate::types::Stage;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, Semaphore, watch};
use tokio::time::sleep;

pub struct RateLimiter {
    rate: u32,
    tokens: AtomicU64,
    max_tokens: u64,
    refill_notify: Notify,
}

impl RateLimiter {
    pub fn new(rate: u32) -> Arc<Self> {
        let max_tokens = rate as u64; // 1 second burst
        Arc::new(Self {
            rate,
            tokens: AtomicU64::new(max_tokens),
            max_tokens,
            refill_notify: Notify::new(),
        })
    }

    pub async fn acquire(&self) {
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current > 0 {
                if self
                    .tokens
                    .compare_exchange(current, current - 1, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
            } else {
                self.refill_notify.notified().await;
            }
        }
    }

    pub async fn run_refiller(self: Arc<Self>) {
        let interval = Duration::from_micros(1_000_000 / self.rate as u64);
        let mut next_refill = Instant::now() + interval;

        loop {
            sleep(next_refill.saturating_duration_since(Instant::now())).await;
            next_refill = Instant::now() + interval;

            let current = self.tokens.load(Ordering::Relaxed);
            if current < self.max_tokens {
                self.tokens.store(current + 1, Ordering::Relaxed);
                self.refill_notify.notify_one();
            }
        }
    }
}

#[allow(dead_code)]
pub struct RampUpScheduler {
    concurrency: u32,
    ramp_duration: Duration,
    active_permits: Arc<Semaphore>,
    start_time: Instant,
}

impl RampUpScheduler {
    pub fn new(concurrency: u32, ramp_duration: Duration) -> Self {
        let initial = if ramp_duration.is_zero() {
            concurrency as usize
        } else {
            1
        };

        Self {
            concurrency,
            ramp_duration,
            active_permits: Arc::new(Semaphore::new(initial)),
            start_time: Instant::now(),
        }
    }

    pub fn permits(&self) -> Arc<Semaphore> {
        self.active_permits.clone()
    }

    pub async fn run(self) {
        if self.ramp_duration.is_zero() {
            return;
        }

        let interval = self.ramp_duration.as_micros() as u64 / self.concurrency as u64;
        let interval = Duration::from_micros(interval.max(1));

        let mut activated = 1u32;

        while activated < self.concurrency {
            sleep(interval).await;
            activated += 1;
            self.active_permits.add_permits(1);
        }
    }

    #[allow(dead_code)]
    pub fn current_active(&self) -> u32 {
        if self.ramp_duration.is_zero() {
            return self.concurrency;
        }

        let elapsed = self.start_time.elapsed();
        if elapsed >= self.ramp_duration {
            self.concurrency
        } else {
            let progress = elapsed.as_secs_f64() / self.ramp_duration.as_secs_f64();
            ((self.concurrency as f64 * progress) as u32).max(1)
        }
    }
}

/// Info about current stage for display purposes
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct StageInfo {
    pub stage_index: usize,
    pub stage_count: usize,
    pub target: u32,
    pub current: u32,
    pub stage_elapsed: Duration,
    pub stage_duration: Duration,
}

#[allow(dead_code)]
pub struct StagesScheduler {
    stages: Vec<Stage>,
    active_permits: Arc<Semaphore>,
    current_target: Arc<AtomicU32>,
    stage_info_tx: watch::Sender<StageInfo>,
    start_time: Instant,
}

impl StagesScheduler {
    pub fn new(stages: Vec<Stage>, max_concurrency: u32) -> (Self, watch::Receiver<StageInfo>) {
        let initial_target = stages
            .first()
            .and_then(|s| s.target)
            .unwrap_or(max_concurrency);
        let initial_permits = initial_target.min(1) as usize; // Start with at least 1

        let (stage_info_tx, stage_info_rx) = watch::channel(StageInfo {
            stage_index: 0,
            stage_count: stages.len(),
            target: initial_target,
            current: initial_permits as u32,
            stage_elapsed: Duration::ZERO,
            stage_duration: stages.first().map(|s| s.duration).unwrap_or(Duration::ZERO),
        });

        (
            Self {
                stages,
                active_permits: Arc::new(Semaphore::new(initial_permits)),
                current_target: Arc::new(AtomicU32::new(initial_target)),
                stage_info_tx,
                start_time: Instant::now(),
            },
            stage_info_rx,
        )
    }

    pub fn permits(&self) -> Arc<Semaphore> {
        self.active_permits.clone()
    }

    #[allow(dead_code)]
    pub fn current_target(&self) -> u32 {
        self.current_target.load(Ordering::Relaxed)
    }

    /// Calculate total duration of all stages
    pub fn total_duration(&self) -> Duration {
        self.stages.iter().map(|s| s.duration).sum()
    }

    pub async fn run(self) {
        if self.stages.is_empty() {
            return;
        }

        let mut current_workers: u32 = self
            .stages
            .first()
            .and_then(|s| s.target)
            .map(|t| t.min(1))
            .unwrap_or(1);
        let mut stage_start = Instant::now();

        for (stage_idx, stage) in self.stages.iter().enumerate() {
            // Skip stages without VU target (they're arrival rate stages)
            let target = match stage.target {
                Some(t) => t,
                None => continue,
            };
            self.current_target.store(target, Ordering::Relaxed);

            // Calculate ramp rate: how often to add/remove a worker
            let workers_diff = (target as i64 - current_workers as i64).unsigned_abs() as u32;
            let ramp_interval = if workers_diff > 0 && !stage.duration.is_zero() {
                stage.duration / workers_diff
            } else {
                Duration::from_millis(100) // Default tick for status updates
            };

            let stage_end = stage_start + stage.duration;

            while Instant::now() < stage_end {
                // Update stage info
                let _ = self.stage_info_tx.send(StageInfo {
                    stage_index: stage_idx,
                    stage_count: self.stages.len(),
                    target,
                    current: current_workers,
                    stage_elapsed: stage_start.elapsed(),
                    stage_duration: stage.duration,
                });

                // Adjust workers toward target
                if current_workers < target {
                    self.active_permits.add_permits(1);
                    current_workers += 1;
                } else if current_workers > target && current_workers > 0 {
                    // To reduce workers, we'd need to signal workers to stop
                    // For simplicity, we just track the target - workers will naturally
                    // complete and not be replaced
                    current_workers = target;
                }

                let sleep_time =
                    ramp_interval.min(stage_end.saturating_duration_since(Instant::now()));
                if sleep_time.is_zero() {
                    break;
                }
                sleep(sleep_time).await;
            }

            stage_start = Instant::now();
        }

        // Final stage info update - only for VU-based stages
        if let Some(last) = self.stages.last()
            && let Some(target) = last.target {
                let _ = self.stage_info_tx.send(StageInfo {
                    stage_index: self.stages.len() - 1,
                    stage_count: self.stages.len(),
                    target,
                    current: target,
                    stage_elapsed: last.duration,
                    stage_duration: last.duration,
                });
            }
    }
}
