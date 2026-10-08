## API Reference

This document is the **narrative** API reference — guided walkthroughs
of the most common surfaces. For the **canonical** manifest of every
public symbol in `1.0.0` (locked under SemVer for the `1.x` line), see
[`API-FREEZE-AUDIT.md`](API-FREEZE-AUDIT.md).

Other companions:

- [`STABILITY.md`](STABILITY.md) — binding SemVer / panic-safety / MSRV / deprecation policy.
- [`COMPARISON.md`](COMPARISON.md) — side-by-side with `anyhow`, `thiserror`, `miette`, `snafu`, `eyre`.
- [`architecture.md`](architecture.md) — design rationale for the trait, hook system, recovery primitives.
- [`migration.md`](migration.md) — upgrade guides from earlier versions.

This document tracks the public surface that is available today. It intentionally favors accuracy over aspiration.

Every `rust` code block in this file is compiled and run as a doctest
(`cargo test --all-features`), so the examples track the real API. The
examples that use `#[derive(ModError)]` or `AsyncForgeError` need the
`derive` and `async` features.

## Feature Flags

| Feature | Enables |
| --- | --- |
| `derive` | Re-exports `#[derive(ModError)]` from `error-forge-derive` |
| `async` | `AsyncForgeError`, `AsyncResult`, and async helpers on `AppError` |
| `serde` | `Serialize` on `AppError` |
| `log` | `logging::log_impl` adapter |
| `tracing` | `logging::tracing_impl` adapter |
| `jitter` | ±20% jitter in `ExponentialBackoff::with_jitter` (pulls in `rand`) |
| `console`, `backtrace`, `registry`, `collector`, `context` | Reserved. They currently gate nothing; the related APIs are always available. |

`define_errors!` adds `#[cfg_attr(feature = "serde", derive(serde::Serialize))]`
to the enums it generates. That `cfg` is evaluated in your crate, so it
follows your crate's own `serde` feature, not error-forge's.

## Core Types

### `ForgeError`

`ForgeError` is the crate's central trait. It extends `std::error::Error` with stable metadata that is useful in logs, HTTP layers, workers, and recovery policies. Its shape, with the provided default bodies:

```rust
use std::backtrace::Backtrace;

pub trait ForgeError: std::error::Error + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool { false }
    fn is_fatal(&self) -> bool { false }
    fn status_code(&self) -> u16 { 500 }
    fn exit_code(&self) -> i32 { 1 }
    fn user_message(&self) -> String { self.to_string() }
    fn dev_message(&self) -> String { format!("[{}] {}", self.kind(), self) }
    fn backtrace(&self) -> Option<&Backtrace> { None }
    // The real default forwards to the registered error hook.
    fn register(&self) {}
}
```

Key defaults:

- `is_retryable()`: `false`
- `is_fatal()`: `false`
- `status_code()`: `500`
- `exit_code()`: `1`

### `AppError`

`AppError` is the built-in general-purpose error enum. Variants:

- `Config`
- `Filesystem`
- `Network`
- `Other`

Convenience constructors:

- `AppError::config(...)`
- `AppError::filesystem(...)`
- `AppError::filesystem_with_source(...)`
- `AppError::network(...)`
- `AppError::network_with_source(...)`
- `AppError::other(...)`

Common modifiers:

- `with_retryable(bool)`
- `with_fatal(bool)`
- `with_status(u16)`
- `with_code(...)`
- `context(...)`

```rust
use error_forge::{AppError, ForgeError};

let error = AppError::network("api.example.com", None).with_status(502);
assert_eq!(error.kind(), "Network");
assert!(error.is_retryable());
assert_eq!(error.status_code(), 502);
```

### `AppResult<T>`

```rust
pub type AppResult<T> = std::result::Result<T, error_forge::AppError>;
```

`error_forge::Result<T>` is a deprecated alias for the same type. It
shadows `std::result::Result` in `use error_forge::*` glob imports.

## Declarative Macros

### `define_errors!`

