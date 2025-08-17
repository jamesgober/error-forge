use crate::error::ForgeError;
use crate::recovery::{RetryPolicy, CircuitBreaker};

/// Extension trait that adds recovery capabilities to ForgeError types
pub trait ForgeErrorRecovery: ForgeError {
    /// Create a retry policy optimized for this error type
    fn create_retry_policy(&self, max_retries: usize) -> RetryPolicy {
        RetryPolicy::new_exponential()
            .with_max_retries(max_retries)
    }

    /// Execute a fallible operation with retries if this error type is retryable
    fn retry<F, T, E>(&self, max_retries: usize, operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: ForgeError,
    {
        let policy = self.create_retry_policy(max_retries);
        policy.forge_executor().retry(operation)
    }

    /// Create a circuit breaker for operations that might result in this error type
    fn create_circuit_breaker(&self, name: impl Into<String>) -> CircuitBreaker {
        CircuitBreaker::new(name)
    }
}

// Implement the extension trait for all ForgeError types
impl<T: ForgeError> ForgeErrorRecovery for T {}
