use crate::recovery::RecoveryResult;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
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
    /// until the probe resolves. Use
    /// [`CircuitBreaker::is_probing`] to tell whether the probe is
    /// already running (calls fail fast) or the next call will be the
    /// probe.
    HalfOpen,
}

/// Configuration for a circuit breaker.
///
/// Marked `#[non_exhaustive]` so future minor releases can add new
/// tuning knobs without breaking callers. Construct via
/// [`CircuitBreakerConfig::default`] then mutate the fields you
/// care about.
///
/// The breaker counts failures, not a failure rate: it opens once
/// `failure_threshold` calls have failed within the last
/// `failure_window_ms`, however many calls succeeded in between. Every
/// `Err` returned by the protected closure counts as a failure.
#[derive(Clone)]
#[non_exhaustive]
pub struct CircuitBreakerConfig {
    /// Number of failures required to open the circuit
    pub failure_threshold: usize,

    /// Time window in milliseconds to count failures
    pub failure_window_ms: u64,

    /// Time in milliseconds that the circuit stays open before trying again
    pub reset_timeout_ms: u64,

    /// Longest time in milliseconds a half-open probe may run before it
    /// counts as failed. `None` (the default) uses `reset_timeout_ms`.
    ///
    /// While the probe runs every other call fails fast, so a probe that
    /// never returns (a dependency that accepts connections but never
    /// answers) would otherwise keep the circuit failing until
    /// [`CircuitBreaker::reset`]. Once the probe is older than this, the
    /// circuit reopens as if the probe had failed, and a new probe is
    /// admitted after the next reset timeout. The late result of the old
    /// probe is then ignored. Closures should still set their own I/O
    /// timeouts: the breaker cannot interrupt a call that hangs.
    pub probe_timeout_ms: Option<u64>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window_ms: 60000, // 1 minute
            reset_timeout_ms: 30000,  // 30 seconds
            probe_timeout_ms: None,
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
            probe_timeout_ms: None,
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

    /// Override the longest time a half-open probe may run before it
    /// counts as failed (see [`Self::probe_timeout_ms`]).
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::CircuitBreakerConfig;
    ///
    /// let config = CircuitBreakerConfig::default().with_probe_timeout_ms(2_000);
    /// assert_eq!(config.probe_timeout_ms, Some(2_000));
    /// ```
    #[must_use]
    pub fn with_probe_timeout_ms(mut self, probe_ms: u64) -> Self {
        self.probe_timeout_ms = Some(probe_ms);
        self
    }

    /// The probe timeout in effect.
    fn probe_timeout(&self) -> Duration {
        Duration::from_millis(self.probe_timeout_ms.unwrap_or(self.reset_timeout_ms))
    }
}

/// Lock the breaker state, ignoring poisoning.
///
/// The protected closure never runs while the lock is held; the lock
/// only covers short updates of the breaker's own fields, each of which
/// leaves the state valid. Taking a poisoned lock anyway keeps every
/// method panic-free, as the non-poisoning `parking_lot` lock used up to
/// 1.0.2 did.
fn lock(inner: &Mutex<CircuitBreakerInner>) -> MutexGuard<'_, CircuitBreakerInner> {
    inner.lock().unwrap_or_else(PoisonError::into_inner)
}

struct CircuitBreakerInner {
    config: CircuitBreakerConfig,
    state: CircuitState,
    /// Failure times inside the window, oldest first, at most
    /// `failure_threshold` of them.
    failures: VecDeque<Instant>,
    last_state_change: Instant,
    /// Set while the single half-open probe call is running.
    probe_in_flight: bool,
    /// When the running probe was admitted.
    probe_started: Instant,
    /// Bumped on every state change. A call records the generation it
    /// was admitted under, and its outcome only counts while that
    /// generation is still current. A slow call admitted while the
    /// circuit was closed therefore cannot close or reopen it after it
    /// has tripped; only the probe admitted in half-open can move the
    /// circuit out of half-open.
    generation: u64,
}

impl CircuitBreakerInner {
    /// The state the breaker is in once the reset timeout is taken into
    /// account, without recording the transition.
    fn effective_state(&self, now: Instant) -> CircuitState {
        let reset = Duration::from_millis(self.config.reset_timeout_ms);
        match self.state {
            CircuitState::Open if now.duration_since(self.last_state_change) >= reset => {
                CircuitState::HalfOpen
            }
            CircuitState::HalfOpen => match self.probe_expired_at(now) {
                // The probe timed out: it failed then, so the circuit
                // reopened then.
                Some(failed_at) if now.duration_since(failed_at) < reset => CircuitState::Open,
                _ => CircuitState::HalfOpen,
            },
            state => state,
        }
    }

    /// If the running probe has run out of time, count it as failed: the
    /// circuit reopened at its deadline, under a new generation, so the
    /// probe's late result counts for nothing.
    fn expire_stale_probe(&mut self, now: Instant) {
        if self.state == CircuitState::HalfOpen {
            if let Some(failed_at) = self.probe_expired_at(now) {
                self.probe_in_flight = false;
                self.transition(CircuitState::Open, failed_at);
            }
        }
    }