Use `define_errors!` when you want a custom error enum with generated constructors and `ForgeError`-style metadata methods. The methods are inherent; the macro does not implement the `ForgeError` trait itself, so add a delegating `impl ForgeError` when the enum has to satisfy a `ForgeError` bound (`group!`, `ForgeErrorRecovery`, `log_error`, `print_error`).

```rust
use error_forge::define_errors;

define_errors! {
    pub enum ApiError {
        #[error(display = "Configuration error: {message}", message)]
        #[kind(Config, status = 500)]
        Config { message: String },

        #[error(display = "Request to {endpoint} failed", endpoint)]
        #[kind(Network, retryable = true, status = 503)]
        Network { endpoint: String },
    }
}

fn main() {
    let error = ApiError::network("api.example.com".to_string());
    assert_eq!(error.to_string(), "Request to api.example.com failed");
    assert_eq!(error.kind(), "Network");
    assert!(error.is_retryable());
    assert_eq!(error.status_code(), 503);
}
```

Rules and behavior:

- Each variant requires `#[kind(...)]`. `#[error(display = ...)]` is optional and may come before or after it; doc comments and other attributes on a variant are kept, and the trailing comma after the last variant is optional.
- Constructor names are the lowercase form of the variant name (`RequestFailed` becomes `requestfailed`).
- `retryable`, `fatal`, `status`, `exit`, and `caption` can be supplied inside `#[kind(...)]`. Any other tag is a compile error.
- The display string is a `format!` string. Fields listed after it are passed as named arguments and fields named inline (`{path:?}`) are captured, so `{{` and `}}` print literal braces. Only the fields the string formats need `Display` or `Debug`.
- A field named `source` is used for `Error::source()` chaining.
- For custom `source` field types, implement `error_forge::macros::ErrorSource` in your crate.
- If `#[error(display = ...)]` is omitted, display falls back to the caption, variant name, and debug-formatted fields (a `source` field is shown with `Display`). Only this fallback needs every field to implement `Debug`.

### `group!`

`group!` creates a parent error enum with `From<T>` conversions for wrapped source types. Every wrapped type must implement `ForgeError`; the generated `ForgeError` impl delegates to the wrapped value. To group a foreign error such as `std::io::Error`, wrap it in your own error type first.

```rust
use error_forge::{group, AppError, ForgeError, ModError};

#[derive(Debug, ModError)]
#[error_prefix("Storage")]
pub enum StorageError {
    #[error_display("Disk full on {0}")]
    #[error_http_status(507)]
    DiskFull(String),
}

group! {
    #[derive(Debug)]
    pub enum ServiceError {
        App(AppError),
        Storage(StorageError),
    }
}

fn main() {
    let error: ServiceError = StorageError::DiskFull("/var".to_string()).into();
    assert_eq!(error.to_string(), "Disk full on /var");
    assert_eq!(error.status_code(), 507);

    let error: ServiceError = AppError::config("missing key").into();
    assert_eq!(error.kind(), "Config");
}
```

This macro is best suited for coarse-grained composition at module or service boundaries.

## Derive Macro

Enable the `derive` feature to use `#[derive(ModError)]`.

Supported attributes:

- `error_prefix`
- `error_display`
- `error_kind`
- `error_caption`
- `error_retryable`
- `error_http_status`
- `error_exit_code`
- `error_fatal`

`error_display` is optional and may mention any subset of the fields: by name for struct-like variants (`{type}` for a raw `r#type` field), by position for tuple variants. Width and precision given where the error is formatted (`{:>20}`) apply to the whole message.

Every attribute accepts the list form (`#[error_http_status(404)]`) and the name-value form (`#[error_http_status = 404]`). `#[error_retryable]` and `#[error_fatal]` alone mean `true`; an explicit `(false)` or `= false` is honoured. A value of the wrong type, or an integer that does not fit (`u16` for `error_http_status`, `i32` for `error_exit_code`), is a compile error.

On a struct only `error_prefix` is read; the struct displays as `"<prefix>: Error"` and the other attributes are ignored.

