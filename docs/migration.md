# error-forge — Migration Guide

> How to move from earlier versions to the current one. Covers
> every breaking change the crate has shipped.

## Upgrading from `0.9.x` to `1.0.0`

`1.0.0` is the API freeze. See
[`docs/STABILITY.md`](STABILITY.md) for the SemVer contract that
the rest of the `1.x` line locks in.

`1.0.0` makes a small number of breaking changes relative to
`0.9.8`. Most callers need zero source changes; a few corner
cases require minor edits.

### 1. `group!` macro now requires `: ForgeError` on each wrapped type

**Before (`0.9.x`):**

```rust
use error_forge::{group, AppError};
use std::io;

group! {
    pub enum ServiceError {
        App(AppError),
        Io(io::Error),         // <-- io::Error does NOT impl ForgeError
    }
}
```

`0.9.x` accepted any wrapped type, but the generated
`ForgeError` impl was silently incorrect — `Io(io::Error)`'s
`kind()` returned `"Io"`, `status_code()` returned `500`, and
`is_retryable()` returned `false` regardless of the actual
inner error.

**After (`1.0.0`):**

```rust
use error_forge::{define_errors, group, AppError, ForgeError};

// Wrap `io::Error` in a ForgeError-implementing newtype first.
define_errors! {
    pub enum FsError {
        #[error(display = "Filesystem failure: {message}", message)]
        #[kind(Filesystem, status = 500)]
        Generic { message: String, source: std::io::Error },
    }
}

group! {
    pub enum ServiceError {
        App(AppError),       // implements ForgeError directly
        Fs(FsError),         // implements ForgeError via define_errors!
    }
}
```

Each variant in a `group!` enum must now wrap a type that
implements `ForgeError`. If you have a bare `std::io::Error` or
similar foreign error type, wrap it once via `define_errors!`
or `#[derive(ModError)]` and then group the result.

### 2. `AsyncForgeError::async_handle` is now optional

**Before (`0.9.x`):**

```rust
#[async_trait]
impl AsyncForgeError for MyAsyncError {
    fn kind(&self) -> &'static str { "MyKind" }
    fn caption(&self) -> &'static str { "My Error" }

    // Required — couldn't be omitted.
    async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        Ok(())
    }
}
```

**After (`1.0.0`):**

```rust
#[async_trait]
impl AsyncForgeError for MyAsyncError {
    fn kind(&self) -> &'static str { "MyKind" }
    fn caption(&self) -> &'static str { "My Error" }

    // Optional — default body returns `Ok(())`.
    // Override only if you want async behaviour.
}
```

`async_handle` has gained a default no-op body. Implementors
who explicitly returned `Ok(())` (the previous canonical stub)
can simply delete the method.

### 3. `AppError::with_async_context` removed

**Before (`0.9.x`):**

```rust
let err = AppError::network("api", None)
    .with_async_context(|| "during onboarding".to_string());
```

The method took a `FnOnce()` and synchronously called it. There
was nothing async about it.

**After (`1.0.0`):**

```rust
// Use ResultExt::with_context — works on any Result.
let result: Result<(), AppError> = Err(AppError::network("api", None));
let result = result.with_context(|| "during onboarding".to_string());
```

Or build the `ContextError` directly:

```rust
use error_forge::ContextError;
let err = ContextError::new(
    AppError::network("api", None),
    "during onboarding".to_string(),
);
```

### 4. `#[non_exhaustive]` added to several public types

The following types are now `#[non_exhaustive]`:

- `error_forge::ErrorContext`
- `error_forge::ErrorLevel`
- `error_forge::ErrorCodeInfo`
- `error_forge::CodedError`
- `error_forge::ContextError`
- `error_forge::recovery::CircuitBreakerConfig`
- `error_forge::recovery::CircuitState`

This means **external code can no longer construct these via
struct-literal syntax**, and exhaustive `match` statements
need a wildcard arm.

**Before (`0.9.x`):**

```rust
let cfg = CircuitBreakerConfig {
    failure_threshold: 3,
    failure_window_ms: 1000,
    reset_timeout_ms: 500,
};

match level {
    ErrorLevel::Debug => ...,
    ErrorLevel::Info => ...,
    ErrorLevel::Warning => ...,
    ErrorLevel::Error => ...,
    ErrorLevel::Critical => ...,
}
```

**After (`1.0.0`):**

```rust
let cfg = CircuitBreakerConfig::new(3, 1000, 500);
// or
let cfg = CircuitBreakerConfig::default()
    .with_failure_threshold(3)
    .with_failure_window_ms(1000)
    .with_reset_timeout_ms(500);

match level {
    ErrorLevel::Debug => ...,
    ErrorLevel::Info => ...,
    ErrorLevel::Warning => ...,
    ErrorLevel::Error => ...,
    ErrorLevel::Critical => ...,
    _ => ...,    // required arm — minor releases may add variants
}
```

