<div align="center">
  <img width="120" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Error Forge logo">
  <h1><strong>Error Forge</strong></h1>
  <p>Pragmatic error modeling, contextual diagnostics, and resilience helpers for Rust.</p>
</div>

Error Forge is a Rust error-handling crate built around a few simple ideas:

- Errors should carry stable metadata such as kind, retryability, status code, and exit code.
- Application code should be able to add context without destroying the original cause chain.
- Operational tooling should have clear hooks for logging, formatting, codes, and recovery policies.
- Feature-gated integrations should stay optional so the core remains lightweight.

It ships with a built-in `AppError`, a declarative `define_errors!` macro, an optional `#[derive(ModError)]` proc macro, error collectors, registry support, console formatting, synchronous retry and circuit-breaker primitives, and async-specific traits behind feature flags.

## Installation

```toml
[dependencies]
error-forge = "0.9.7"
```

Common optional features:

- `derive`: enables `#[derive(ModError)]`
- `async`: enables `AsyncForgeError`
- `serde`: enables serialization support where compatible
- `log`: enables the `log` adapter
- `tracing`: enables the `tracing` adapter

## Quick Start

### Built-in `AppError`

```rust
use error_forge::{AppError, ForgeError};

fn load_config() -> Result<(), AppError> {
    Err(AppError::config("Missing DATABASE_URL").with_fatal(true))
}

fn main() {
    let error = load_config().unwrap_err();

    assert_eq!(error.kind(), "Config");
    assert!(error.is_fatal());
    println!("{}", error);
}
```

### Defining Custom Errors With `define_errors!`

`define_errors!` is the lowest-friction way to create a custom error enum with generated constructors and `ForgeError` metadata.

```rust
use error_forge::{define_errors, ForgeError};
use std::io;

define_errors! {
    pub enum ServiceError {
        #[error(display = "Configuration is invalid: {message}", message)]
        #[kind(Config, status = 500)]
        Config { message: String },

        #[error(display = "Request to {endpoint} failed", endpoint)]
        #[kind(Network, retryable = true, status = 503)]
        Network { endpoint: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

        #[error(display = "Could not read {path}", path)]
        #[kind(Filesystem, status = 500)]
        Filesystem { path: String, source: io::Error },
    }
}

fn main() {
    let error = ServiceError::config("Missing API token".to_string());
    assert_eq!(error.kind(), "Config");
    assert_eq!(error.status_code(), 500);
}
```

Notes:

- `#[kind(...)]` is required for each variant.
- Constructors are generated from the lowercase variant name, such as `ServiceError::config(...)`.
- A field named `source` participates in `std::error::Error::source()` chaining.
- For custom `source` field types, implement `error_forge::macros::ErrorSource` in your crate.
- With the `serde` feature enabled, source fields must themselves be serializable if you want to derive serialization through the macro-generated enum.

### Adding Context Without Losing the Original Error

```rust
use error_forge::{AppError, ResultExt};

fn connect() -> Result<(), AppError> {
    Err(AppError::network("db.internal", None))
}

fn main() {
    let error = connect()
        .with_context(|| "opening primary database connection".to_string())
        .unwrap_err();

    println!("{}", error);
}
```

### Collecting Multiple Errors

```rust
use error_forge::{AppError, ErrorCollector};

fn main() {
    let mut collector = ErrorCollector::new();
    collector.push(AppError::config("missing host"));
    collector.push(AppError::other("invalid timeout"));

    assert_eq!(collector.len(), 2);
    println!("{}", collector.summary());
}
```

## Derive Macro

Enable the `derive` feature to use `#[derive(ModError)]`.

```rust
use error_forge::{ForgeError, ModError};

#[derive(Debug, ModError)]
#[error_prefix("Database")]
enum DbError {
    #[error_display("Connection failed: {0}")]
    #[error_retryable]
    #[error_http_status(503)]
    ConnectionFailed(String),

    #[error_display("Query failed for {query}")]
    QueryFailed { query: String },

    #[error_display("Permission denied")]
    #[error_fatal]
    PermissionDenied,
}

fn main() {
    let error = DbError::ConnectionFailed("primary".to_string());
    assert!(error.is_retryable());
    assert_eq!(error.status_code(), 503);
}
```

Supported derive attributes:

- `error_prefix`
- `error_display`
- `error_kind`
- `error_caption`
- `error_retryable`
- `error_http_status`
- `error_exit_code`
- `error_fatal`

Both list-style and name-value forms are supported for `error_prefix`.

## Recovery and Resilience

The recovery module is intentionally synchronous today. It is designed for blocking code, worker threads, and service wrappers where a small sleep is acceptable.

```rust
use error_forge::recovery::{CircuitBreaker, RetryPolicy};

fn main() {
    let breaker = CircuitBreaker::new("inventory-service");
    let policy = RetryPolicy::new_fixed(25).with_max_retries(3);

    let value: Result<u32, std::io::Error> = breaker.execute(|| {
        policy.retry(|| Ok(42))
    });

    assert_eq!(value.unwrap(), 42);
}
```

If you need async retries, keep Error Forge for modeling and classification, then wrap retry behavior with your async runtime of choice.

## Hooks, Logging, and Formatting

### Error Hooks

```rust
use error_forge::{
    AppError,
    macros::{try_register_error_hook, ErrorLevel},
};

fn main() {
    let _ = try_register_error_hook(|ctx| {
        if matches!(ctx.level, ErrorLevel::Critical | ErrorLevel::Error) {
            eprintln!("{} [{}]", ctx.caption, ctx.kind);
        }
    });

    let _ = AppError::config("Missing environment variable");
}
```

### Logging Adapters

- `logging::register_logger(...)` installs a custom logger once.
- `logging::log_impl::init()` is available with the `log` feature.
- `logging::tracing_impl::init()` is available with the `tracing` feature.

### Console Output

```rust
use error_forge::{console_theme::print_error, AppError};

fn main() {
    let error = AppError::filesystem("config.toml", None);
    print_error(&error);
}
```

## Error Codes

Attach stable codes to errors when you want machine-readable identifiers or documentation links.

```rust
use error_forge::{register_error_code, AppError, ForgeError};

fn main() {
    let _ = register_error_code(
        "AUTH-001",
        "Authentication failed",
        Some("https://example.com/errors/AUTH-001"),
        false,
    );

    let error = AppError::config("Invalid credentials")
        .with_code("AUTH-001")
        .with_status(401);

    assert_eq!(error.status_code(), 401);
    println!("{}", error.dev_message());
}
```

## Quality Bar

The crate is validated with:

- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- targeted examples and feature-gated regression coverage

## Documentation

- API reference: `docs/API.md`
- Examples: `examples/`
- Crate documentation: https://docs.rs/error-forge

## License

Licensed under Apache-2.0.