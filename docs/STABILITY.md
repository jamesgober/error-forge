# error-forge — Stability and SemVer Policy

> Formal stability contract for the `1.x` line. Binding — any
> release that violates it is a bug.

## Headline guarantee

**Every public symbol in `1.0.0` is locked for the entire `1.x`
line.** Signatures, observable behaviour, and the documented
contract of every method cannot change without a major version
bump to `2.0.0`.

This applies to:

- Every symbol re-exported by `mod error_forge` (the crate root)
  and the per-module re-exports under
  `error_forge::{console_theme, context, collector, error,
  logging, macros, recovery, registry}` (and `async_error` /
  `async_error_impl` under `feature = "async"`).
- The trait implementations on the public types
  (`std::error::Error`, `std::fmt::Display`, `std::fmt::Debug`,
  `Clone`, `From<T>`).
- The documented behavioural contracts of every `pub fn`,
  including:
  - Panic-vs-error semantics.
  - Auto-firing of `call_error_hook` from `AppError` / `define_errors!`
    constructors.
  - `ForgeError::register` calling the same hook (and the
    documented double-fire if both paths are exercised).
  - `define_errors!` generating lowercase constructors and
    `Display` / `Error` / `ForgeError` impls.
  - `group!` requiring each wrapped type to implement
    `ForgeError`.

## Version policy

The `1.x` line follows [Semantic Versioning 2.0.0](https://semver.org/).

### Patch releases (`1.x.Y`)

May contain:

- Bug fixes that do not change observable output for any
  documented input.
- Documentation improvements.
- Internal performance work that does not move any public
  surface.
- Test additions.
- Internal dependency updates within their compatible range.

May NOT contain:

- New public items.
- Behavioural changes (including panic-message wording when
  callers may parse it, hook-firing semantics, etc.).
- MSRV bumps.

### Minor releases (`1.Y.0`)

May contain everything a patch release may contain, plus:

- New public items (functions, methods, types, modules) so long
  as they are pure additions — no signature change to any
  existing symbol.
- New optional cargo features that default to off, or default-on
  features that activate purely additive code paths.
- New variants on `#[non_exhaustive]` enums (`ErrorLevel`,
  `CircuitState`) or new fields on `#[non_exhaustive]` structs
  (`ErrorContext`, `ErrorCodeInfo`, `CodedError`,
  `ContextError`, `CircuitBreakerConfig`).
- MSRV bumps. Each MSRV bump is a minor-version bump minimum
  and is called out in the CHANGELOG.

### Major releases (`2.0.0`)

Required for any change that violates the above. Specifically,
any of the following requires `2.0.0`:

- Removal or rename of any public symbol.
- A signature change to any existing public symbol (including
  loosening or tightening the panicking conditions in a
  user-observable way).
- Removing a `#[non_exhaustive]` marker (which would let
  external code resume struct-literal construction —
  considered a contract narrowing, not a widening).
- Changing the default-feature set
  (default features are `[]` in `1.0.0`; adding a default
  feature, or making a default feature opt-in, is breaking).
- Adding a runtime dependency that is not gated behind an
  opt-in feature.
- A change that breaks the documented `0.9.x → 1.0.0`
  migration in [`docs/migration.md`](migration.md).

## Deprecation policy

A symbol marked `#[deprecated]` in a `1.x.Y` release:

- Remains callable for the entire `1.x` line.
- Continues to behave per its documented contract.
- May only be removed in a `2.0.0` release.

Items deprecated in `1.0.0`:

- `error_forge::macros::register_error_hook` (non-`try`
  variant). Use `try_register_error_hook` instead. The non-`try`
  variant silently discards `Err`; the `try_` variant returns it.
- `error_forge::Result<T>` (the type alias). Use
  `error_forge::AppResult<T>` instead. The unqualified `Result`
  name shadows `std::result::Result` in `use error_forge::*`
  glob imports.

Both deprecated names remain callable through the entire `1.x`
line and are removed only in `2.0.0`.

## Panic safety

The crate panics in exactly one place by design: hook
registration is per-process, and a corrupt registry state would
be a bug. Specifically:

- `AppError` and `define_errors!` constructors never panic
  through normal use.
- `ForgeError` default methods never panic.
- `register_error_hook` / `try_register_error_hook` never panic
  on duplicate registration; the `try_` variant returns
  `Err("Error hook already registered")` instead.
- `ConsoleTheme` formatting never panics; ANSI escapes are
  static strings.
- `recovery::CircuitBreaker` uses `parking_lot::Mutex`, which
  does not poison; `lock()` cannot panic in our usage.
- `recovery::RetryExecutor::retry` propagates the inner
  operation's panic if the closure panics (matches what every
  retry library does).

## What is NOT covered

The stability contract does NOT cover:

- **Exact wording of error messages and panic-payload messages.**
  The error *kind* (visible through `ForgeError::kind()`), the
  `Display` chain via `Error::source`, and the prefix
  `"[CODE] "` on `CodedError`-formatted output are part of the
  contract; the human-readable text following them is not.
- **Internal types and modules.** Any item not re-exported
  through `lib.rs` (`pastey`, internal helpers in
  `error.rs::panic_payload_to_listener_error`, the private
  `recovery::retry::{BackoffStrategy, BackoffType, RetryPredicate}`)
  is internal and may move or change between minor releases.
- **Performance characteristics.** A `1.x.Y` may make any
  operation faster or slower than `1.0.0`. Documented benchmark
  numbers (where they exist) are illustrative, not contractual.

## Dependencies

The following runtime dependencies are sealed for the `1.x` line:

- `thiserror` (always-on, error-handling support).
- `pastey` (always-on, macro support — drop-in fork of the
  archived `paste`).
- `parking_lot` (always-on, non-poisoning `Mutex` used by
  `recovery::CircuitBreaker`).
- `error-forge-derive` (optional, gated on `derive` feature).
- `serde` (optional, gated on `serde` feature).
- `log` (optional, gated on `log` feature).
- `tracing` (optional, gated on `tracing` feature).
- `async-trait` (optional, gated on `async` feature).
- `rand` (optional, gated on `jitter` feature).

Adding a new runtime dependency requires a `2.0.0` bump.
Removing any of the always-on deps requires a `2.0.0` bump.
Adding new optional dependencies behind a new opt-in feature is
a minor-version bump.

## MSRV

`1.0.0` ships with MSRV `1.81.0`. Any change to the MSRV is a
minor-version bump minimum.

The `1.81` floor is driven by three independent constraints:

1. `io::Error::other` (used by `AppError::filesystem`) is stable
   since `1.74`.
2. The committed `Cargo.lock` is format v4, which Cargo cannot
   parse on toolchains older than `1.78`.
3. Clippy's `incompatible_msrv` lint flags additional `1.81`
   items in the current source.

`1.81` is the conservative floor satisfying all three.

## Feature flags

The default feature set is `[]` — every optional feature is
opt-in. The full feature list is documented in
[`docs/API.md`](API.md).

Removing a feature from defaults is fine (no change since the
default is empty). Adding a default feature is breaking.
Renaming a feature is breaking.

## Reporting a stability break

If you encounter what looks like a `1.x` stability break:

1. Run the failing case against the latest patch release of
   `1.x` to confirm reproducibility.
2. Capture the exact API call sequence and observed-vs-expected
   behaviour.
3. Open an issue at
   <https://github.com/jamesgober/error-forge/issues> with the
   repro.

Stability breaks are bugs and are fixed in the next patch
release.
