use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, Semaphore};
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
