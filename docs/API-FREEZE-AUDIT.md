# error-forge — 1.0.0 API Freeze Audit

> Manifest of every public symbol in `error-forge 1.0.0`. Locked
> under SemVer for the `1.x` line. See
> [`docs/STABILITY.md`](STABILITY.md) for the binding policy.

## Status

This document is the public-surface manifest for `1.0.0`. Every
symbol below is part of the SemVer contract.

`1.0.0` is **not** a strict superset of `0.9.x`. Breaking changes
at the freeze boundary:

1. **`group!` macro requires `: ForgeError` on each wrapped type.**
   In `0.9.x`, `group!` accepted any wrapped type but the
   generated `ForgeError` impl was silently incorrect — every
   wrapped variant returned default fallback values regardless of
   the inner type's `ForgeError` impl. `1.0` rewrites the
   delegation to call trait methods directly, which the compiler
   verifies via the `: ForgeError` bound.
2. **`AsyncForgeError::async_handle` is now optional.** Was a
   required trait method in `0.9.x` (with a no-op stub on
   `AppError`). The trait now provides a no-op default body; the
   `AppError` stub is removed. Implementors override only if they
   want async behaviour.
3. **`AppError::with_async_context` removed.** The method took a
   `FnOnce() -> C` and synchronously called it — there was
   nothing async about it. Callers wanting deferred-context
   evaluation use `ResultExt::with_context` instead.
4. **`#[non_exhaustive]` added** to `ErrorLevel`, `ErrorContext`,
   `ErrorCodeInfo`, `CodedError`, `ContextError`,
   `CircuitBreakerConfig`, and `CircuitState`. External code can
   no longer construct these via struct-literal / unguarded
   `match` syntax; use the documented constructors (`new`,
   `Default::default()` + mutation, or appropriate API
   surfaces) and add `_ =>` arms to exhaustive matches.
5. **`CodedError::new` no longer auto-registers** the code in
   the global registry. Pre-register codes at startup with
   `register_error_code` if you want documentation URLs or
   per-code retryability metadata.
6. **Hook callback type widened** from `fn(ErrorContext)` to
   `Box<dyn Fn(ErrorContext<'_>) + Send + Sync + 'static>`. The
   `register_error_hook` / `try_register_error_hook` signatures
   now accept `impl Fn(ErrorContext<'_>) + Send + Sync + 'static`
   — closures capturing thread-safe state work as the callback.
7. **`pub use crate::macros::*;` narrowed to explicit re-exports.**
   The wildcard re-exported five items
   (`ErrorContext`, `ErrorLevel`, `ErrorSource`,
   `register_error_hook`, `try_register_error_hook`,
   `call_error_hook`). `1.0` lists the first five explicitly and
   drops `call_error_hook` (it was marked `#[doc(hidden)]` in
   `0.9.x` so this is mostly bookkeeping — the symbol is still
   reachable via `error_forge::macros::call_error_hook`).

### Deprecations (still callable in 1.x)

- `error_forge::macros::register_error_hook` (non-`try` variant).
- `error_forge::Result<T>` type alias (shadows
  `std::result::Result`). Use `error_forge::AppResult<T>`.

Both deprecations stay callable through the entire `1.x` line.

---

## Public surface

### Always available (no feature flag required)

#### Top-level re-exports

```rust
// Core
pub use crate::error::{AppError, AppResult, ForgeError};
#[allow(deprecated)]
pub use crate::error::Result;   // deprecated alias for AppResult

// Console formatting
pub use crate::console_theme::{install_panic_hook, print_error, ConsoleTheme};

// Context
pub use crate::context::{ContextError, ResultExt};

// Registry
pub use crate::registry::{
    register_error_code, CodedError, ErrorCodeInfo, ErrorRegistry, WithErrorCode,
};

// Collector
pub use crate::collector::{CollectError, ErrorCollector};

// Logging
pub use crate::logging::{log_error, logger, register_logger, ErrorLogger};

// Hook system (explicit re-exports — no wildcard)
#[allow(deprecated)]
pub use crate::macros::{
    register_error_hook, try_register_error_hook,
    ErrorContext, ErrorLevel, ErrorSource,
};
```

#### Module `error_forge`

```rust
pub mod collector;
pub mod console_theme;
pub mod context;
pub mod error;
pub mod group_macro;
pub mod logging;
pub mod macros;
pub mod recovery;
pub mod registry;
```

#### Trait — `ForgeError`