Example:

```rust
use error_forge::{ForgeError, ModError};

#[derive(Debug, ModError)]
#[error_prefix("Database")]
enum DbError {
    #[error_display("Connection failed: {0}")]
    #[error_retryable]
    #[error_http_status(503)]
    ConnectionFailed(String),

    #[error_display("Query failed: {reason}")]
    QueryFailed { reason: String, query: String },

    #[error_display("Permission denied")]
    #[error_fatal]
    PermissionDenied,
}

let err = DbError::ConnectionFailed("primary".into());
assert!(err.is_retryable());
assert_eq!(err.status_code(), 503);
assert_eq!(err.caption(), "Database: Error");

let err = DbError::QueryFailed {
    reason: "timeout".into(),
    query: "SELECT 1".into(),
};
assert_eq!(err.to_string(), "Query failed: timeout");
assert!(DbError::PermissionDenied.is_fatal());
```

## Context and Wrapping

### `ContextError<E, C>`

Wraps an error and an arbitrary context value. The `error` and `context` fields are public; the struct is `#[non_exhaustive]`, so build it with `ContextError::new` or the extension methods.

Useful methods:

- `ContextError::new(error, context)`
- `into_error()`
- `map_context(...)`
- `context(...)` to nest additional context layers

### `ResultExt`

Extension trait for `Result<T, E>`:

- `context(value)` eagerly adds context on error
- `with_context(|| value)` lazily creates context only on error

```rust
use error_forge::{AppError, ForgeError, ResultExt};

fn load_settings(path: &str) -> Result<String, AppError> {
    Err(AppError::config(format!("{path} is empty")))
}

let err = load_settings("app.toml")
    .with_context(|| "Loading settings".to_string())
    .unwrap_err();

assert_eq!(err.context, "Loading settings");
assert_eq!(
    err.to_string(),
    "Loading settings: ⚙️ Configuration Error: app.toml is empty"
);
// Metadata comes from the wrapped error.
assert_eq!(err.kind(), "Config");

let outer = err.context("Starting service");
assert!(outer.to_string().starts_with("Starting service: Loading settings: "));
```

## Error Codes and Registry

### `register_error_code(...)`

Registers a stable code with description, optional documentation URL, and retryability metadata. Registering the same code twice returns `Err`.

### `WithErrorCode` and `CodedError<E>`

Attach a code to an error with `with_code(...)`.

`CodedError<E>` preserves the underlying error and supports instance-level overrides:

- `with_retryable(bool)`
- `with_fatal(bool)`
- `with_status(u16)`

Retryability resolution order:

1. explicit instance override
2. registered code metadata
3. underlying error metadata

Status-code resolution order:

1. explicit instance override
2. underlying error status code

```rust
use error_forge::{register_error_code, AppError, ForgeError};

register_error_code(
    "AUTH-001",
    "Credentials were rejected",
    Some("https://docs.example.com/errors/auth-001"),
    false,
)
.expect("code registered once");

let error = AppError::other("bad password").with_code("AUTH-001");
assert_eq!(error.to_string(), "[AUTH-001] 🚨 Error: bad password");
assert!(!error.is_retryable());
assert!(error.dev_message().ends_with("(https://docs.example.com/errors/auth-001)"));
assert_eq!(error.code_info().unwrap().description, "Credentials were rejected");

// Per-instance overrides win over the registry and the inner error.
let error = AppError::other("rate limited")
    .with_code("AUTH-001")
    .with_retryable(true)
    .with_status(429);
assert!(error.is_retryable());
assert_eq!(error.status_code(), 429);
```

## Collection

### `ErrorCollector<E>`

An accumulator for collecting multiple errors before returning.

Useful methods:

- `new()`
- `push(...)`
- `with(...)`
- `len()`
- `is_empty()`
- `into_result(ok_value)`
- `result(ok_value)`
- `try_collect(...)`
- `errors()`, `into_errors()`
- `summary()` for `E: ForgeError`
- `has_fatal()` for `E: ForgeError`
- `all_retryable()` for `E: ForgeError`

