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
//!
//! # Examples
//!
//! ```rust,ignore
//! use error_forge::recovery::{ExponentialBackoff, RetryPolicy};
//!
//! let backoff = ExponentialBackoff::new()
//!     .with_initial_delay(100)
//!     .with_max_delay(10000)
//!     .with_jitter(true);
//!
//! let policy = RetryPolicy::new(backoff)
//!     .with_max_retries(3);
//!
//! let result = policy.retry(|| {
//!     // Operation that might fail
//!     Ok(())
//! });
//! ```

mod backoff;
mod circuit_breaker;
mod retry;
mod forge_extensions;

pub use backoff::{Backoff, ExponentialBackoff, LinearBackoff, FixedBackoff};
pub use circuit_breaker::{CircuitBreaker, CircuitState, CircuitOpenError, CircuitBreakerConfig};
pub use retry::{RetryPolicy, RetryExecutor};
pub use forge_extensions::ForgeErrorRecovery;

/// Result type for recovery operations
pub type RecoveryResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