Equivalent constructors are provided for `ErrorContext` and
`CircuitBreakerConfig`. The other types are typically only
returned by `error-forge`'s own surface (`metrics()`,
`code_info()`), so external construction is rare.

### 5. `CodedError::new` no longer auto-registers the code

**Before (`0.9.x`):**

```rust
// First use of a code lazily registered it with default metadata.
let err = AppError::config("missing").with_code("AUTH-001");
let info = ErrorRegistry::global().get_code_info("AUTH-001");
// info.description == "Error code AUTH-001"  (auto-generated)
```

**After (`1.0.0`):**

```rust
// Pre-register codes at startup if you want documented metadata.
use error_forge::register_error_code;

fn main() {
    let _ = register_error_code(
        "AUTH-001",
        "Authentication failed",
        Some("https://example.com/errors/AUTH-001"),
        false,
    );
    // ... rest of app ...
}

// Anywhere in the app, attach the code to an error:
let err = AppError::config("invalid credentials").with_code("AUTH-001");
let info = err.code_info().unwrap();
// info.description == "Authentication failed"  (your registration)
```

Codes attached via `with_code` that were never registered now
return `None` from `code_info()`. If you were relying on the
auto-generated `"Error code AUTH-001"` description, add an
explicit `register_error_code` call.

### 6. Hook callback widened to support closures

**Before (`0.9.x`):**

```rust
fn my_hook(ctx: ErrorContext) {
    println!("{}: {}", ctx.kind, ctx.caption);
}
register_error_hook(my_hook);   // function pointer only
```

**After (`1.0.0`):**

```rust
// Function pointer still works:
register_error_hook(my_hook);

// Closures capturing state now work too:
let log = Arc::new(Mutex::new(Vec::<String>::new()));
let log_for_hook = Arc::clone(&log);
let _ = try_register_error_hook(move |ctx| {
    log_for_hook.lock().unwrap()
        .push(format!("{}: {}", ctx.kind, ctx.caption));
});
```

The hook is now stored as `Box<dyn Fn(ErrorContext<'_>) + Send + Sync + 'static>`.
Pure function-pointer callers see no change; closure-capturing
callers gain a new capability.

### 7. `register_error_hook` deprecated in favour of `try_register_error_hook`

**Before (`0.9.x`):**

```rust
register_error_hook(my_hook);   // silently drops registration failure
```

**After (`1.0.0`):**

```rust
let _ = try_register_error_hook(my_hook);   // explicit
```

`register_error_hook` still works (it forwards to
`try_register_error_hook` and discards the result) but is
marked `#[deprecated]`. It will be removed in `2.0`.

### 8. `Result<T>` deprecated in favour of `AppResult<T>`

**Before (`0.9.x`):**

```rust
use error_forge::Result;          // shadows std::result::Result!

fn load() -> Result<()> { ... }
```

**After (`1.0.0`):**

```rust
use error_forge::AppResult;       // does not shadow

fn load() -> AppResult<()> { ... }
```

The `Result<T>` alias still exists but is `#[deprecated]`.
`error_forge::AppResult<T>` is the same type with a name that
doesn't collide with `std::result::Result` in glob imports.

### 9. `paste` runtime dep replaced with `pastey`

This is internal — `define_errors!` previously expanded to
`paste::paste! { ... }` and now expands to
`error_forge::__private::pastey::paste! { ... }`. Users who
were carrying `paste` in their own `Cargo.toml` purely to
satisfy `define_errors!` can remove it; `error-forge` carries
the dep transitively.

### 10. `atty` dep removed, MSRV declared `1.81`

Shipped in `0.9.8`, summarised here for completeness:

- `atty` (unmaintained, RUSTSEC-2021-0145) replaced with
  `std::io::IsTerminal`.
- `once_cell` dep + `thread-safety` feature removed (dead).
- MSRV formally declared as `1.81.0` and verified in CI.

If you're upgrading directly from `0.9.7` (or earlier), see
the `[0.9.8]` CHANGELOG entry for the full delta.

## Upgrading from `0.6.x` to `0.9.x`

Several breaking changes shipped through the `0.9.0` and `0.9.6`
work. The two large ones:

### Error metadata via `ForgeError`

`0.9.0` introduced the `ForgeError` trait with `kind`,
`caption`, `is_retryable`, etc. If you were on `0.6.x` you only
had `Display + Error`. Upgrade path:

- Use `define_errors!` or `#[derive(ModError)]` to generate the
  metadata for your existing enums, OR
- Implement `ForgeError` manually on each error type.

### `recovery` module

`0.9.6` added `RetryPolicy`, `CircuitBreaker`, and the three
backoff strategies. There's no migration step — these are
pure additions.

## Upgrading from `0.1.x` / `0.2.x` / `0.5.x` to current

The `0.5.0` release was the first public release. Anything
older than `0.6.x` should be considered a research version;
upgrade via the `0.6.x → 0.9.x → 1.0.0` path described above.
