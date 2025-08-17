# Changelog

All notable changes to error-forge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Cross-platform CI workflow for testing on Linux, macOS, and Windows
- Clippy checks in CI to ensure code quality

### Fixed
- Fixed Clippy warnings:
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

[Unreleased]: https://github.com/jamesgober/error-forge/compare/0.6.3...HEAD
[0.7.0]: https://github.com/jamesgober/error-forge/compare/0.6.3...v0.7.0
[0.6.3]: https://github.com/jamesgober/error-forge/compare/0.6.1...0.6.3
1