```rust
pub trait ForgeError: std::error::Error + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool { false }
    fn is_fatal(&self) -> bool { false }
    fn status_code(&self) -> u16 { 500 }
    fn exit_code(&self) -> i32 { 1 }
    fn user_message(&self) -> String;       // default: self.to_string()
    fn dev_message(&self) -> String;        // default: format!("[{}] {}", self.kind(), self)
    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> { None }
    fn register(&self);                     // calls call_error_hook
}
```

#### Enum — `AppError`

```rust
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AppError {
    Config { message: String, retryable: bool, fatal: bool, status: u16 },
    Filesystem { path: Option<PathBuf>, source: io::Error, retryable: bool, fatal: bool, status: u16 },
    Network { endpoint: String, source: Option<Box<dyn StdError + Send + Sync>>, retryable: bool, fatal: bool, status: u16 },
    Other { message: String, retryable: bool, fatal: bool, status: u16 },
}
```

Methods (constructors + chainable modifiers):

```rust
impl AppError {
    pub fn config(message: impl Into<String>) -> Self;
    pub fn filesystem(path: impl Into<String>, source: impl Into<Option<io::Error>>) -> Self;
    pub fn filesystem_with_source(path: impl Into<PathBuf>, source: io::Error) -> Self;
    pub fn network(endpoint: impl Into<String>, source: impl Into<Option<Box<dyn StdError + Send + Sync>>>) -> Self;
    pub fn network_with_source(endpoint: impl Into<String>, source: Option<Box<dyn StdError + Send + Sync>>) -> Self;
    pub fn other(message: impl Into<String>) -> Self;

    pub fn with_retryable(self, retryable: bool) -> Self;
    pub fn with_fatal(self, fatal: bool) -> Self;
    pub fn with_status(self, status: u16) -> Self;
    pub fn with_code(self, code: impl Into<String>) -> CodedError<Self>;
    pub fn context<C>(self, context: C) -> ContextError<Self, C>
    where
        C: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;
}

impl ForgeError for AppError { /* ... */ }
impl std::fmt::Display for AppError;
impl std::error::Error for AppError;
impl From<io::Error> for AppError;
```

#### Type alias — `AppResult` (and deprecated `Result`)

```rust
pub type AppResult<T> = std::result::Result<T, AppError>;

#[deprecated(since = "1.0.0", note = "use AppResult; Result shadows std::result::Result")]
pub type Result<T> = AppResult<T>;
```

#### Macros (always available, exported at crate root)

- `define_errors!` — declarative custom error enum with generated
  constructors and `ForgeError` metadata.
- `group!` — coarse-grained composition of multiple
  `ForgeError`-implementing types into a single parent enum.

#### `module::collector`

```rust
pub struct ErrorCollector<E> { /* ... */ }

impl<E> ErrorCollector<E> {
    pub fn new() -> Self;
    pub fn push(&mut self, error: E);
    pub fn with(self, error: E) -> Self;
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn into_result<T>(self, ok_value: T) -> Result<T, Self>;
    pub fn result<T>(&self, ok_value: T) -> Result<T, &Self>;
    pub fn into_errors(self) -> Vec<E>;
    pub fn errors(&self) -> &Vec<E>;
    pub fn errors_mut(&mut self) -> &mut Vec<E>;
    pub fn extend(&mut self, other: ErrorCollector<E>);
    pub fn try_collect<F, T>(&mut self, op: F) -> Option<T>
    where F: FnOnce() -> Result<T, E>;
}

impl<E: ForgeError> ErrorCollector<E> {
    pub fn summary(&self) -> String;
    pub fn has_fatal(&self) -> bool;
    pub fn all_retryable(&self) -> bool;
}

impl<E: fmt::Display> fmt::Display for ErrorCollector<E>;
impl<E: Error> Error for ErrorCollector<E>;

pub trait CollectError<T, E> {
    fn collect_err(self, collector: &mut ErrorCollector<E>) -> Option<T>;
}

impl<T, E> CollectError<T, E> for Result<T, E>;
```

#### `module::console_theme`

```rust
pub struct ConsoleTheme { /* private fields, &'static str-backed */ }

impl ConsoleTheme {
    pub fn new() -> Self;
    pub const fn with_colors() -> Self;
    pub const fn plain() -> Self;
    pub fn error(&self, text: &str) -> String;
    pub fn warning(&self, text: &str) -> String;
    pub fn info(&self, text: &str) -> String;
    pub fn success(&self, text: &str) -> String;
    pub fn caption(&self, text: &str) -> String;
    pub fn bold(&self, text: &str) -> String;
    pub fn dim(&self, text: &str) -> String;
    pub fn format_error<E: ForgeError>(&self, err: &E) -> String;
}

impl Default for ConsoleTheme;

pub fn print_error<E: ForgeError>(err: &E);
pub fn install_panic_hook();
```