    /// When the running probe ran out of time, if it has.
    fn probe_expired_at(&self, now: Instant) -> Option<Instant> {
        if !self.probe_in_flight {
            return None;
        }
        let deadline = self
            .probe_started
            .checked_add(self.config.probe_timeout())?;
        (now >= deadline).then_some(deadline)
    }

    /// Move to `state` and start a new generation.
    fn transition(&mut self, state: CircuitState, now: Instant) {
        self.state = state;
        self.last_state_change = now;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Records a failed call admitted in the closed state, opening the
    /// circuit once the threshold is reached within the window.
    fn record_failure(&mut self, admission: Admission, now: Instant) {
        // A call admitted under an earlier generation (before the
        // circuit tripped, or before a reset) no longer counts.
        if admission.is_probe
            || self.state != CircuitState::Closed
            || admission.generation != self.generation
        {
            return;
        }

        self.failures.push_back(now);

        // Drop failures outside the window, oldest first. When the
        // window reaches back past the start of the monotonic clock (a
        // very large `failure_window_ms`), every recorded failure is
        // inside it and nothing is dropped. Plain `Instant - Duration`
        // panics in that case.
        let window = Duration::from_millis(self.config.failure_window_ms);
        if let Some(window_start) = now.checked_sub(window) {
            while self
                .failures
                .front()
                .is_some_and(|&time| time < window_start)
            {
                let _expired = self.failures.pop_front();
            }
        }
        // Only the newest `failure_threshold` failures can ever matter,
        // so the list never grows past it.
        while self.failures.len() > self.config.failure_threshold.max(1) {
            let _oldest = self.failures.pop_front();
        }

        if self.failures.len() >= self.config.failure_threshold {
            self.transition(CircuitState::Open, now);
        }
    }

    /// Whether a call admitted under `admission` is the probe of the
    /// current half-open period.
    fn is_current_probe(&self, admission: Admission) -> bool {
        admission.is_probe
            && admission.generation == self.generation
            && self.state == CircuitState::HalfOpen
    }
}

/// How a call was let through: as the half-open probe or as a normal
/// call in the closed state, and under which generation.
#[derive(Clone, Copy)]
struct Admission {
    is_probe: bool,
    generation: u64,
}

/// Records a failure if the protected closure unwinds.
///
/// A panicking probe counts as a failed probe: the circuit reopens and
/// the reset timeout starts again. Without this the slot would stay
/// taken and the breaker would reject every later call. A panicking
/// call in the closed state counts as a failure like any `Err`.
struct ProbeGuard<'a> {
    inner: &'a Mutex<CircuitBreakerInner>,
    admission: Admission,
    armed: bool,
}

impl Drop for ProbeGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let mut inner = lock(self.inner);
            if self.admission.is_probe {
                inner.expire_stale_probe(Instant::now());
                if inner.is_current_probe(self.admission) {
                    inner.probe_in_flight = false;
                    inner.transition(CircuitState::Open, Instant::now());
                }
            } else {
                inner.record_failure(self.admission, Instant::now());
            }
        }
    }
}

