//! Resilience, Circuit Breakers & Adaptive Concurrency Limiting.
//!
//! Protects backend services, database pools, and external API integrations
//! (Stripe, OpenAI, Resend) from cascading failures and thread pool starvation.

use std::{
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// Operating state of a [`CircuitBreaker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    /// Normal operation: requests pass through freely.
    Closed,
    /// Downstream service failing: requests fail fast immediately without hitting upstream.
    Open,
    /// Trial period: a limited number of requests are permitted to test upstream health.
    HalfOpen,
}

/// Configuration parameters for a [`CircuitBreaker`].
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u32,
    /// Duration in seconds to stay in Open state before testing recovery (Half-Open).
    pub recovery_timeout_secs: u64,
    /// Number of successful trial requests in Half-Open to restore to Closed.
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_secs: 30,
            success_threshold: 2,
        }
    }
}

/// High-performance thread-safe Circuit Breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: Arc<RwLock<BreakerState>>,
    consecutive_failures: Arc<AtomicU32>,
    consecutive_successes: Arc<AtomicU32>,
    last_state_change: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration.
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            name: name.into(),
            config,
            state: Arc::new(RwLock::new(BreakerState::Closed)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            consecutive_successes: Arc::new(AtomicU32::new(0)),
            last_state_change: Arc::new(AtomicU64::new(now)),
        }
    }

    /// Circuit breaker name identifier.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current circuit breaker state.
    pub fn state(&self) -> BreakerState {
        let current = *self.state.read().expect("Lock poisoned");
        if current == BreakerState::Open {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let opened_at = self.last_state_change.load(Ordering::Relaxed);
            if now >= opened_at + self.config.recovery_timeout_secs {
                // Transition to HalfOpen
                let mut lock = self.state.write().expect("Lock poisoned");
                *lock = BreakerState::HalfOpen;
                self.consecutive_successes.store(0, Ordering::Relaxed);
                return BreakerState::HalfOpen;
            }
        }
        current
    }

    /// Check whether a request is allowed to proceed.
    pub fn allow_request(&self) -> bool {
        match self.state() {
            BreakerState::Closed => true,
            BreakerState::HalfOpen => true,
            BreakerState::Open => false,
        }
    }

    /// Record a successful operation.
    pub fn record_success(&self) {
        let mut lock = self.state.write().expect("Lock poisoned");
        match *lock {
            BreakerState::Closed => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
            }
            BreakerState::HalfOpen => {
                let succ = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
                if succ >= self.config.success_threshold {
                    *lock = BreakerState::Closed;
                    self.consecutive_failures.store(0, Ordering::Relaxed);
                    self.consecutive_successes.store(0, Ordering::Relaxed);
                }
            }
            BreakerState::Open => {}
        }
    }

    /// Record a failed operation.
    pub fn record_failure(&self) {
        let mut lock = self.state.write().expect("Lock poisoned");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match *lock {
            BreakerState::Closed => {
                let fails = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if fails >= self.config.failure_threshold {
                    *lock = BreakerState::Open;
                    self.last_state_change.store(now, Ordering::Relaxed);
                }
            }
            BreakerState::HalfOpen => {
                *lock = BreakerState::Open;
                self.last_state_change.store(now, Ordering::Relaxed);
                self.consecutive_successes.store(0, Ordering::Relaxed);
            }
            BreakerState::Open => {}
        }
    }

    /// Execute a fallible asynchronous operation wrapped in the circuit breaker.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, BreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if !self.allow_request() {
            return Err(BreakerError::CircuitOpen(self.name.clone()));
        }

        match f().await {
            Ok(val) => {
                self.record_success();
                Ok(val)
            }
            Err(err) => {
                self.record_failure();
                Err(BreakerError::Inner(err))
            }
        }
    }
}

/// Error returned when calling through a circuit breaker.
#[derive(Debug, PartialEq, Eq)]
pub enum BreakerError<E> {
    /// Circuit is open and rejected the call immediately.
    CircuitOpen(String),
    /// Underlying operation failed.
    Inner(E),
}

/// Adaptive Concurrency Limiter regulating in-flight requests to prevent server exhaustion.
#[derive(Debug, Clone)]
pub struct AdaptiveConcurrencyLimiter {
    max_concurrency: usize,
    current_concurrency: Arc<AtomicU32>,
}

impl AdaptiveConcurrencyLimiter {
    /// Create a new limiter with maximum in-flight requests.
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency,
            current_concurrency: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Attempt to acquire a concurrency slot (returns `None` if capacity exceeded).
    pub fn try_acquire(&self) -> Option<ConcurrencyGuard> {
        let mut current = self.current_concurrency.load(Ordering::Relaxed);
        loop {
            if current as usize >= self.max_concurrency {
                return None;
            }
            match self.current_concurrency.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(ConcurrencyGuard {
                        counter: Arc::clone(&self.current_concurrency),
                    });
                }
                Err(actual) => current = actual,
            }
        }
    }

    /// Current number of requests currently executing.
    pub fn current_in_flight(&self) -> usize {
        self.current_concurrency.load(Ordering::Relaxed) as usize
    }
}

/// RAII Guard releasing a concurrency slot upon drop.
pub struct ConcurrencyGuard {
    counter: Arc<AtomicU32>,
}

impl Drop for ConcurrencyGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Bulkhead isolator allocating separate capacity pools for different workloads.
#[derive(Debug, Clone)]
pub struct Bulkhead {
    name: String,
    limiter: AdaptiveConcurrencyLimiter,
}

impl Bulkhead {
    pub fn new(name: impl Into<String>, max_concurrency: usize) -> Self {
        Self {
            name: name.into(),
            limiter: AdaptiveConcurrencyLimiter::new(max_concurrency),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn try_acquire(&self) -> Option<ConcurrencyGuard> {
        self.limiter.try_acquire()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_transitions() {
        let cb = CircuitBreaker::with_config(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 2,
                recovery_timeout_secs: 1,
                success_threshold: 1,
            },
        );

        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(cb.allow_request());

        // First failure
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Closed);

        // Second failure -> Trips to Open
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);
        assert!(!cb.allow_request());

        let res = cb.call(|| async { Ok::<&str, &str>("data") }).await;
        assert_eq!(
            res,
            Err(BreakerError::CircuitOpen("test-service".to_string()))
        );
    }

    #[test]
    fn test_adaptive_concurrency_limiter() {
        let limiter = AdaptiveConcurrencyLimiter::new(2);
        let g1 = limiter.try_acquire().expect("Should acquire first slot");
        let g2 = limiter.try_acquire().expect("Should acquire second slot");
        assert!(limiter.try_acquire().is_none());
        assert_eq!(limiter.current_in_flight(), 2);

        drop(g1);
        assert_eq!(limiter.current_in_flight(), 1);
        let _g3 = limiter.try_acquire().expect("Should acquire released slot");
        drop(g2);
    }
}
