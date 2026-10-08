use error_forge::recovery::{
    Backoff, CircuitBreaker, CircuitBreakerConfig, CircuitState, ExponentialBackoff, FixedBackoff,
    LinearBackoff, RetryPolicy,
};
use std::error::Error as StdError;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn test_exponential_backoff() {
    let backoff = ExponentialBackoff::new()
        .with_initial_delay(100)
        .with_max_delay(10000)
        .with_factor(2.0);

    // Test increasing delay pattern
    let delay1 = backoff.next_delay(0);
    let delay2 = backoff.next_delay(1);
    let delay3 = backoff.next_delay(2);

    assert_eq!(delay1.as_millis(), 100);
    assert_eq!(delay2.as_millis(), 200);
    assert_eq!(delay3.as_millis(), 400);

    // Test max delay cap
    let delay_max = backoff.next_delay(10);
    assert!(delay_max.as_millis() <= 10000);
}

#[test]
fn test_linear_backoff() {
    let backoff = LinearBackoff::new()
        .with_initial_delay(100)
        .with_increment(50)
        .with_max_delay(500);

    // Test increasing delay pattern
    let delay1 = backoff.next_delay(0);
    let delay2 = backoff.next_delay(1);
    let delay3 = backoff.next_delay(2);

    assert_eq!(delay1.as_millis(), 100);
    assert_eq!(delay2.as_millis(), 150);
    assert_eq!(delay3.as_millis(), 200);

    // Test max delay cap
    let delay_max = backoff.next_delay(10);
    assert_eq!(delay_max.as_millis(), 500);
}

#[test]
fn test_fixed_backoff() {
    let backoff = FixedBackoff::new(200);

    // All delays should be the same
    assert_eq!(backoff.next_delay(0).as_millis(), 200);
    assert_eq!(backoff.next_delay(1).as_millis(), 200);
    assert_eq!(backoff.next_delay(10).as_millis(), 200);
}

// Simple error type for testing
#[derive(Debug)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestError: {}", self.0)
    }
}

impl StdError for TestError {}

#[test]
fn test_circuit_breaker() {
    let circuit =
        CircuitBreaker::with_config("test-circuit", CircuitBreakerConfig::new(2, 1000, 100));

    // Initially closed
    assert_eq!(circuit.state(), CircuitState::Closed);

    // First failure
    let result = circuit.execute(|| -> Result<(), TestError> { Err(TestError("error")) });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Closed);

    // Second failure should trip the circuit
    let result = circuit.execute(|| -> Result<(), TestError> { Err(TestError("error")) });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Open);

    // Circuit is open, should fail fast
    let start = Instant::now();
    let result = circuit.execute(|| -> Result<(), TestError> {
        // This shouldn't execute
        std::thread::sleep(Duration::from_millis(50));
        Ok(())
    });
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_millis(10)); // Should fail fast

    // Wait for reset timeout - use a longer timeout for test stability
    std::thread::sleep(Duration::from_millis(200));

    // The first call after reset timeout should transition to half-open, then to closed if successful
    let result = circuit.execute(|| -> Result<(), TestError> { Ok(()) });
    assert!(result.is_ok());

    // After a successful call in half-open state, the circuit should close
    assert_eq!(circuit.state(), CircuitState::Closed);

    // Successful execution should close the circuit
    let result = circuit.execute(|| -> Result<(), TestError> { Ok(()) });
    assert!(result.is_ok());
    assert_eq!(circuit.state(), CircuitState::Closed);
}

