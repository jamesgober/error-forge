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
- [Examples](#examples)

<br><br>

## Installation

### Install Manually
```toml
[dependencies]
error-forge = "0.6.0"
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
