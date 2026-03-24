//! # Error Forge
//!
//! Error Forge is a pragmatic Rust error-handling crate for applications that need
//! structured metadata, readable output, and operational hooks without forcing a
//! single application architecture.
//!
//! It provides:
//!
//! - `ForgeError` for stable error metadata
//! - `AppError` for immediate use in small and medium projects
//! - `define_errors!` for declarative custom enums
//! - `group!` for coarse-grained composition
//! - optional derive support with `#[derive(ModError)]`
//! - context wrapping, error codes, collectors, logging hooks, and console formatting
//! - synchronous retry and circuit-breaker helpers in `recovery`
//!
//! ## Quick Start
//!
//! ```ignore
//! use error_forge::{define_errors, ForgeError};
//!
//! define_errors! {
//!     pub enum ServiceError {
//!         #[error(display = "Configuration error: {message}", message)]
//!         #[kind(Config, status = 500)]
//!         Config { message: String },
//!
//!         #[error(display = "Request to {endpoint} failed", endpoint)]
//!         #[kind(Network, retryable = true, status = 503)]
//!         Network { endpoint: String },
//!     }
//! }
//!
//! let error = ServiceError::config("Missing DATABASE_URL".to_string());
//! assert_eq!(error.kind(), "Config");
//! ```
//!
//! ## Built-in Formatting
//!
//! ```rust
//! use error_forge::{console_theme::print_error, AppError};
//!
//! let error = AppError::config("Database connection failed");
//! print_error(&error);
//! ```
pub mod collector;
pub mod console_theme;
pub mod context;
pub mod error;
pub mod group_macro;
pub mod logging;
pub mod macros;
pub mod recovery;
pub mod registry;

#[cfg(feature = "async")]
pub mod async_error;
#[cfg(feature = "async")]
pub mod async_error_impl;

// Re-export core types and traits
pub use crate::console_theme::{install_panic_hook, print_error, ConsoleTheme};
pub use crate::error::{AppError, ForgeError, Result};

// Re-export context module
pub use crate::context::{ContextError, ResultExt};

// Re-export registry module
pub use crate::registry::{
    register_error_code, CodedError, ErrorCodeInfo, ErrorRegistry, WithErrorCode,
};

// Re-export collector module
pub use crate::collector::{CollectError, ErrorCollector};

// Re-export logging module
pub use crate::logging::{log_error, logger, register_logger, ErrorLogger};

// Re-export async module (when enabled)
#[cfg(feature = "async")]
pub use crate::async_error::{AsyncForgeError, AsyncResult};

#[cfg(feature = "serde")]
extern crate serde;

// Re-export macros for convenient use
#[allow(unused_imports)]
pub use crate::macros::*;

// Optional re-export of the proc macro
#[cfg(feature = "derive")]
pub use error_forge_derive::*;

// Extension methods are implemented in error.rs

#[cfg(test)]
mod tests {
    use crate::ForgeError;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_error_display() {
        let err = crate::error::AppError::config("Test error");
        assert!(err.to_string().contains("Test error"));
    }

    #[test]
    fn test_error_kind() {
        let err = crate::error::AppError::config("Test error");
        assert_eq!(err.kind(), "Config");
    }
}
