use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::recovery::RecoveryResult;

/// Represents the current state of a circuit breaker
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed and operations are allowed to execute
    Closed,
    
    /// Circuit is open and operations will fail fast
    Open,
    
    /// Circuit is partially open, allowing a test request
    HalfOpen,
}

/// Configuration for a circuit breaker
#[derive(Clone)]
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

struct CircuitBreakerInner {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failures: Vec<Instant>,
    last_state_change: Instant,
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
            })),
        }
    }
    
    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.lock().unwrap();
        inner.state
    }
    
    /// Get the name of the circuit breaker
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Execute a function protected by the circuit breaker
    pub fn execute<F, T, E>(&self, f: F) -> RecoveryResult<T>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // First check if we can proceed with the call
        let can_proceed = {
            let mut inner = self.inner.lock().unwrap();
            self.update_state(&mut inner);
            inner.state != CircuitState::Open
        };
        
        // If circuit is open, fail fast
        if !can_proceed {
            return Err(Box::new(CircuitOpenError::new(&self.name)));
        }
        
        // Execute the function
        match f() {
            Ok(value) => {
                // Success, potentially reset circuit breaker
                self.on_success();
                Ok(value)
            }
            Err(err) => {
                // Failure, record it and potentially trip circuit
                self.on_failure();
                Err(Box::new(err))
            }
        }
    }
    
    /// Manually reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.state = CircuitState::Closed;
        inner.failures.clear();
        inner.last_state_change = Instant::now();
    }
    
    /// Called when an operation succeeds
    fn on_success(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.state == CircuitState::HalfOpen {
            // Successful test request, close the circuit
            inner.state = CircuitState::Closed;
            inner.failures.clear();
            inner.last_state_change = Instant::now();
        }
    }
    
    /// Called when an operation fails
    fn on_failure(&self) {
        let mut inner = self.inner.lock().unwrap();
        
        if inner.state == CircuitState::HalfOpen {
            // Failed during test request, reopen the circuit
            inner.state = CircuitState::Open;
            inner.last_state_change = Instant::now();
            return;
        }
        
        // Add the failure
        let now = Instant::now();
        inner.failures.push(now);
        
        // Remove old failures outside the window
        let window_start = now - Duration::from_millis(inner.config.failure_window_ms);
        inner.failures.retain(|&time| time >= window_start);
        
        // Check if threshold is reached
        if inner.state == CircuitState::Closed && 
           inner.failures.len() >= inner.config.failure_threshold {
            // Trip the circuit
            inner.state = CircuitState::Open;
            inner.last_state_change = now;
        }
    }
    
    /// Update the circuit state based on timing
    fn update_state(&self, inner: &mut CircuitBreakerInner) {
        if inner.state == CircuitState::Open {
            let now = Instant::now();
            let elapsed = now.duration_since(inner.last_state_change);
            
            if elapsed >= Duration::from_millis(inner.config.reset_timeout_ms) {
                // Reset timeout has elapsed, try half-open state
                inner.state = CircuitState::HalfOpen;
                inner.last_state_change = now;
            }
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