```rust
use error_forge::{AppError, CollectError, ErrorCollector};

struct Form {
    username: String,
    email: String,
}

fn validate(form: &Form) -> Result<(), ErrorCollector<AppError>> {
    let mut errors = ErrorCollector::new();
    if form.username.len() < 3 {
        errors.push(AppError::other("username is too short"));
    }
    if !form.email.contains('@') {
        errors.push(AppError::other("email is invalid"));
    }
    let parsed: Result<u32, AppError> = "42".parse().map_err(|_| AppError::other("bad age"));
    let _age = parsed.collect_err(&mut errors);
    errors.into_result(())
}

let errors = validate(&Form {
    username: "al".into(),
    email: "nowhere".into(),
})
.unwrap_err();
assert_eq!(errors.len(), 2);
assert!(errors.summary().starts_with("2 errors collected (0 fatal, 0 retryable)"));
```

## Logging and Hooks

### Hook API

Available in `error_forge::macros` (and re-exported at the crate root):

- `try_register_error_hook(...)`
- `register_error_hook(...)` (deprecated; ignores a failed registration)
- `ErrorContext`
- `ErrorLevel`

`call_error_hook(...)` is hidden and used by generated code. Only one hook can be installed per process; `try_register_error_hook(...)` returns an error if a hook was already installed. Every `AppError` and `define_errors!` constructor calls the hook, as does `ForgeError::register`.

The hook is not re-entered: an error created while the hook runs on the same thread (for example by a log sink that fails) does not call it again. A panic inside the hook is caught and discarded so it does not unwind through the constructor that fired it; the process panic hook still reports it.

```rust
use error_forge::{try_register_error_hook, AppError, ErrorLevel};
use std::sync::{Arc, Mutex};

let seen: Arc<Mutex<Vec<String>>> = Arc::default();
let sink = Arc::clone(&seen);
try_register_error_hook(move |ctx| {
    let level = match ctx.level {
        ErrorLevel::Critical => "critical",
        ErrorLevel::Error => "error",
        ErrorLevel::Warning => "warning",
        ErrorLevel::Info => "info",
        ErrorLevel::Debug => "debug",
        // `ErrorLevel` is `#[non_exhaustive]`.
        _ => "other",
    };
    sink.lock().unwrap().push(format!("{level}: {}", ctx.kind));
})
.expect("first hook in this process");

let _config = AppError::config("missing key");
let _network = AppError::network("api.example.com", None);

assert_eq!(*seen.lock().unwrap(), ["error: Config", "info: Network"]);
assert!(try_register_error_hook(|_| {}).is_err());
```

### Logging API

Available in `error_forge::logging`:

- `register_logger(...)`
- `logger()`
- `log_error(...)`
- `ErrorLogger`
- `custom::ErrorLoggerBuilder`

Feature-gated adapters:

- `logging::log_impl::init()` with the `log` feature
- `logging::tracing_impl::init()` with the `tracing` feature

`ErrorLogger::log_panic` is not called automatically; forward to it from your own panic hook if you want panics logged.

```rust
use error_forge::logging::custom::ErrorLoggerBuilder;
use error_forge::{log_error, register_logger, AppError};
use std::sync::{Arc, Mutex};

let lines: Arc<Mutex<Vec<String>>> = Arc::default();
let sink = Arc::clone(&lines);
let logger = ErrorLoggerBuilder::new()
    .with_error_fn(move |error, level| {
        sink.lock().unwrap().push(format!("{level:?} {}", error.dev_message()));
    })
    .build();
register_logger(logger).expect("first logger in this process");

log_error(&AppError::config("missing key").with_fatal(true));
assert_eq!(
    *lines.lock().unwrap(),
    ["Critical [Config] ⚙️ Configuration Error: missing key"]
);
```

## Formatting

### `ConsoleTheme`

Provides console-friendly formatting and panic-hook installation. `ConsoleTheme::new()` detects colour support (stderr must be a terminal, `TERM` not `dumb`, and `NO_COLOR` unset or empty); `with_colors()` and `plain()` force the choice.

Useful exports:

- `ConsoleTheme`
- `print_error(...)`
- `install_panic_hook()`

```rust
use error_forge::{print_error, AppError, ConsoleTheme};

