# Changelog

All notable changes to error-forge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Fixed

## [0.9.6] - 2025-08-17

### Added
- Async error handling support via `AsyncForgeError` trait
- Integration with `async-trait` for async error handling in async contexts
- New async utilities like `from_async_result` and `async_handle` methods
- Retry logic with async support in examples
- Comprehensive error recovery module with:
  - Backoff strategies (Exponential, Linear, Fixed) with configurable parameters and jitter
  - Circuit breaker pattern to prevent cascading failures
  - Retry policy framework with custom predicates and backoff support
  - `ForgeErrorRecovery` extension trait for all `ForgeError` types

## [0.9.0] - 2025-08-17

### Added
- Cross-platform CI workflow for testing on Linux, macOS, and Windows
- Clippy checks in CI to ensure code quality
- Windows-specific terminal color detection
- Automatic color disabling for non-interactive terminals
- Thread-safe error hook system using `OnceLock` instead of `static mut`
- Structured context support with `ContextError` type
- Error wrapping with context via `context()` and `with_context()` methods
- Error registry with support for error codes and documentation URLs
- Non-fatal error collection system with `ErrorCollector`
- Optional logging integration with support for:
  - Custom logging implementations
  - Integration with the `log` crate (optional)
  - Integration with the `tracing` crate (optional)
- Improved error chaining with source tracking

### Changed
- Replaced unsafe global mutable state with thread-safe alternatives

### Fixed
- Fixed Clippy warnings:
  - Removed needless doctest main function
  - Fixed collapsible match pattern in derive macros
  - Fixed unused variables and imports
- Addressed deprecated `PanicInfo` usage, replaced with `PanicHookInfo`
- Fixed thread safety test reliability issues
- Fixed `CodedError` state tracking to properly update and report fatal flags
  - Removed collapsible match patterns in error-forge-derive crate
  - Removed unnecessary `fn main()` from doctest examples

## [0.6.3] - 2025-08-17

Current release version.

## [0.5.0] - 2025 (Prior release)

Initial public release with core functionality.

### Added
- `define_errors!` macro for declarative error definitions
- `ForgeError` trait for unified error handling
- `group!` macro for error composition
- `#[derive(ModError)]` for simplified error implementation
- Console formatting with ANSI color support
- Error hook system with severity levels
- Zero external dependencies design

[Unreleased]: https://github.com/jamesgober/error-forge/compare/0.9.6...HEAD
[0.9.6]: https://github.com/jamesgober/error-forge/compare/0.9.0...v0.9.6
[0.9.0]: https://github.com/jamesgober/error-forge/compare/0.6.3...v0.9.0
[0.6.3]: https://github.com/jamesgober/error-forge/compare/0.6.1...0.6.3
