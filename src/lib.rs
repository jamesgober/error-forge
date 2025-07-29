//! # Error Forge
//! 
//! A Rust library for creating and managing custom error types with ease.
//!
//! This library provides a simple and ergonomic way to define custom error types
//! using the `thiserror` crate. It allows developers to create structured error types
//! with minimal boilerplate, making error handling in Rust applications more efficient
//! and readable.
//! 
//! ## Features
//! - Define custom error types with `thiserror`
//! - Use `Result<T, E>` type alias for simplified error handling
//! - Provides a unified error handling mechanism for various scenarios
//! - Supports serialization and deserialization of errors
//! - Integrates seamlessly with the Rust ecosystem
pub mod error;
pub mod macros;

#[cfg(feature = "serde")]
extern crate serde;

#[cfg(test)]
mod tests {
	// Add your tests here
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}