let error = AppError::config("Configuration file not found");

// Writes to stderr with the cached default theme.
print_error(&error);

let report = ConsoleTheme::plain().format_error(&error);
assert!(report.contains("Configuration file not found"));
assert!(report.contains("Retryable: No"));

// `install_panic_hook()` installs a hook that prints panics through the
// default theme and then runs the hook that was installed before it.
error_forge::install_panic_hook();
```

## Recovery

The recovery APIs are synchronous.

### Backoff Strategies

- `ExponentialBackoff`: `with_initial_delay`, `with_max_delay`, `with_factor`, `with_jitter`
- `LinearBackoff`: `with_initial_delay`, `with_increment`, `with_max_delay`
- `FixedBackoff::new(delay_ms)`

Every strategy implements the `Backoff` trait (`next_delay(attempt)`). Delays never exceed the configured maximum.

```rust
use error_forge::recovery::{Backoff, ExponentialBackoff, FixedBackoff, LinearBackoff};
use std::time::Duration;

let exponential = ExponentialBackoff::new()
    .with_initial_delay(100)
    .with_max_delay(1_000)
    .with_factor(2.0);
assert_eq!(exponential.next_delay(0), Duration::from_millis(100));
assert_eq!(exponential.next_delay(2), Duration::from_millis(400));
assert_eq!(exponential.next_delay(10), Duration::from_millis(1_000));

let linear = LinearBackoff::new()
    .with_initial_delay(100)
    .with_increment(50)
    .with_max_delay(300);
assert_eq!(linear.next_delay(1), Duration::from_millis(150));
assert_eq!(linear.next_delay(9), Duration::from_millis(300));

let fixed = FixedBackoff::new(200);
assert_eq!(fixed.next_delay(5), Duration::from_millis(200));
```

### Retry

- `RetryPolicy::new_exponential()`
- `RetryPolicy::new_linear()`
- `RetryPolicy::new_fixed(delay_ms)`
- `with_max_retries(...)`
- `executor::<E>()`
- `forge_executor::<E>()` (retries only errors whose `is_retryable()` is true)
- `retry(...)`
- `RetryExecutor::with_retry_if(...)` and `retry_with_handler(...)`

`RetryExecutor` uses blocking sleeps, so it is best suited to sync workloads or dedicated worker threads.

```rust
use error_forge::recovery::RetryPolicy;
use error_forge::AppError;
use std::cell::Cell;

let attempts = Cell::new(0);
let result = RetryPolicy::new_fixed(1)
    .with_max_retries(3)
    .forge_executor::<AppError>()
    .retry(|| {
        attempts.set(attempts.get() + 1);
        if attempts.get() < 3 {
            // Network errors are retryable by default.
            Err(AppError::network("api.example.com", None))
        } else {
            Ok("response")
        }
    });
assert_eq!(result.unwrap(), "response");
assert_eq!(attempts.get(), 3);

// Config errors are not retryable, so they fail on the first attempt.
let attempts = Cell::new(0);
let result: Result<(), AppError> = RetryPolicy::new_fixed(1)
    .forge_executor()
    .retry(|| {
        attempts.set(attempts.get() + 1);
        Err(AppError::config("bad key"))
    });
assert!(result.is_err());
assert_eq!(attempts.get(), 1);
```

### Circuit Breaker

- `CircuitBreaker::new(name)`
- `CircuitBreaker::with_config(name, config)`
- `execute(...)`
- `state()`
- `reset()`
- `CircuitBreakerConfig::new(threshold, window_ms, reset_ms)` or `CircuitBreakerConfig::default()` with `with_failure_threshold`, `with_failure_window_ms`, `with_reset_timeout_ms`

States:

- `Closed`: calls pass through and failures are counted inside the window
- `Open`: calls fail fast with `CircuitOpenError`
- `HalfOpen`: the reset timeout has elapsed; one probe call is admitted and others fail fast until it resolves

```rust
use error_forge::recovery::{CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState};
use error_forge::AppError;

