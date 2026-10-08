use crate::recovery::RecoveryResult;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Represents the current state of a circuit breaker.
///
/// Marked `#[non_exhaustive]` so future minor releases can add new
/// states (e.g. `Disabled`, `ForcedOpen`) without breaking callers
/// that exhaustively `match` on the enum.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CircuitState {
    /// Circuit is closed and operations are allowed to execute
    Closed,

    /// Circuit is open and operations will fail fast
    Open,

    /// Circuit is partially open: the reset timeout has elapsed and
    /// a single probe request is admitted. Other calls fail fast
    /// until the probe resolves.
    HalfOpen,
}

/// Configuration for a circuit breaker.
///
/// Marked `#[non_exhaustive]` so future minor releases can add new
/// tuning knobs without breaking callers. Construct via
/// [`CircuitBreakerConfig::default`] then mutate the fields you
/// care about.
#[derive(Clone)]
#[non_exhaustive]
pub struct CircuitBreakerConfig {
    /// Number of failures required to open the circuit
    pub failure_threshold: usize,

    /// Time window in milliseconds to count failures
    pub failure_window_ms: u64,

    /// Time in milliseconds that the circuit stays open before trying again
    pub reset_timeout_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window_ms: 60000, // 1 minute
            reset_timeout_ms: 30000,  // 30 seconds
        }
    }
}

impl CircuitBreakerConfig {
    /// Construct a [`CircuitBreakerConfig`] from its three core knobs.
    ///
    /// Provided so external callers (tests, custom circuit-breaker
    /// wiring) can build the struct without depending on its field
    /// list, which may grow over the `1.x` line. For tuning only a
    /// subset, start from [`Self::default`] and use the
    /// `with_*` builder methods.
    pub fn new(failure_threshold: usize, failure_window_ms: u64, reset_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            failure_window_ms,
            reset_timeout_ms,
        }
    }

    /// Override the failure-count threshold.
    #[must_use]
    pub fn with_failure_threshold(mut self, threshold: usize) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Override the failure-counting window in milliseconds.
    #[must_use]
    pub fn with_failure_window_ms(mut self, window_ms: u64) -> Self {
        self.failure_window_ms = window_ms;
        self
    }

    /// Override the reset-timeout in milliseconds.
    #[must_use]
    pub fn with_reset_timeout_ms(mut self, reset_ms: u64) -> Self {
        self.reset_timeout_ms = reset_ms;
        self
    }
}

struct CircuitBreakerInner {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failures: Vec<Instant>,
    last_state_change: Instant,
    /// Set while the single half-open probe call is running.
    probe_in_flight: bool,
}

impl CircuitBreakerInner {
    /// The state the breaker is in once the reset timeout is taken into
    /// account, without recording the transition.
    fn effective_state(&self, now: Instant) -> CircuitState {
        if self.state == CircuitState::Open
            && now.duration_since(self.last_state_change)
                >= Duration::from_millis(self.config.reset_timeout_ms)
        {
            CircuitState::HalfOpen
        } else {
            self.state
        }
    }
}

/// Releases the half-open probe slot if the protected closure unwinds.
///
/// A panicking probe counts as a failed probe: the circuit reopens and
/// the reset timeout starts again. Without this the slot would stay
/// taken and the breaker would reject every later call.
struct ProbeGuard<'a> {
    inner: &'a Mutex<CircuitBreakerInner>,
    armed: bool,
}

impl Drop for ProbeGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let mut inner = self.inner.lock();
            inner.probe_in_flight = false;
            inner.state = CircuitState::Open;
            inner.last_state_change = Instant::now();
        }
    }
}