#[test]
fn test_retry_policy() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let policy = RetryPolicy::new_fixed(10).with_max_retries(3);

    // Should succeed on the third attempt
    let result: Result<(), TestError> = policy.retry(|| {
        let current = counter_clone.fetch_add(1, Ordering::SeqCst);
        if current < 2 {
            Err(TestError("not ready"))
        } else {
            Ok(())
        }
    });

    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_with_predicate() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    // Create a retry executor with a predicate
    let executor = RetryPolicy::new_fixed(10)
        .with_max_retries(5)
        .executor::<TestError>()
        .with_retry_if(|err| err.0 == "retry");

    // Should not retry on "stop" error
    let result: Result<(), TestError> = executor.retry(|| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Err(TestError("stop"))
    });

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1); // Only one attempt

    // Reset counter
    counter.store(0, Ordering::SeqCst);

    // Should retry on "retry" error but eventually fail
    let result: Result<(), TestError> = executor.retry(|| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Err(TestError("retry"))
    });

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 6); // Initial + 5 retries
}

#[test]
fn test_circuit_breaker_window_longer_than_clock_does_not_panic() {
    // A failure window longer than the monotonic clock has been
    // running used to panic with "overflow when subtracting duration
    // from instant" on the first recorded failure.
    let circuit = CircuitBreaker::with_config(
        "long-window",
        CircuitBreakerConfig::new(2, u64::MAX, 60_000),
    );

    let result = circuit.execute(|| -> Result<(), TestError> { Err(TestError("error")) });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Closed);

    // Every failure stays inside an unbounded window, so the second
    // one still trips the circuit.
    let result = circuit.execute(|| -> Result<(), TestError> { Err(TestError("error")) });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Open);
}

#[test]
fn test_linear_backoff_saturates_instead_of_overflowing() {
    // `initial + attempt * increment` used to overflow: a panic in
    // debug builds and a wrapped, far too short delay in release.
    let backoff = LinearBackoff::new()
        .with_increment(u64::MAX)
        .with_max_delay(5_000);
    assert_eq!(backoff.next_delay(2), Duration::from_millis(5_000));

    let backoff = LinearBackoff::new()
        .with_initial_delay(u64::MAX)
        .with_increment(1)
        .with_max_delay(5_000);
    assert_eq!(backoff.next_delay(1), Duration::from_millis(5_000));
    assert_eq!(backoff.next_delay(usize::MAX), Duration::from_millis(5_000));
}

#[test]
fn test_exponential_backoff_large_attempt_stays_capped() {
    // `attempt as i32` used to wrap to a negative exponent, so a very
    // large attempt count produced a zero delay instead of the cap.
    let backoff = ExponentialBackoff::new()
        .with_initial_delay(100)
        .with_max_delay(10_000)
        .with_factor(2.0);
    let attempt = i32::MAX as usize + 1;
    assert_eq!(backoff.next_delay(attempt), Duration::from_millis(10_000));
    assert_eq!(
        backoff.next_delay(usize::MAX),
        Duration::from_millis(10_000)
    );
}

/// Trips `circuit` (threshold 1) and waits out its reset timeout.
fn trip_and_wait(circuit: &CircuitBreaker, reset_timeout: Duration) {
    let result = circuit.execute(|| -> Result<(), TestError> { Err(TestError("error")) });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Open);
    std::thread::sleep(reset_timeout + Duration::from_millis(150));
}