#### `module::context`

```rust
#[non_exhaustive]
pub struct ContextError<E, C> { pub error: E, pub context: C }

impl<E, C> ContextError<E, C> {
    pub fn new(error: E, context: C) -> Self;
    pub fn into_error(self) -> E;
    pub fn map_context<D, F>(self, f: F) -> ContextError<E, D>
    where F: FnOnce(C) -> D;
    pub fn context<D>(self, context: D) -> ContextError<Self, D>
    where D: fmt::Display + fmt::Debug + Send + Sync + 'static;
}

impl<E: fmt::Display, C: fmt::Display> fmt::Display for ContextError<E, C>;
impl<E: Error + 'static, C: ...> Error for ContextError<E, C>;
impl<E: ForgeError, C: ...> ForgeError for ContextError<E, C>;

pub trait ResultExt<T, E> {
    fn context<C>(self, context: C) -> Result<T, ContextError<E, C>>;
    fn with_context<C, F>(self, f: F) -> Result<T, ContextError<E, C>>
    where F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for Result<T, E>;
```

#### `module::macros`

```rust
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorLevel { Debug, Info, Warning, Error, Critical }

#[non_exhaustive]
pub struct ErrorContext<'a> {
    pub caption: &'a str,
    pub kind: &'a str,
    pub level: ErrorLevel,
    pub is_fatal: bool,
    pub is_retryable: bool,
}

impl<'a> ErrorContext<'a> {
    pub fn new(caption: &'a str, kind: &'a str, level: ErrorLevel,
               is_fatal: bool, is_retryable: bool) -> Self;
}

pub trait ErrorSource {
    fn as_source(&self) -> Option<&(dyn Error + 'static)>;
}

// Built-in `ErrorSource` impls: io::Error, Box<dyn Error+Send+Sync>,
// Box<dyn Error>, Option<io::Error>, Option<Box<dyn Error+Send+Sync>>,
// Option<Box<dyn Error>>.

#[deprecated(since = "1.0.0", note = "use try_register_error_hook")]
pub fn register_error_hook<F>(callback: F)
where F: Fn(ErrorContext<'_>) + Send + Sync + 'static;

pub fn try_register_error_hook<F>(callback: F) -> Result<(), &'static str>
where F: Fn(ErrorContext<'_>) + Send + Sync + 'static;

#[doc(hidden)]
pub fn call_error_hook(caption: &str, kind: &str,
                       is_fatal: bool, is_retryable: bool);
```

#### `module::registry`

```rust
pub struct ErrorRegistry { /* private */ }

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ErrorCodeInfo {
    pub code: String,
    pub description: String,
    pub documentation_url: Option<String>,
    pub retryable: bool,
}

impl ErrorRegistry {
    pub fn register_code(&self, code: String, description: String,
                         documentation_url: Option<String>,
                         retryable: bool) -> Result<(), String>;
    pub fn get_code_info(&self, code: &str) -> Option<ErrorCodeInfo>;
    pub fn is_registered(&self, code: &str) -> bool;
    pub fn global() -> &'static ErrorRegistry;
}

#[non_exhaustive]
#[derive(Debug)]
pub struct CodedError<E> {
    pub error: E,
    pub code: String,
    pub retryable: Option<bool>,
    pub fatal: bool,
    pub status: Option<u16>,
}

impl<E> CodedError<E> {
    pub fn new(error: E, code: impl Into<String>) -> Self;
    pub fn code_info(&self) -> Option<ErrorCodeInfo>;
    pub fn with_retryable(self, retryable: bool) -> Self;
    pub fn with_fatal(self, fatal: bool) -> Self;
    pub fn with_status(self, status: u16) -> Self;
}

impl<E: fmt::Display> fmt::Display for CodedError<E>;
impl<E: Error + 'static> Error for CodedError<E>;
impl<E: ForgeError> ForgeError for CodedError<E>;

pub trait WithErrorCode<E> {
    fn with_code(self, code: impl Into<String>) -> CodedError<E>;
}

impl<E> WithErrorCode<E> for E;

pub fn register_error_code(
    code: impl Into<String>,
    description: impl Into<String>,
    documentation_url: Option<impl Into<String>>,
    retryable: bool,
) -> Result<(), String>;
```

