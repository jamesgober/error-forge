# Changelog

All notable changes to error-forge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.8] - 2026-05-18

Cleanup release. Drops the deprecated `atty` dependency, prunes a dead feature flag, tightens `ConsoleTheme` allocations, resolves three of the four ignored doctests, and rewrites the `Cargo.toml` description without banned terms. No public API changes. Sets `rust-version` in `Cargo.toml` for the first time and ships the corresponding MSRV-verification CI job.

### Added
- `rust-version = "1.81"` in `Cargo.toml`. MSRV is now declared explicitly and CI runs a dedicated MSRV-pin job verifying the crate builds against the exact `1.81.0` toolchain. The floor is driven by three independent constraints: `io::Error::other` (used by `AppError::filesystem` builders) is stable since 1.74; clippy's `incompatible_msrv` lint flags additional 1.81 items in the current source; and the committed `Cargo.lock` is format v4, which Cargo cannot parse on toolchains older than 1.78. `1.81` is the conservative floor that satisfies all three and matches `mod-events` in the workspace for consistency.
- `MSRV` badge in `README.md`.
- `cargo audit` CI job that runs the RustSec advisory database against every push.
- `.dev/release/v0.9.8.md` release note describing the full cleanup scope.

### Changed
- **`Cargo.toml` description rewritten without banned terms** (`comprehensive`, `robust`). The new text reads: "Pragmatic Rust error-handling framework with stable error metadata, contextual diagnostics, optional async support, and synchronous recovery primitives (retry, circuit-breaker, backoff). Optional `#[derive(ModError)]`, declarative `define_errors!`, and feature-gated logging / tracing / serde adapters."
- **`ConsoleTheme` colour fields are now `&'static str`** instead of `String`. Saves 8 heap allocations per `ConsoleTheme::default()` / `with_colors()` / `plain()` construction. `with_colors()` and `plain()` are now `const fn`.
- **`print_error` caches its default `ConsoleTheme`** in a process-wide `OnceLock`. The terminal-capability check runs at most once regardless of how many errors are printed.
- **`ConsoleTheme::format_error` writes into a single `String` buffer** using `std::fmt::Write` instead of allocating an intermediate string per section. Allocations drop from ~6 to 1 per error formatted.
- **`terminal_supports_ansi()` decision is now cached in a `OnceLock`** on every platform (the Windows branch already did this; the Unix branch did the env-var checks on every call).
- README `MSRV` badge added; install snippet bumped from `0.9.7` to `0.9.8` and now documents the `1.70` MSRV. Per-feature install lines unchanged.
- `lib.rs` doctests for the crate-level Quick Start and Built-in Formatting blocks no longer carry the `ignore` annotation — they compile and run as part of `cargo test --all-features`.
- `recovery/mod.rs` module-level doctest now compiles and runs (the closure return type is declared so the retry policy can type-check the loop).
- `async_error.rs::AsyncForgeError` doctest is now gated behind a hidden `#[cfg(feature = "async")]` block and runs under `cargo test --all-features`.
- `group_macro.rs::group` doctest remains `ignore`d but now documents two known limitations explicitly (a macro-parse ambiguity in `@with_impl` and a broken `ForgeError` delegation pattern). Both are scheduled to be fixed in `1.0`.
- CHANGELOG `[0.6.3]` entry no longer carries the "Current release version" marker (`0.6.3` is well past current).

### Fixed
- **Replaced `atty` with `std::io::IsTerminal`.** `atty` is unmaintained and on the RustSec radar (RUSTSEC-2021-0145 — unaligned-read race). `std::io::IsTerminal` (stable since Rust 1.70) is the canonical replacement and lets `error-forge` drop the dependency entirely. Affects `console_theme::terminal_supports_ansi`; behaviour is unchanged from the user's perspective.

### Removed
- **`atty` dependency.** No longer needed after the `IsTerminal` migration.
- **`once_cell` dependency and the `thread-safety` feature flag** that gated it. `once_cell::sync::OnceCell` was never imported anywhere in the crate — every `OnceLock` usage was already `std::sync::OnceLock` (stable since `1.70`). The `thread-safety` feature was dead code; enabling it added a dep with no behaviour change.
- **`extern crate serde;`** from `lib.rs`. This Rust 2015 idiom is unnecessary in 2021 edition and provided no functional value.
- **Placeholder `it_works` test from `lib.rs`** (`assert_eq!(2 + 2, 4);`). The two remaining inline tests (`test_error_display`, `test_error_kind`) carry actual coverage; placeholder cruft is gone.

### Compatibility
- No public API changes from `0.9.7`. The `Cargo.toml` `version` bump, `rust-version` declaration, and dependency-table shrinkage are the only consumer-visible deltas. Existing source compiles unchanged.
- Removing the `thread-safety` feature is technically a breaking change for any consumer that lists it explicitly, but the feature gated zero behaviour, so no observable consequences.
- MSRV is now formally `1.81` (driven by `io::Error::other` + lockfile format v4 + clippy `incompatible_msrv` lint findings; see the `Added` section for the full rationale). A dedicated MSRV CI job verifies this on every push.

## [0.9.7] - 2026-03-24

### Added
- Added `try_register_error_hook(...)` so applications can detect duplicate hook registration explicitly.
- Added regression coverage for `define_errors!`, `#[derive(ModError)]`, coded-error overrides, and feature-gated behavior.

### Changed
- Rewrote the public README and API reference to match the real crate surface, supported attributes, and current recovery model.
- Updated crate-level and recovery module documentation examples to reflect the current API.
- Made `define_errors!` default `fatal` behavior consistent with `ForgeError` and `AppError` by defaulting to `false`.

### Fixed
- Fixed `CodedError::with_retryable(...)` and `CodedError::with_status(...)` so they now apply real per-instance overrides.
- Fixed `define_errors!` helper expansion bugs affecting tag parsing, display formatting, and source chaining.
- Fixed the derive macro so list-style attributes such as `#[error_display("...")]` and `#[error_http_status(...)]` are honored.
- Fixed strict lint failures so `cargo clippy --all-targets --all-features -- -D warnings` now passes cleanly.
- Removed stale documentation that described unsupported APIs or outdated examples.


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

Historical maintenance release; superseded by `0.9.0`.

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

[Unreleased]: https://github.com/jamesgober/error-forge/compare/v0.9.8...HEAD
[0.9.8]: https://github.com/jamesgober/error-forge/compare/v0.9.7...v0.9.8
[0.9.7]: https://github.com/jamesgober/error-forge/compare/0.9.6...v0.9.7
[0.9.6]: https://github.com/jamesgober/error-forge/compare/0.9.0...v0.9.6
[0.9.0]: https://github.com/jamesgober/error-forge/compare/0.6.3...v0.9.0
[0.6.3]: https://github.com/jamesgober/error-forge/compare/0.6.1...0.6.3
