//! Error recovery patterns for handling transient errors
//!
//! This module provides various error recovery strategies that can be used to make
//! applications more resilient to transient failures.
//!
//! # Features
//!
//! - Backoff strategies for controlling retry timing
//! - Circuit breaker pattern to prevent cascading failures
//! - Retry policies for flexible retry behaviors
//! - `ForgeError`-aware retry executors for sync workloads
//!
//! # Examples
//!
//! ```rust,ignore
//! use error_forge::recovery::RetryPolicy;
//!
//! let policy = RetryPolicy::new_exponential()
//!     .with_max_retries(3);
//!
//! let result = policy.retry(|| {
//!     // Operation that might fail
//!     Ok(())
//! });
//! ```

mod backoff;
mod circuit_breaker;
mod forge_extensions;
mod retry;

pub use backoff::{Backoff, ExponentialBackoff, FixedBackoff, LinearBackoff};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState};
pub use forge_extensions::ForgeErrorRecovery;
pub use retry::{RetryExecutor, RetryPolicy};

/// Result type for recovery operations
pub type RecoveryResult<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
