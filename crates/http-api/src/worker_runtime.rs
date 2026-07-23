//! Bounded background worker runtime (R1-REC-001).
//!
//! Every control-plane background unit must declare batch size, concurrency,
//! deadline, retry budget, and cooperate with process shutdown via
//! [`CancellationToken`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Shared process-wide shutdown flag for background workers.
#[derive(Debug, Clone, Default)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_shutdown(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_shutdown(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Limits for a single background worker loop.
#[derive(Debug, Clone)]
pub struct WorkerLimits {
    pub name: &'static str,
    pub batch_size: usize,
    pub max_concurrency: usize,
    pub cycle_deadline: Duration,
    pub retry_budget: u32,
    pub idle_sleep: Duration,
}

impl WorkerLimits {
    pub fn from_env(name: &'static str, prefix: &str, defaults: WorkerLimits) -> WorkerLimits {
        let parse_usize = |key: &str, default: usize| {
            std::env::var(format!("{prefix}_{key}"))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default)
        };
        let parse_u32 = |key: &str, default: u32| {
            std::env::var(format!("{prefix}_{key}"))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default)
        };
        let parse_secs = |key: &str, default: u64| {
            std::env::var(format!("{prefix}_{key}"))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default)
        };
        WorkerLimits {
            name,
            batch_size: parse_usize("BATCH", defaults.batch_size).clamp(1, 10_000),
            max_concurrency: parse_usize("CONCURRENCY", defaults.max_concurrency).clamp(1, 64),
            cycle_deadline: Duration::from_secs(
                parse_secs("DEADLINE_SECS", defaults.cycle_deadline.as_secs()).clamp(1, 3600),
            ),
            retry_budget: parse_u32("RETRY_BUDGET", defaults.retry_budget).clamp(0, 32),
            idle_sleep: Duration::from_millis(
                parse_secs("IDLE_MS", defaults.idle_sleep.as_millis() as u64).clamp(50, 60_000),
            ),
        }
    }
}

/// Tracks progress cursor/lease before external side effects.
#[derive(Debug, Clone, Default)]
pub struct WorkerCursor {
    pub last_id: Option<String>,
    pub processed: u64,
}

impl WorkerCursor {
    pub fn advance(&mut self, id: impl Into<String>) {
        self.last_id = Some(id.into());
        self.processed = self.processed.saturating_add(1);
    }
}

/// Run `body` until shutdown, enforcing cycle deadline logging.
pub async fn run_loop<F, Fut>(shutdown: ShutdownFlag, limits: WorkerLimits, mut body: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    tracing::info!(
        worker = limits.name,
        batch = limits.batch_size,
        concurrency = limits.max_concurrency,
        deadline_secs = limits.cycle_deadline.as_secs(),
        retry_budget = limits.retry_budget,
        "background worker started"
    );
    while !shutdown.is_shutdown() {
        let started = Instant::now();
        body().await;
        let elapsed = started.elapsed();
        if elapsed > limits.cycle_deadline {
            tracing::warn!(
                worker = limits.name,
                ?elapsed,
                deadline = ?limits.cycle_deadline,
                "worker cycle exceeded deadline"
            );
        }
        if shutdown.is_shutdown() {
            break;
        }
        tokio::time::sleep(limits.idle_sleep).await;
    }
    tracing::info!(
        worker = limits.name,
        "background worker drained and stopped"
    );
}
