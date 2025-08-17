<div align="center">
   <img width="120px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <h1>
        <strong>Error Forge</strong>
        <sup><br><sub>API REFERENCE</sub><br></sup>
    </h1>
</div>
<br>

## Table of Contents

- [Installation](#installation)
- [Core Traits and Types](#core-traits-and-types)
  - [ForgeError Trait](#forgeerror-trait)
  - [AsyncForgeError Trait](#asyncforgeerror-trait)
  - [Result Type](#result-type)
- [Macros](#macros)
  - [define_errors!](#define_errors)
  - [group!](#group)
- [Derive Macros](#derive-macros)
  - [ModError](#moderror)
- [Error Handling and Display](#error-handling-and-display)
  - [ConsoleTheme](#consoletheme)
  - [Error Hooks](#error-hooks)
  - [Panic Hook](#panic-hook)
- [Structured Context](#structured-context)
  - [ContextError](#contexterror)
  - [Context Methods](#context-methods)
- [Error Registry](#error-registry)
  - [ErrorRegistry](#errorregistry)
  - [Error Codes](#error-codes)
- [Error Collection](#error-collection)
  - [ErrorCollector](#errorcollector)
- [Error Recovery](#error-recovery)
  - [Backoff Strategies](#backoff-strategies)
  - [Circuit Breaker](#circuit-breaker)
  - [Retry Policy](#retry-policy)
  - [ForgeErrorRecovery](#forgeerrorrecovery)
- [Async Support](#async-support)
  - [Async Error Handling](#async-error-handling)
  - [Async Utilities](#async-utilities)
- [Examples](#examples)

<br><br>

## Installation

### Install Manually
```toml
[dependencies]
error-forge = "0.9.6"
```

### Install Using Cargo
```bash
cargo install error-forge
```

<br>

## Core Traits and Types

### ForgeError Trait

`ForgeError` is the central trait that defines the common interface for all errors in the Error Forge ecosystem.

**Signature:**
```rust
pub trait ForgeError: std::error::Error + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn caption(&self) -> &'static str;
    fn is_retryable(&self) -> bool;
    fn is_fatal(&self) -> bool;
    fn status_code(&self) -> u16;
    fn exit_code(&self) -> i32;
    fn user_message(&self) -> String;
    fn dev_message(&self) -> String;
    fn backtrace(&self) -> Option<&Backtrace>;
    fn register(&self);
}
```

**Methods:**

| Method | Return Type | Description |
|--------|-------------|--------------|
| `kind()` | `&'static str` | Returns the error kind, typically matching the enum variant name |
| `caption()` | `&'static str` | Returns a human-readable caption for the error |
| `is_retryable()` | `bool` | Returns true if the operation can be retried (default: false) |
| `is_fatal()` | `bool` | Returns true if the error is fatal and should terminate the program (default: true) |
| `status_code()` | `u16` | Returns an appropriate HTTP status code for the error (default: 500) |
| `exit_code()` | `i32` | Returns an appropriate process exit code for the error (default: 1) |
| `user_message()` | `String` | Returns a user-facing message that can be shown to end users |
| `dev_message()` | `String` | Returns a detailed technical message for developers/logs |
| `backtrace()` | `Option<&Backtrace>` | Returns a backtrace if available (default: None) |
| `register()` | `()` | Registers the error with the central error registry |

**Example:**
```rust
use error_forge::{ForgeError, define_errors};

// Define custom error with ForgeError implementation
define_errors! {
    #[derive(Debug)]
    pub enum MyError {
        #[error(display = "Failed to process: {message}")]
        #[kind(Process, retryable = true, status = 503)]
        Process { message: String },
    }
}

fn example() -> Result<(), Box<dyn ForgeError>> {
    let error = MyError::process("task interrupted");
    
    println!("Kind: {}", error.kind());  // "Process"
    println!("Retryable: {}", error.is_retryable());  // true
    println!("Status code: {}", error.status_code());  // 503
    
    Err(Box::new(error))
}
```

### AsyncForgeError Trait

`AsyncForgeError` extends the `ForgeError` trait to support asynchronous error handling in async contexts.

**Signature:**
```rust
#[async_trait]
pub trait AsyncForgeError: ForgeError {
    async fn from_async_result<T, E>(result: Result<T, E>) -> Result<T, Self>
    where
        E: std::error::Error + Send + 'static;
        
    async fn async_handle<F, T>(self, handler: F) -> Result<T, Self>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, Self>> + Send>> + Send,
        T: Send;
}
```

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `from_async_result` | `result: Result<T, E>` | `Result<T, Self>` | Converts an async result to the implementing error type |
| `async_handle` | `handler: F` | `Result<T, Self>` | Handles an error in an async context with the provided handler |

**Example:**
```rust
use error_forge::{define_errors, AsyncForgeError};
use async_trait::async_trait;

define_errors! {
    pub enum ApiError {
        #[error(display = "Request to {} failed: {}", endpoint, message)]
        #[kind(Request, retryable = true, status = 502)]
        RequestFailed { endpoint: String, message: String },
    }
}

#[async_trait]
impl AsyncForgeError for ApiError {
    // Implementation provided by the macro
}

async fn fetch_data(url: &str) -> Result<String, ApiError> {
    // Some HTTP request that might fail
    let response = reqwest::get(url).await
        .map_err(|e| ApiError::request_failed(url.to_string(), e.to_string()))?;
        
    if response.status().is_success() {
        let body = response.text().await
            .map_err(|e| ApiError::request_failed(url.to_string(), e.to_string()))?;
        Ok(body)
    } else {
        Err(ApiError::request_failed(
            url.to_string(),
            format!("Status: {}", response.status())
        ))
    }
}

async fn process_with_retry(url: &str) -> Result<String, ApiError> {
    let result = fetch_data(url).await;
    
    // Handle the error with retry logic
    if let Err(err) = result {
        if err.is_retryable() {
            println!("Retrying request to {}...", url);
            // Wait and retry
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            return fetch_data(url).await;
        }
        return Err(err);
    }
    
    result
}
```

### Result Type

`Result` is a type alias for the standard Result with an error type of `AppError`.

**Signature:**
```rust
pub type Result<T> = std::result::Result<T, error_forge::error::AppError>;
```

**Example:**
```rust
use error_forge::Result;

fn might_fail() -> Result<String> {
    // Some operation that might fail
    if true {
        Ok("Success".to_string())
    } else {
        Err(error_forge::AppError::config("Something went wrong"))
    }
}
```

<br>

## Macros

### define_errors!

The `define_errors!` macro creates rich error enum types with minimal boilerplate, automatically implementing `ForgeError` and other necessary traits.

**Syntax:**
```rust
define_errors! {
    #[attributes]
    pub enum ErrorName {
        #[error(display = "format string {param}")]
        #[kind(KindName, param1 = value1, param2 = value2, ...)]
        VariantName { param: Type, ... },
        
        // Additional variants...
    }
}
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `#[error(display = "...")]` | Attribute | Display format string for the error variant |
| `#[kind(...)]` | Attribute | Kind name and optional parameters |
| `KindName` | Identifier | Name of the error kind |
| `retryable` | `bool` | Whether the error is retryable |
| `status` | `u16` | HTTP status code for the error |
| `exit` | `i32` | Process exit code for the error |
| `fatal` | `bool` | Whether the error is fatal |

**Example:**
```rust
use error_forge::define_errors;

define_errors! {
    #[derive(Debug)]
    pub enum AppError {
        #[error(display = "Configuration error: {message}")]
        #[kind(Config, retryable = false, status = 500)]
        Config { message: String },
        
        #[error(display = "Database error: {message}")]
        #[kind(Database, retryable = true, status = 503)]
        Database { message: String },
        
        #[error(display = "Network request to {endpoint} failed")]
        #[kind(Network, retryable = true, status = 502)]
        Network { endpoint: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },
    }
}

// Use the generated constructor methods
let config_error = AppError::config("Missing configuration file");
let db_error = AppError::database("Connection timed out");
```

### group!

The `group!` macro composes multi-error enums with automatic `From` conversions for included error types.

**Syntax:**
```rust
group! {
    #[attributes]
    pub enum ParentError {
        ErrorType1(ErrorType1),
        ErrorType2(ErrorType2),
        // Additional wrapped error types...
        
        // Optional custom variants
        CustomVariant { field: Type, ... },
    }
}
```

**Parameters:**

| Parameter | Description |
|-----------|--------------|
| `ParentError` | Name of the parent error enum |
| `ErrorTypeN` | Error types to wrap |
| `CustomVariant` | Optional custom error variants |

**Example:**
```rust
use error_forge::{group, AppError};
use std::io;

// Define a custom error type
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

// Group multiple error types into a parent error
group! {
    #[derive(Debug)]
    pub enum ServiceError {
        // Include the AppError
        App(AppError),
        
        // Include io::Error
        Io(io::Error),
        
        // Include the custom DatabaseError
        Database(DatabaseError),
        
        // Custom variant
        Custom { message: String },
    }
}

// Now you can use From conversions automatically
fn example() -> Result<(), ServiceError> {
    // These will automatically convert to ServiceError
    let _: ServiceError = AppError::config("Missing config").into();
    let _: ServiceError = io::Error::new(io::ErrorKind::NotFound, "File not found").into();
    let _: ServiceError = DatabaseError::ConnectionFailed("Timeout".to_string()).into();
    
    Ok(())
}
```

<br>

## Derive Macros

### ModError

The `ModError` derive macro provides "lazy mode" error creation with minimal boilerplate, automatically implementing the `ForgeError` trait for a struct or enum.

**Attributes:**

| Attribute | Description |
|-----------|--------------|
| `#[error_prefix("Text")]` | Sets a prefix for error messages (applied to the type) |
| `#[error_display("Message")]` | Sets the display format for the variant |
| `#[error_retryable]` | Marks the variant as retryable |
| `#[error_http_status(code)]` | Sets the HTTP status code for the variant |
| `#[error_exit_code(code)]` | Sets the process exit code for the variant |

**Example:**
```rust
use error_forge::ModError;

#[derive(Debug, ModError)]
#[error_prefix("Database")]
pub enum DbError {
    #[error_display("Connection to {0} failed")]
    ConnectionFailed(String),

    #[error_display("Query execution failed: {reason}")]
    QueryFailed { reason: String },

    #[error_display("Transaction error")]
    #[error_http_status(400)]
    #[error_retryable]
    TransactionError,
}

// Use the error type
let connection_error = DbError::ConnectionFailed("localhost:5432".to_string());
println!("Error: {}", connection_error);  // "Database: Connection to localhost:5432 failed"
```

<br>

## Error Handling and Display

### ConsoleTheme

`ConsoleTheme` provides ANSI color formatting for error messages displayed in terminal environments.

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `new()` | None | `ConsoleTheme` | Creates a new theme with default colors |
| `plain()` | None | `ConsoleTheme` | Creates a new theme with no colors |
| `error()` | `text: &str` | `String` | Formats text with error color |
| `warning()` | `text: &str` | `String` | Formats text with warning color |
| `info()` | `text: &str` | `String` | Formats text with info color |
| `success()` | `text: &str` | `String` | Formats text with success color |
| `caption()` | `text: &str` | `String` | Formats text with caption color |
| `bold()` | `text: &str` | `String` | Formats text as bold |
| `dim()` | `text: &str` | `String` | Formats text as dim |
| `format_error()` | `err: &E` | `String` | Formats an error with structure |

**Example:**
```rust
use error_forge::{AppError, console_theme::{ConsoleTheme, print_error}};

fn example() {
    let error = AppError::config("Missing configuration file");
    
    // Using print_error helper
    print_error(&error);
    
    // Or with custom theming
    let theme = ConsoleTheme::new();
    println!("{}", theme.format_error(&error));
    
    // Using individual theme methods
    println!("{}", theme.error("This is an error"));
    println!("{}", theme.warning("This is a warning"));
}
```

### Error Hooks

Error Forge provides a centralized error hook mechanism to perform actions when errors are created, with support for different error levels and contexts.

**Types:**

| Type | Description |
|------|-------------|
| `ErrorLevel` | Enum representing error severity levels: `Info`, `Warning`, `Error`, `Critical` |
| `ErrorContext` | Struct containing error context: `caption`, `kind`, `level`, `is_fatal`, `is_retryable` |

**Functions:**

| Function | Parameters | Description |
|----------|------------|-------------|
| `register_error_hook()` | `callback: fn(ErrorContext)` | Register a callback function to be called when errors are created |

**Example:**
```rust
use error_forge::{AppError, macros::{register_error_hook, ErrorLevel, ErrorContext}};
use log::{info, warn, error, critical};

fn main() {
    // Register a hook that maps error levels to your logging system
    register_error_hook(|ctx| {
        // Map to appropriate log levels
        match ctx.level {
            ErrorLevel::Info => info!("{} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Warning => warn!("{} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Error => error!("{} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Critical => {
                critical!("{} [{}]", ctx.caption, ctx.kind);
                // Send alerts for critical errors
                if ctx.is_fatal {
                    send_alert("CRITICAL ERROR", ctx.caption);
                }
            }
        }
    });
    
    // These will trigger the hook with different levels
    let _config_error = AppError::config("Missing configuration"); // Error level
    let _network_error = AppError::network("api.example.com", None); // Error or Warning level
}

fn send_alert(level: &str, message: &str) {
    // Send notifications via email, SMS, or monitoring service
    println!("ALERT SENT: {} - {}", level, message);
}
```

### Panic Hook

Error Forge provides a customizable panic hook that formats panics using the `ConsoleTheme`.

**Functions:**

| Function | Parameters | Description |
|----------|------------|-------------|
| `install_panic_hook()` | None | Installs a panic hook that formats panics using the ConsoleTheme |

**Example:**
```rust
use error_forge::console_theme::install_panic_hook;

fn main() {
    // Install the custom panic hook
    install_panic_hook();
    
    // This panic will be formatted with the ConsoleTheme
    panic!("Something went terribly wrong!");
}
```

<br>

## Structured Context

Error Forge provides structured context support for wrapping errors with additional information.

### ContextError

`ContextError` is a wrapper type that adds context information to any error type.

**Signature:**
```rust
pub struct ContextError<E> {
    context: String,
    source: E,
}
```

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `new()` | `source: E, context: String` | `ContextError<E>` | Creates a new context error wrapping the source error |
| `context()` | None | `&str` | Returns the context message |
| `source()` | None | `&E` | Returns a reference to the source error |
| `into_source()` | None | `E` | Consumes the context error and returns the source error |

### Context Methods

Error Forge extends `Result` with context methods for easy error wrapping.

**Extension Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `context()` | `context: &str` | `Result<T, ContextError<E>>` | Wraps the error with context |
| `with_context()` | `f: FnOnce() -> C` | `Result<T, ContextError<E>>` | Wraps the error with lazily evaluated context |

**Example:**
```rust
use error_forge::{define_errors, context::ContextError};
use std::fs::File;
use std::io::Read;

define_errors! {
    pub enum FileError {
        #[error(display = "Failed to open file")]
        OpenFailed,
        
        #[error(display = "Failed to read file")]
        ReadFailed,
    }
}

fn read_config() -> Result<String, ContextError<FileError>> {
    // Add context to the error
    let mut file = File::open("config.json")
        .map_err(|_| FileError::OpenFailed)
        .context("Opening configuration file")?;
        
    // Add context with a closure for dynamic messages
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|_| FileError::ReadFailed)
        .with_context(|| format!("Reading {} bytes from config", file.metadata().map_or(0, |m| m.len())))?;
        
    Ok(contents)
}
```

<br>

## Error Registry

Error Forge provides a central registry for errors with support for error codes and documentation URLs.

### ErrorRegistry

`ErrorRegistry` is a global registry for tracking error types and their metadata.

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `register()` | `kind: &str, metadata: ErrorMetadata` | `()` | Registers an error with its metadata |
| `get()` | `kind: &str` | `Option<&ErrorMetadata>` | Gets metadata for an error kind |
| `register_url_format()` | `format: String` | `()` | Sets the URL format for documentation links |

### Error Codes

Error Forge supports numeric error codes for errors registered in the `ErrorRegistry`.

**Example:**
```rust
use error_forge::{define_errors, registry::{ErrorRegistry, ErrorMetadata}};

// Configure the error registry
fn configure_registry() {
    // Set URL format for documentation links
    ErrorRegistry::register_url_format("https://example.com/errors/{code}".to_string());
    
    // Register errors with codes and categories
    ErrorRegistry::register("Config", ErrorMetadata {
        code: 1001,
        category: "configuration",
        description: "Configuration-related errors",
    });
    
    ErrorRegistry::register("Database", ErrorMetadata {
        code: 2001,
        category: "database",
        description: "Database access and query errors",
    });
}

// Define errors that will use the registry
define_errors! {
    pub enum AppError {
        #[error(display = "Configuration error: {message}")]
        Config { message: String },
        
        #[error(display = "Database error: {message}")]
        Database { message: String },
    }
}

fn example() {
    configure_registry();
    
    let error = AppError::config("Missing database URL");
    
    // Get the error code from the registry
    if let Some(metadata) = ErrorRegistry::get(error.kind()) {
        println!("Error code: {}", metadata.code);  // 1001
        println!("Category: {}", metadata.category);  // "configuration"
        println!("Documentation: {}", metadata.documentation_url());  // https://example.com/errors/1001
    }
}
```

<br>

## Error Collection

Error Forge provides a system for collecting multiple non-fatal errors instead of returning on the first error.

### ErrorCollector

`ErrorCollector` accumulates errors during processing for batch handling.

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `new()` | None | `ErrorCollector<E>` | Creates a new empty error collector |
| `push()` | `error: E` | `()` | Adds an error to the collection |
| `errors()` | None | `&[E]` | Returns a slice of all collected errors |
| `is_empty()` | None | `bool` | Returns true if no errors have been collected |
| `into_result()` | None | `Result<(), E>` | Returns Ok if no errors, or Err with the first error |
| `into_error()` | None | `Option<E>` | Consumes the collector and returns the first error if any |

**Example:**
```rust
use error_forge::{define_errors, collector::ErrorCollector};

define_errors! {
    pub enum ValidationError {
        #[error(display = "Field '{}' is required", field)]
        Required { field: String },
        
        #[error(display = "Value '{}' for field '{}' is invalid", value, field)]
        InvalidValue { field: String, value: String },
    }
}

struct Form {
    username: String,
    email: String,
    age: Option<u32>,
}

fn validate_form(form: &Form) -> Result<(), ValidationError> {
    let mut collector = ErrorCollector::new();
    
    // Validate username
    if form.username.is_empty() {
        collector.push(ValidationError::required("username"));
    } else if form.username.len() < 3 {
        collector.push(ValidationError::invalid_value("username", &form.username));
    }
    
    // Validate email
    if form.email.is_empty() {
        collector.push(ValidationError::required("email"));
    } else if !form.email.contains('@') {
        collector.push(ValidationError::invalid_value("email", &form.email));
    }
    
    // Return all collected errors at once
    collector.into_result()
}
```

## Error Recovery

Error Forge provides resilience patterns for handling errors in production systems, including retry policies with various backoff strategies and circuit breakers to prevent cascading failures.

### Backoff Strategies

Backoff strategies determine how long to wait between retry attempts.

**Backoff Trait:**

```rust
pub trait Backoff: Send + Sync + 'static {
    fn next_delay(&self, attempt: usize) -> Duration;
}
```

**Available Implementations:**

| Strategy | Description |
|----------|-------------|
| `ExponentialBackoff` | Increases delay exponentially based on attempt number with optional jitter |
| `LinearBackoff` | Increases delay linearly based on attempt number with optional jitter |
| `FixedBackoff` | Uses a constant delay between retry attempts with optional jitter |

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `with_initial_delay()` | `delay_ms: u64` | `Self` | Sets the initial delay in milliseconds |
| `with_max_delay()` | `max_delay_ms: u64` | `Self` | Sets the maximum delay in milliseconds |
| `with_factor()` | `factor: f64` | `Self` | Sets the multiplication factor (for exponential/linear) |
| `with_jitter()` | `jitter: f64` | `Self` | Sets jitter factor (0.0-1.0) to randomize delays |

**Example:**
```rust
use error_forge::recovery::{ExponentialBackoff, LinearBackoff, FixedBackoff, Backoff};
use std::time::Duration;

// Exponential backoff: 100ms, 200ms, 400ms, 800ms, ...
let exp_backoff = ExponentialBackoff::new()
    .with_initial_delay(100)
    .with_max_delay(10000)
    .with_factor(2.0)
    .with_jitter(0.1);

// Linear backoff: 100ms, 200ms, 300ms, 400ms, ...
let linear_backoff = LinearBackoff::new()
    .with_initial_delay(100)
    .with_max_delay(5000)
    .with_factor(100)
    .with_jitter(0.05);

// Fixed backoff: 200ms, 200ms, 200ms, ...
let fixed_backoff = FixedBackoff::new()
    .with_delay(200)
    .with_jitter(0.1);

// Using the backoff strategies
let delay1 = exp_backoff.next_delay(0);  // ~100ms (with jitter)
let delay2 = exp_backoff.next_delay(1);  // ~200ms (with jitter)
let delay3 = exp_backoff.next_delay(2);  // ~400ms (with jitter)
```

### Circuit Breaker

Circuit Breaker prevents repeated calls to failing operations and allows the system to recover.

**States:**

| State | Description |
|-------|-------------|
| `Closed` | Normal operation, calls pass through |
| `Open` | Circuit is tripped, calls fail fast |
| `HalfOpen` | Testing if the system has recovered |

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `new()` | `config: CircuitBreakerConfig` | `CircuitBreaker` | Creates a new circuit breaker with configuration |
| `execute()` | `operation: F` | `Result<T, E>` | Executes an operation through the circuit breaker |
| `state()` | None | `CircuitState` | Returns the current state of the circuit breaker |
| `reset()` | None | `()` | Resets the circuit breaker to the closed state |

**CircuitBreakerConfig:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `default()` | None | `CircuitBreakerConfig` | Creates a default configuration |
| `with_failure_threshold()` | `threshold: u32` | `Self` | Number of failures before opening |
| `with_success_threshold()` | `threshold: u32` | `Self` | Number of successes in half-open before closing |
| `with_reset_timeout()` | `timeout: Duration` | `Self` | Time before transitioning from open to half-open |

**Example:**
```rust
use error_forge::recovery::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use std::time::Duration;

// Create a circuit breaker configuration
let config = CircuitBreakerConfig::default()
    .with_failure_threshold(3)   // Open after 3 consecutive failures
    .with_success_threshold(2)   // Close after 2 consecutive successes in half-open
    .with_reset_timeout(Duration::from_secs(30));  // Try again after 30 seconds

// Create a circuit breaker
let circuit_breaker = CircuitBreaker::new(config);

// Execute an operation through the circuit breaker
let result = circuit_breaker.execute(|| {
    // Operation that might fail
    database_operation()
});

// Check the current state
if circuit_breaker.state() == CircuitState::Open {
    println!("Circuit is open, service is unavailable");
}
```

### Retry Policy

Retry Policy combines predicate logic with backoff strategies for controlled retries.

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `new_exponential()` | None | `RetryPolicy` | Creates a new policy with exponential backoff |
| `new_linear()` | None | `RetryPolicy` | Creates a new policy with linear backoff |
| `new_fixed()` | None | `RetryPolicy` | Creates a new policy with fixed backoff |
| `with_max_retries()` | `max_retries: usize` | `Self` | Sets the maximum number of retry attempts |
| `with_initial_delay()` | `delay_ms: u64` | `Self` | Sets the initial delay in milliseconds |
| `with_max_delay()` | `delay_ms: u64` | `Self` | Sets the maximum delay in milliseconds |
| `with_jitter()` | `jitter: f64` | `Self` | Sets jitter factor (0.0-1.0) to randomize delays |
| `with_predicate()` | `predicate: P` | `Self` | Sets a retry predicate function |
| `forge_executor()` | None | `RetryExecutor<E>` | Gets an executor to run operations with this policy |

**RetryExecutor:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `retry()` | `operation: F` | `Result<T, E>` | Runs an operation with retries based on the policy |

**Example:**
```rust
use error_forge::recovery::RetryPolicy;
use std::{thread, time::Duration};

// Create a retry policy with exponential backoff
let retry_policy = RetryPolicy::new_exponential()
    .with_max_retries(3)
    .with_initial_delay(100)
    .with_max_delay(5000)
    .with_jitter(0.1)
    .with_predicate(|err: &MyError| err.is_retryable());

// Execute an operation with retries
let result = retry_policy.forge_executor().retry(|| {
    // Operation that might fail
    make_http_request("https://api.example.com")
});

// For async operations
let result = async {
    retry_policy.forge_executor().retry(|| async {
        make_async_http_request("https://api.example.com").await
    }).await
}.await;
```

### ForgeErrorRecovery

Extension trait that adds recovery capabilities to `ForgeError` types.

**Methods:**

| Method | Parameters | Return Type | Description |
|--------|------------|-------------|-------------|
| `create_retry_policy()` | `max_retries: usize` | `RetryPolicy` | Creates a retry policy optimized for this error type |
| `retry()` | `max_retries: usize, operation: F` | `Result<T, E>` | Executes a fallible operation with retries |

**Example:**
```rust
use error_forge::{define_errors, recovery::ForgeErrorRecovery};

define_errors! {
    pub enum ServiceError {
        #[error(display = "Request failed: {}", message)]
        #[kind(Request, retryable = true, status = 500)]
        RequestFailed { message: String },
        
        #[error(display = "Timeout: {}", message)]
        #[kind(Timeout, retryable = true, status = 504)]
        Timeout { message: String },
    }
}

// Implement the recovery trait
impl ForgeErrorRecovery for ServiceError {}

// Using the retry capabilities
fn make_request_with_retry() -> Result<String, ServiceError> {
    // Create a dummy error to use its retry method
    let error_template = ServiceError::request_failed("Template");
    
    // Retry the operation up to 3 times
    error_template.retry(3, || {
        match make_service_call() {
            Ok(response) => Ok(response),
            Err(e) => Err(ServiceError::request_failed(e.to_string()))
        }
    })
}

fn make_service_call() -> Result<String, std::io::Error> {
    // Simulated service call
    Ok("Response data".to_string())
}
```

<br>

## Async Support

Error Forge provides comprehensive support for asynchronous error handling in async Rust applications.

### Async Error Handling

The async error handling system is built around the `AsyncForgeError` trait (which extends `ForgeError`) and integrates with the `async-trait` crate for seamless async/await support.

**Core Components:**

| Component | Description |
|-----------|-------------|
| `AsyncForgeError` trait | Base trait for async error handling |
| `from_async_result` method | Converts async results to error types |
| `async_handle` method | Processes errors in an async context |

**Implementing AsyncForgeError:**

```rust
use error_forge::{define_errors, AsyncForgeError};
use async_trait::async_trait;

define_errors! {
    pub enum AsyncError {
        #[error(display = "Database error: {}", message)]
        #[kind(Database, retryable = true, status = 503)]
        DbError { message: String },
    }
}

// The AsyncForgeError implementation is automatically generated when
// you use define_errors! with async enabled in your features
#[async_trait]
impl AsyncForgeError for AsyncError {}
```

### Async Utilities

Error Forge provides utilities specifically designed for async contexts.

**Key Async Functions:**

| Function | Description |
|----------|-------------|
| `async_handle` | Processes errors in an async context with a handler function |
| `from_async_result` | Converts an async Result into your error type |

**Working with Async Results:**

```rust
use error_forge::{define_errors, AsyncForgeError};
use async_trait::async_trait;

define_errors! {
    pub enum ApiError {
        #[error(display = "API request failed: {}", message)]
        #[kind(Api, retryable = true, status = 502)]
        RequestFailed { message: String },
    }
}

#[async_trait]
impl AsyncForgeError for ApiError {}

async fn fetch_external_data() -> Result<String, reqwest::Error> {
    // External API call that returns a Result
    reqwest::get("https://api.example.com/data").await?.text().await
}

async fn process_data() -> Result<String, ApiError> {
    // Convert external error type to our ApiError
    let data = ApiError::from_async_result(fetch_external_data().await)
        .await?
        .trim()
        .to_string();
    
    Ok(data)
}
```

**Combining with Recovery Patterns:**

```rust
use error_forge::{define_errors, AsyncForgeError, recovery::ForgeErrorRecovery};
use async_trait::async_trait;

define_errors! {
    pub enum NetworkError {
        #[error(display = "Connection failed: {}", message)]
        #[kind(Connection, retryable = true, status = 503)]
        ConnectionFailed { message: String },
    }
}

#[async_trait]
impl AsyncForgeError for NetworkError {}
impl ForgeErrorRecovery for NetworkError {}

async fn fetch_with_retry() -> Result<String, NetworkError> {
    // Create retry policy with exponential backoff
    let retry_policy = NetworkError::connection_failed("dummy")
        .create_retry_policy(3)
        .with_initial_delay(100)
        .with_max_delay(2000)
        .with_jitter(0.2);
    
    // Use retry policy with async operation
    retry_policy.forge_executor()
        .retry(|| async {
            match make_request().await {
                Ok(data) => Ok(data),
                Err(e) => Err(NetworkError::connection_failed(e.to_string()))
            }
        })
        .await
}

async fn make_request() -> Result<String, std::io::Error> {
    // Simulated async network request
    Ok("Response data".to_string())
}
```

<br>

## Examples

### Basic Error Definition

```rust
use error_forge::{define_errors, ForgeError};

// Define our error type
define_errors! {
    #[derive(Debug)]
    pub enum AppError {
        #[error(display = "Configuration error: {message}")]
        #[kind(Config, retryable = false, status = 500)]
        Config { message: String },
        
        #[error(display = "Database error: {message}")]
        #[kind(Database, retryable = true, status = 503)]
        Database { message: String },
    }
}

// Use the error type
fn main() -> Result<(), AppError> {
    if true {
        return Err(AppError::config("Missing configuration"));
    }
    Ok(())
}
```

### Error Groups

```rust
use error_forge::{group, AppError};
use std::io;

// Define module-specific error types
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Operation failed: {0}")]
    Failed(String),
}

// Group errors into a parent type
group! {
    #[derive(Debug)]
    pub enum ServiceError {
        App(AppError),
        Io(io::Error),
        Module(ModuleError)
    }
}

// Now you can use all error types with automatic conversions
fn example() -> Result<(), ServiceError> {
    let result = std::fs::read_to_string("config.toml")
        .map_err(ServiceError::from)?;  // io::Error -> ServiceError
        
    if result.is_empty() {
        return Err(AppError::config("Empty config file").into());  // AppError -> ServiceError
    }
    
    Ok(())
}
```

### Derive Macro Usage

```rust
use error_forge::ModError;

#[derive(Debug, ModError)]
#[error_prefix("API")]
pub enum ApiError {
    #[error_display("Request to {0} failed with status {1}")]
    RequestFailed(String, u16),
    
    #[error_display("Rate limit exceeded")]
    #[error_http_status(429)]
    #[error_retryable]
    RateLimited,
    
    #[error_display("Authentication failed: {reason}")]
    AuthFailed { reason: String },
}

// Use the error type with automatically implemented methods
fn example() {
    let error = ApiError::RequestFailed("https://api.example.com".to_string(), 404);
    
    println!("Error: {}", error);  // "API: Request to https://api.example.com failed with status 404"
    println!("Kind: {}", error.kind());  // "RequestFailed"
    println!("Retryable: {}", error.is_retryable());  // false
    
    let rate_error = ApiError::RateLimited;
    println!("Retryable: {}", rate_error.is_retryable());  // true
    println!("Status: {}", rate_error.status_code());  // 429
}
```

### Formatted Error Output

```rust
use error_forge::{AppError, console_theme::{ConsoleTheme, print_error}};

fn main() {
    // Create an error
    let error = AppError::config("Configuration file not found");
    
    // Print with default formatting
    print_error(&error);
    
    // Or with custom theme
    let theme = ConsoleTheme::new();
    println!("{}", theme.format_error(&error));
}
```

### Using Error Hooks

```rust
use error_forge::{AppError, macros::register_error_hook};
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    // Setup a hook that logs errors to a file
    register_error_hook(|message| {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("error_log.txt")
            .unwrap();
            
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    });
    
    // This will trigger the hook and log to the file
    let _error = AppError::config("Missing database connection string");
}
```