/// Circuit breaker implementation to prevent cascading failures
///
/// The circuit breaker tracks failures and "trips" after a threshold is reached,
/// preventing further calls and allowing the system to recover.
pub struct CircuitBreaker {
    /// Shared with every `CircuitOpenError` this breaker returns, so the
    /// fail-fast path does not copy the name.
    name: Arc<str>,
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
            name: Arc::from(name.into()),
            inner: Arc::new(Mutex::new(CircuitBreakerInner {
                config,
                state: CircuitState::Closed,
                failures: VecDeque::new(),
                last_state_change: Instant::now(),
                probe_in_flight: false,
                probe_started: Instant::now(),
                generation: 0,
            })),
        }
    }

    /// Get the current state of the circuit breaker.
    ///
    /// An open circuit whose reset timeout has elapsed reports
    /// [`CircuitState::HalfOpen`]: the next [`execute`](Self::execute)
    /// call is admitted as the probe. Reading the state does not
    /// change it.
    ///
    /// A half-open probe that has run longer than the probe timeout
    /// (see [`CircuitBreakerConfig::probe_timeout_ms`]) counts as failed,
    /// so the state reads [`CircuitState::Open`] again until the next
    /// reset timeout elapses.
    pub fn state(&self) -> CircuitState {
        let inner = lock(&self.inner);
        inner.effective_state(Instant::now())
    }

    /// Whether a half-open probe is running right now, so every other
    /// call fails fast until it returns or times out.
    ///
    /// [`state`](Self::state) reports [`CircuitState::HalfOpen`] both
    /// while the probe runs and when the next call will be admitted as
    /// the probe; this tells the two apart.
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::CircuitBreaker;
    ///
    /// let breaker = CircuitBreaker::new("db");
    /// assert!(!breaker.is_probing());
    /// ```
    pub fn is_probing(&self) -> bool {
        let inner = lock(&self.inner);
        inner.state == CircuitState::HalfOpen
            && inner.probe_in_flight
            && inner.probe_expired_at(Instant::now()).is_none()
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
    ///
    /// Only the probe decides how the circuit leaves half-open. A call
    /// that was admitted while the circuit was still closed and finishes
    /// after it tripped does not affect the new state. A probe that runs
    /// longer than the probe timeout counts as failed: the circuit
    /// reopens, and the probe's own result, when it comes, is ignored.
    ///
    /// The closure runs on the calling thread and the breaker cannot
    /// interrupt it, so give it its own I/O timeouts. A panic in the
    /// closure counts as a failure.
    ///
    /// The closure's error is boxed into [`RecoveryResult`]; downcast it
    /// to tell a fail-fast rejection from the closure's own error.
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::{CircuitBreaker, CircuitOpenError};
    ///
    /// let breaker = CircuitBreaker::new("inventory-service");
    ///
    /// let value = breaker.execute(|| Ok::<u32, std::io::Error>(42));
    /// assert_eq!(value.unwrap(), 42);
    ///
    /// let failed = breaker.execute(|| Err::<u32, _>(std::io::Error::other("down")));
    /// let error = failed.unwrap_err();
    /// assert!(error.downcast_ref::<CircuitOpenError>().is_none());
    /// assert!(error.downcast_ref::<std::io::Error>().is_some());
    /// ```
    pub fn execute<F, T, E>(&self, f: F) -> RecoveryResult<T>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // First check if we can proceed with the call
        let admission = {
            let mut inner = lock(&self.inner);
            self.update_state(&mut inner);
            let is_probe = match inner.state {
                CircuitState::Closed => false,
                CircuitState::HalfOpen if !inner.probe_in_flight => {
                    inner.probe_in_flight = true;
                    inner.probe_started = Instant::now();
                    true
                }
                // Open, or half-open with the probe already running.
                _ => return Err(Box::new(CircuitOpenError::new(Arc::clone(&self.name)))),
            };
            Admission {
                is_probe,
                generation: inner.generation,
            }
        };

        // Armed for every call: a panicking probe reopens the circuit,
        // and a panicking call in the closed state counts as a failure.
        let mut guard = ProbeGuard {
            inner: &self.inner,
            admission,
            armed: true,
        };

        // Execute the function
        let outcome = f();
        guard.armed = false;

        match outcome {
            Ok(value) => {
                // Success, potentially reset circuit breaker
                self.on_success(admission);
                Ok(value)
            }
            Err(err) => {
                // Failure, record it and potentially trip circuit
                self.on_failure(admission);
                Err(Box::new(err))
            }
        }
    }

    /// Manually reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut inner = lock(&self.inner);
        inner.transition(CircuitState::Closed, Instant::now());
        inner.failures.clear();
        inner.probe_in_flight = false;
    }

    /// Called when an operation succeeds
    fn on_success(&self, admission: Admission) {
        // Only the half-open probe changes state on success, so a normal
        // call returns without taking the lock a second time.
        if !admission.is_probe {
            return;
        }
        let mut inner = lock(&self.inner);
        inner.expire_stale_probe(Instant::now());
        if inner.is_current_probe(admission) {
            // Successful test request, close the circuit
            inner.probe_in_flight = false;
            inner.failures.clear();
            inner.transition(CircuitState::Closed, Instant::now());
        }
    }

    /// Called when an operation fails
    fn on_failure(&self, admission: Admission) {
        let mut inner = lock(&self.inner);

        if admission.is_probe {
            inner.expire_stale_probe(Instant::now());
            if inner.is_current_probe(admission) {
                // Failed during test request, reopen the circuit
                inner.probe_in_flight = false;
                inner.transition(CircuitState::Open, Instant::now());
            }
            return;
        }

        inner.record_failure(admission, Instant::now());
    }

    /// Update the circuit state based on timing
    fn update_state(&self, inner: &mut CircuitBreakerInner) {
        let now = Instant::now();
        inner.expire_stale_probe(now);
        if inner.state == CircuitState::Open && inner.effective_state(now) == CircuitState::HalfOpen
        {
            // Reset timeout has elapsed, try half-open state
            inner.transition(CircuitState::HalfOpen, now);
            inner.probe_in_flight = false;
        }
    }
}

/// Error returned when circuit is open
#[derive(Debug)]
pub struct CircuitOpenError {
    circuit_name: Arc<str>,
}

impl CircuitOpenError {
    fn new(circuit_name: Arc<str>) -> Self {
        Self { circuit_name }
    }
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circuit '{}' is open, failing fast", self.circuit_name)
    }
}

impl std::error::Error for CircuitOpenError {}