#### `module::recovery`

```rust
// Backoff strategies (re-exported)
pub use backoff::{Backoff, ExponentialBackoff, FixedBackoff, LinearBackoff};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState};
pub use forge_extensions::ForgeErrorRecovery;
pub use retry::{RetryExecutor, RetryPolicy};

pub type RecoveryResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

pub trait Backoff: Send + Sync + 'static {
    fn next_delay(&self, attempt: usize) -> Duration;
    fn reset(&mut self) {}
    fn box_clone(&self) -> Box<dyn Backoff>;
}

#[non_exhaustive]
#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub failure_window_ms: u64,
    pub reset_timeout_ms: u64,
}

impl CircuitBreakerConfig {
    pub fn new(failure_threshold: usize, failure_window_ms: u64, reset_timeout_ms: u64) -> Self;
    pub fn with_failure_threshold(self, t: usize) -> Self;
    pub fn with_failure_window_ms(self, w: u64) -> Self;
    pub fn with_reset_timeout_ms(self, r: u64) -> Self;
}

impl Default for CircuitBreakerConfig;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState { Closed, Open, HalfOpen }

pub struct CircuitBreaker { /* private — uses parking_lot::Mutex */ }

impl CircuitBreaker {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self;
    pub fn state(&self) -> CircuitState;
    pub fn name(&self) -> &str;
    pub fn execute<F, T, E>(&self, f: F) -> RecoveryResult<T>
    where F: FnOnce() -> Result<T, E>, E: Error + Send + Sync + 'static;
    pub fn reset(&self);
}

#[derive(Debug)]
pub struct CircuitOpenError { /* private */ }

impl Display for CircuitOpenError;
impl Error for CircuitOpenError;
```

### `derive` feature

- `#[derive(ModError)]` from `error_forge_derive`.
- Supported attributes: `error_prefix`, `error_display`,
  `error_kind`, `error_caption`, `error_retryable`,
  `error_http_status`, `error_exit_code`, `error_fatal`.

### `serde` feature

- `Serialize` impls on `AppError` and `define_errors!`-generated enums.

### `console` feature

- (reserved; no items currently gated on this feature)

### `backtrace` feature

- (reserved; no items currently gated on this feature)

### `jitter` feature

- `recovery::ExponentialBackoff::with_jitter(true)` activates ±20%
  jitter on each calculated delay. Without the feature, the flag
  is silently ignored.

### `log` feature

- `error_forge::logging::log_impl::{LogAdapter, init}`.

### `tracing` feature

- `error_forge::logging::tracing_impl::{TracingAdapter, init}`.

### `async` feature

```rust
pub type AsyncResult<T, E> = Result<T, E>;

#[async_trait]
pub trait AsyncForgeError: Error + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool { false }
    fn is_fatal(&self) -> bool { false }
    fn status_code(&self) -> u16 { 500 }
    fn exit_code(&self) -> i32 { 1 }
    fn user_message(&self) -> String;
    fn dev_message(&self) -> String;
    fn backtrace(&self) -> Option<&Backtrace> { None }
    async fn async_handle(&self) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }
    fn register(&self);
}

#[async_trait]
impl AsyncForgeError for AppError;

impl AppError {
    pub async fn from_async_result<T, E>(result: Result<T, E>) -> Result<T, Self>
    where E: Error + Send + Sync + 'static;
    pub async fn handle_async(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
```

### `registry`, `collector`, `context` features

These cargo features exist for granular feature-gating but
currently gate no behaviour (the underlying types are always
available). They are reserved for future minor releases to gate
optional extensions.

---

## Internal items (NOT part of the public surface)

These exist in `src/` but are deliberately `pub(crate)`,
private, or live in non-`pub`-re-exported modules. They may
change between any two `1.x` releases without notice.

- `src/error.rs::panic_payload_to_listener_error` (`pub(crate)`).
- `src/macros.rs::call_error_hook` (`#[doc(hidden)] pub`).
- `src/recovery/retry.rs::{BackoffStrategy, BackoffType,
  RetryPredicate}` (`pub` syntactically but the `retry` module
  is private inside `recovery`, so these are not externally
  reachable).
- `src/lib.rs::__private::pastey` re-export
  (`#[doc(hidden)] pub mod __private`) — for macro use only.
