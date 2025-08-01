//! # Error Forge
//! 
//! A high-performance, flexible Rust error framework for defining, formatting, chaining, 
//! and managing rich custom errors across large-scale applications.
//!
//! ## Overview
//!
//! Error Forge provides a comprehensive solution for error handling in Rust applications,
//! focusing on performance, flexibility, and developer ergonomics. It enables defining 
//! structured error types with minimal boilerplate, making error handling more efficient
//! and maintainable.
//!
//! ## Key Features
//!
//! - **`define_errors!` macro** - Create rich error enums with minimal boilerplate
//! - **`#[derive(ModError)]` proc-macro** - "Lazy mode" errors with attribute-based configuration
//! - **`group!` macro** - Compose multi-error enums with automatic `From` conversions
//! - **Console theming** - ANSI-colored errors for CLI applications
//! - **Built-in panic hook** - Enhanced panic formatting
//! - **ForgeError trait** - Unified interface for error handling
//! - **Serialization support** - Optional serde integration
//!
//! ## Quick Start
//!
//! ```ignore
//! use error_forge::{define_errors, ForgeError};
//! 
//! // Define our error type
//! define_errors! {
//!     #[derive(Debug)]
//!     pub enum AppError {
//!         #[error(display = "Configuration error: {message}")]
//!         #[kind(Config, retryable = false, status = 500)]
//!         Config { message: String },
//!         
//!         #[error(display = "Database error: {message}")]
//!         #[kind(Database, retryable = true, status = 503)]
//!         Database { message: String },
//!     }
//! }
//!
//! // Use the error type
//! fn main() -> Result<(), error_forge::AppError> {
//!     if cfg!(debug_assertions) {
//!         return Err(error_forge::AppError::config("Missing configuration"));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Enhanced Error Reporting
//!
//! ```rust
//! // Import the predefined AppError type from the library
//! use error_forge::{console_theme::print_error, AppError};
//!
//! fn main() {
//!     let error = AppError::config("Database connection failed");
//!     print_error(&error);  // Displays a nicely formatted error message
//! }
//! ```
pub mod error;
pub mod macros;
pub mod group_macro;
pub mod console_theme;

// Re-export core types and traits
pub use crate::error::{ForgeError, Result, AppError};
pub use crate::console_theme::{ConsoleTheme, print_error, install_panic_hook};

#[cfg(feature = "serde")]
extern crate serde;

// Re-export macros for convenient use
#[allow(unused_imports)]
pub use crate::group_macro::*;

// Optional re-export of the proc macro
#[cfg(feature = "derive")]
pub use error_forge_derive::*;

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