/// Circuit breaker implementation to prevent cascading failures
///
/// The circuit breaker tracks failures and "trips" after a threshold is reached,
/// preventing further calls and allowing the system to recover.
pub struct CircuitBreaker {
    name: String,
    inner: Arc<Mutex<CircuitBreakerInner>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given name and default configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            inner: Arc::new(Mutex::new(CircuitBreakerInner {
                config,
                state: CircuitState::Closed,
                failures: Vec::new(),
                last_state_change: Instant::now(),
                probe_in_flight: false,
            })),
        }
    }

    /// Get the current state of the circuit breaker.
    ///
    /// An open circuit whose reset timeout has elapsed reports
    /// [`CircuitState::HalfOpen`]: the next [`execute`](Self::execute)
    /// call is admitted as the probe. Reading the state does not
    /// change it.
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.lock();
        inner.effective_state(Instant::now())
    }

    /// Get the name of the circuit breaker
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Execute a function protected by the circuit breaker.
    ///
    /// While the circuit is open the call fails fast with
    /// [`CircuitOpenError`]. Once the reset timeout elapses exactly one
    /// call is admitted as a half-open probe; concurrent calls fail fast
    /// until it resolves. A successful probe closes the circuit, and a
    /// failed or panicking probe reopens it.
    pub fn execute<F, T, E>(&self, f: F) -> RecoveryResult<T>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // First check if we can proceed with the call
        let is_probe = {
            let mut inner = self.inner.lock();
            self.update_state(&mut inner);
            match inner.state {
                CircuitState::Closed => false,
                CircuitState::HalfOpen if !inner.probe_in_flight => {
                    inner.probe_in_flight = true;
                    true
                }
                // Open, or half-open with the probe already running.
                _ => return Err(Box::new(CircuitOpenError::new(&self.name))),
            }
        };

        let mut guard = ProbeGuard {
            inner: &self.inner,
            armed: is_probe,
        };

        // Execute the function
        let outcome = f();
        guard.armed = false;

        match outcome {
            Ok(value) => {
                // Success, potentially reset circuit breaker
                self.on_success(is_probe);
                Ok(value)
            }
            Err(err) => {
                // Failure, record it and potentially trip circuit
                self.on_failure(is_probe);
                Err(Box::new(err))
            }
        }
    }

    /// Manually reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        inner.state = CircuitState::Closed;
        inner.failures.clear();
        inner.last_state_change = Instant::now();
        inner.probe_in_flight = false;
    }

    /// Called when an operation succeeds
    fn on_success(&self, was_probe: bool) {
        let mut inner = self.inner.lock();
        if was_probe {
            inner.probe_in_flight = false;
        }
        if inner.state == CircuitState::HalfOpen {
            // Successful test request, close the circuit
            inner.state = CircuitState::Closed;
            inner.failures.clear();
            inner.last_state_change = Instant::now();
        }
    }

    /// Called when an operation fails
    fn on_failure(&self, was_probe: bool) {
        let mut inner = self.inner.lock();
        if was_probe {
            inner.probe_in_flight = false;
        }

        if inner.state == CircuitState::HalfOpen {
            // Failed during test request, reopen the circuit
            inner.state = CircuitState::Open;
            inner.last_state_change = Instant::now();
            return;
        }

        // Add the failure
        let now = Instant::now();
        inner.failures.push(now);

        // Remove old failures outside the window. When the window
        // reaches back past the start of the monotonic clock (a very
        // large `failure_window_ms`), every recorded failure is inside
        // it and nothing is dropped. Plain `Instant - Duration` panics
        // in that case.
        let window = Duration::from_millis(inner.config.failure_window_ms);
        if let Some(window_start) = now.checked_sub(window) {
            inner.failures.retain(|&time| time >= window_start);
        }

        // Check if threshold is reached
        if inner.state == CircuitState::Closed
            && inner.failures.len() >= inner.config.failure_threshold
        {
            // Trip the circuit
            inner.state = CircuitState::Open;
            inner.last_state_change = now;
        }
    }

    /// Update the circuit state based on timing
    fn update_state(&self, inner: &mut CircuitBreakerInner) {
        let now = Instant::now();
        if inner.state == CircuitState::Open && inner.effective_state(now) == CircuitState::HalfOpen
        {
            // Reset timeout has elapsed, try half-open state
            inner.state = CircuitState::HalfOpen;
            inner.last_state_change = now;
        }
    }
}

/// Error returned when circuit is open
#[derive(Debug)]
pub struct CircuitOpenError {
    circuit_name: String,
}

impl CircuitOpenError {
    fn new(circuit_name: &str) -> Self {
        Self {
            circuit_name: circuit_name.to_string(),
        }
    }
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circuit '{}' is open, failing fast", self.circuit_name)
    }
}

impl std::error::Error for CircuitOpenError {}
