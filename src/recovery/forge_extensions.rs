use crate::error::ForgeError;
use crate::recovery::{CircuitBreaker, RetryPolicy};

/// Extension trait that adds recovery capabilities to ForgeError types
pub trait ForgeErrorRecovery: ForgeError {
    /// Create a retry policy optimized for this error type
    fn create_retry_policy(&self, max_retries: usize) -> RetryPolicy {
        RetryPolicy::new_exponential().with_max_retries(max_retries)
    }

    /// Execute a fallible operation with retries if this error type is retryable.
    ///
    /// When `self.is_retryable()` is `false` the operation runs exactly
    /// once and its result is returned as is. Otherwise it is retried up
    /// to `max_retries` times with exponential backoff, for as long as
    /// the error it returns is itself retryable.
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::{recovery::ForgeErrorRecovery, AppError};
    ///
    /// // `AppError::config` is not retryable, so the operation runs once.
    /// let template = AppError::config("bad config");
    /// let mut calls = 0;
    /// let result: Result<(), AppError> = template.retry(3, || {
    ///     calls += 1;
    ///     Err(AppError::network("db", None))
    /// });
    /// assert!(result.is_err());
    /// assert_eq!(calls, 1);
    /// ```
    fn retry<F, T, E>(&self, max_retries: usize, operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: ForgeError,
    {
        let max_retries = if self.is_retryable() { max_retries } else { 0 };
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