#[test]
fn test_circuit_breaker_state_reports_half_open_after_timeout() {
    // `state()` used to keep reporting `Open` after the reset timeout
    // until the next `execute` call happened to move the circuit on.
    let circuit = CircuitBreaker::with_config("state", CircuitBreakerConfig::new(1, 60_000, 100));
    trip_and_wait(&circuit, Duration::from_millis(100));

    assert_eq!(circuit.state(), CircuitState::HalfOpen);
    // Reading the state has no side effects: it stays half-open and
    // the next call is still admitted as the probe.
    assert_eq!(circuit.state(), CircuitState::HalfOpen);
    let result = circuit.execute(|| -> Result<u8, TestError> { Ok(7) });
    assert_eq!(result.unwrap(), 7);
    assert_eq!(circuit.state(), CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_half_open_admits_single_probe() {
    // Every caller used to be let through while the circuit was
    // half-open, not just one probe.
    use std::sync::mpsc;

    let circuit = CircuitBreaker::with_config("probe", CircuitBreakerConfig::new(1, 60_000, 100));
    trip_and_wait(&circuit, Duration::from_millis(100));

    let (probe_started_tx, probe_started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let second_ran = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        let probe = scope.spawn(|| {
            circuit.execute(move || -> Result<(), TestError> {
                probe_started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
        });

        probe_started_rx.recv().unwrap();
        assert_eq!(circuit.state(), CircuitState::HalfOpen);

        // The probe is still running, so this call must fail fast.
        let second = circuit.execute(|| -> Result<(), TestError> {
            second_ran.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        // Release the probe before asserting so a failure here cannot
        // leave the scoped thread blocked forever.
        release_tx.send(()).unwrap();
        let probe_result = probe.join().unwrap();

        let err = second.unwrap_err();
        assert!(err.is::<error_forge::recovery::CircuitOpenError>());
        assert_eq!(second_ran.load(Ordering::SeqCst), 0);
        assert!(probe_result.is_ok());
    });

    assert_eq!(circuit.state(), CircuitState::Closed);
    assert!(circuit
        .execute(|| -> Result<(), TestError> { Ok(()) })
        .is_ok());
}

#[test]
fn test_circuit_breaker_panicking_probe_reopens_circuit() {
    // A probe that panics counts as a failed probe. It must not leave
    // the breaker stuck half-open with the probe slot taken.
    let circuit = CircuitBreaker::with_config("panic", CircuitBreakerConfig::new(1, 60_000, 200));
    trip_and_wait(&circuit, Duration::from_millis(200));

    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = circuit.execute(|| -> Result<(), TestError> { panic!("probe panicked") });
    }));
    assert!(unwound.is_err());
    assert_eq!(circuit.state(), CircuitState::Open);

    std::thread::sleep(Duration::from_millis(350));
    assert_eq!(circuit.state(), CircuitState::HalfOpen);
    assert!(circuit
        .execute(|| -> Result<(), TestError> { Ok(()) })
        .is_ok());
    assert_eq!(circuit.state(), CircuitState::Closed);
}

#[test]
fn test_exponential_backoff_first_attempt_is_capped() {
    // Attempt 0 used to return the initial delay even when it was
    // larger than the configured maximum.
    let backoff = ExponentialBackoff::new()
        .with_initial_delay(5_000)
        .with_max_delay(1_000);
    assert_eq!(backoff.next_delay(0), Duration::from_millis(1_000));
    assert_eq!(backoff.next_delay(1), Duration::from_millis(1_000));
}

#[cfg(feature = "jitter")]
#[test]
fn test_exponential_backoff_jitter_respects_max_delay() {
    // Jitter used to scale the capped delay by up to 1.2, so a delay
    // at the cap could come out 20% above `max_delay`.
    let backoff = ExponentialBackoff::new()
        .with_initial_delay(1_000)
        .with_max_delay(1_000)
        .with_jitter(true);
    for attempt in 0..200 {
        let delay = backoff.next_delay(attempt % 8);
        assert!(delay <= Duration::from_millis(1_000), "{delay:?}");
        assert!(delay >= Duration::from_millis(800), "{delay:?}");
    }
}

#[cfg(feature = "jitter")]
#[test]
fn test_exponential_backoff_jitter_applies_to_first_attempt() {
    // Attempt 0 used to skip jitter and always return exactly the
    // initial delay.
    let backoff = ExponentialBackoff::new()
        .with_initial_delay(1_000)
        .with_max_delay(10_000)
        .with_jitter(true);
    let delays: Vec<Duration> = (0..200).map(|_| backoff.next_delay(0)).collect();
    assert!(delays
        .iter()
        .all(|d| *d >= Duration::from_millis(800) && *d < Duration::from_millis(1_200)));
    assert!(delays.iter().any(|d| *d != Duration::from_millis(1_000)));
}
