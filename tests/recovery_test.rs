use error_forge::recovery::{
    Backoff, 
    ExponentialBackoff, 
    LinearBackoff, 
    FixedBackoff,
    CircuitBreaker,
    CircuitBreakerConfig, 
    CircuitState,
    RetryPolicy
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
    let circuit = CircuitBreaker::with_config(
        "test-circuit", 
        CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window_ms: 1000,
            reset_timeout_ms: 100,
        }
    );
    
    // Initially closed
    assert_eq!(circuit.state(), CircuitState::Closed);
    
    // First failure
    let result = circuit.execute(|| -> Result<(), TestError> {
        Err(TestError("error"))
    });
    assert!(result.is_err());
    assert_eq!(circuit.state(), CircuitState::Closed);
    
    // Second failure should trip the circuit
    let result = circuit.execute(|| -> Result<(), TestError> {
        Err(TestError("error"))
    });
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
    let result = circuit.execute(|| -> Result<(), TestError> {
        Ok(())
    });
    assert!(result.is_ok());
    assert_eq!(circuit.state(), CircuitState::Closed);
}

#[test]
fn test_retry_policy() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    let policy = RetryPolicy::new_fixed(10)
        .with_max_retries(3);
    
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