let config = CircuitBreakerConfig::default()
    .with_failure_threshold(2)
    .with_reset_timeout_ms(30_000);
let breaker = CircuitBreaker::with_config("inventory", config);

for _ in 0..2 {
    let result = breaker.execute(|| Err::<(), _>(AppError::network("inventory", None)));
    assert!(result.is_err());
}
assert_eq!(breaker.state(), CircuitState::Open);

// While open, calls fail fast without running the closure.
let err = breaker.execute(|| Ok::<_, AppError>("unreachable")).unwrap_err();
assert!(err.is::<CircuitOpenError>());

breaker.reset();
assert_eq!(breaker.state(), CircuitState::Closed);
```

### `ForgeErrorRecovery`

Extension trait implemented for every `ForgeError` type. It needs no manual impl.

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `create_retry_policy()` | `max_retries: usize` | `RetryPolicy` | Exponential retry policy with the given retry limit |
| `retry()` | `max_retries: usize, operation: F` | `Result<T, E>` | Runs `operation` once if the receiver's `is_retryable()` is false; otherwise retries errors whose `is_retryable()` is true |
| `create_circuit_breaker()` | `name` | `CircuitBreaker` | Circuit breaker with the default configuration |

```rust
use error_forge::recovery::ForgeErrorRecovery;
use error_forge::AppError;

let template = AppError::network("api.example.com", None);
let policy = template.create_retry_policy(2);

let result: Result<u8, AppError> = policy.retry(|| Ok(7));
assert_eq!(result.unwrap(), 7);

let breaker = template.create_circuit_breaker("api");
assert_eq!(breaker.name(), "api");
```

## Async Support

Enabled with the `async` feature.

### `AsyncForgeError`

The trait mirrors `ForgeError` and adds an async hook:

```rust
use async_trait::async_trait;
use std::backtrace::Backtrace;
use std::error::Error;

#[async_trait]
pub trait AsyncForgeError: Error + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool { false }
    fn is_fatal(&self) -> bool { false }
    fn status_code(&self) -> u16 { 500 }
    fn exit_code(&self) -> i32 { 1 }
    fn user_message(&self) -> String { self.to_string() }
    fn dev_message(&self) -> String { format!("[{}] {}", self.kind(), self) }
    fn backtrace(&self) -> Option<&Backtrace> { None }
    async fn async_handle(&self) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }
    // The real default forwards to the registered error hook.
    fn register(&self) {}
}
```

Additional async exports:

- `AsyncResult<T, E>`
- `AppError::from_async_result(...)`
- `AppError::handle_async()`

```rust
use async_trait::async_trait;
use error_forge::{AppError, AsyncForgeError};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct UploadError(String);

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "upload failed: {}", self.0)
    }
}

impl Error for UploadError {}

#[async_trait]
impl AsyncForgeError for UploadError {
    fn kind(&self) -> &'static str { "Upload" }
    fn caption(&self) -> &'static str { "Upload Error" }
    fn is_retryable(&self) -> bool { true }

    async fn async_handle(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Flush telemetry, release resources, and so on.
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let error = UploadError("bucket offline".into());
    assert!(error.is_retryable());
    assert!(error.async_handle().await.is_ok());

    let io: Result<(), std::io::Error> = Err(std::io::Error::other("timed out"));
    let wrapped = AppError::from_async_result(io).await.unwrap_err();
    assert!(wrapped.to_string().contains("timed out"));
}
```

## Stability Notes

- The crate is cross-platform and tested on Windows-friendly paths and outputs.
- Public examples are kept aligned with `cargo test --all-features` and strict Clippy.
- Recovery helpers are sync-first by design; async runtimes should wrap those patterns intentionally rather than rely on hidden blocking behavior.
