<div align="center">
   <img width="120px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <h1>
        <strong>Error Forge</strong>
        <sup><br><sub>RUST ERROR FRAMEWORK</sub><br></sup>
    </h1>
        <a href="https://crates.io/crates/error-forge" alt="Error-Protocol on Crates.io"><img alt="Crates.io" src="https://img.shields.io/crates/v/error-forge"></a>
        <span>&nbsp;</span>
        <a href="https://crates.io/crates/error-forge" alt="Download Error-Forge"><img alt="Crates.io Downloads" src="https://img.shields.io/crates/d/error-forge?color=%230099ff"></a>
        <span>&nbsp;</span>
        <a href="https://docs.rs/error-forge" title="Error-Forge Documentation"><img alt="docs.rs" src="https://img.shields.io/docsrs/error-forge"></a>
        <span>&nbsp;</span>
        <a href="https://github.com/jamesgober/error-forge/actions"><img alt="GitHub CI" src="https://github.com/jamesgober/error-forge/actions/workflows/ci.yml/badge.svg"></a>
</div>
<br>

# Error Forge

Error Forge is a comprehensive, zero-dependency error management framework for Rust applications. It simplifies error handling through expressive macros, automatic trait implementations, and extensible error hooks for seamless integration with external logging systems.

## Features

- **Rich Error Types**: Define expressive error types with minimal boilerplate
- **ForgeError Trait**: Unified interface for all error types with contextual metadata
- **Declarative Macros**: Generate complete error enums with the `define_errors!` macro
- **Error Composition**: Combine errors from multiple modules with the `group!` macro
- **Derive Macros**: Quickly implement errors with `#[derive(ModError)]`
- **Console Formatting**: ANSI color formatting for terminal output with `ConsoleTheme`
- **Error Hooks**: Register callbacks for errors with severity level support
- **Zero External Dependencies**: Completely standalone with no third-party dependencies

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
error-forge = "0.6.2"
```

## Usage

### Basic Error Definition

```rust
use error_forge::define_errors;

define_errors! {
    pub enum DatabaseError {
        #[error(display = "Database connection failed: {}", message)]
        ConnectionFailed { message: String },
        
        #[error(display = "Query execution failed: {}", message)]
        QueryFailed { message: String, query: String },
        
        #[error(display = "Record not found with ID: {}", id)]
        RecordNotFound { id: String },
    }
}

// The macro automatically implements constructors and the ForgeError trait
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an error using the generated constructor
    let error = DatabaseError::connection_failed("Timeout after 30 seconds");
    
    // Use ForgeError methods
    println!("Error kind: {}", error.kind());
    println!("Caption: {}", error.caption());
    println!("Status code: {}", error.status_code());
    
    Err(Box::new(error))
}
```

### Error Composition with the `group!` Macro

```rust
use error_forge::{define_errors, group};

define_errors! {
    pub enum ApiError {
        #[error(display = "Invalid API key")]
        InvalidApiKey,
        
        #[error(display = "Rate limit exceeded")]
        RateLimitExceeded,
    }
}

define_errors! {
    pub enum ValidationError {
        #[error(display = "Required field {} is missing", field)]
        MissingField { field: String },
        
        #[error(display = "Field {} has invalid format", field)]
        InvalidFormat { field: String },
    }
}

// Combine errors from different modules into a single error type
group! {
    pub enum AppError {
        Api(ApiError),
        Validation(ValidationError),
        // Add more error types as needed
    }
}

fn validate_request() -> Result<(), AppError> {
    // Create a ValidationError
    let error = ValidationError::missing_field("username");
    
    // The error will be automatically converted to AppError
    Err(error.into())
}
```

### Using Derive Macro

```rust
use error_forge::ModError;

#[derive(Debug, ModError)]
#[module_error(kind = "AuthError")]
pub enum AuthError {
    #[error(display = "Invalid credentials")]
    InvalidCredentials,
    
    #[error(display = "Account locked: {}", reason)]
    AccountLocked { reason: String },
    
    #[error(display = "Session expired")]
    #[http_status(401)]
    SessionExpired,
}

fn login() -> Result<(), AuthError> {
    Err(AuthError::InvalidCredentials)
}
```

### Error Hooks for Logging Integration

```rust
use error_forge::{AppError, macros::{register_error_hook, ErrorLevel, ErrorContext}};

fn main() {
    // Register a hook for centralized error handling
    register_error_hook(|ctx| {
        match ctx.level {
            ErrorLevel::Info => println!("INFO: {} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Warning => println!("WARN: {} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Error => println!("ERROR: {} [{}]", ctx.caption, ctx.kind),
            ErrorLevel::Critical => {
                println!("CRITICAL: {} [{}]", ctx.caption, ctx.kind);
                
                // Send notifications for critical errors
                if ctx.is_fatal {
                    send_notification("Critical error occurred", ctx.caption);
                }
            }
        }
    });
    
    // This will trigger the hook
    let _error = AppError::config("Configuration file not found");
}

fn send_notification(level: &str, message: &str) {
    // Send notifications via your preferred channel
    println!("Notification sent: {} - {}", level, message);
}
```

### Console Formatting

```rust
use error_forge::{AppError, ConsoleTheme};

fn main() {
    // Create an error
    let error = AppError::config("Database configuration missing");
    
    // Set up console theme
    let theme = ConsoleTheme::new();
    
    // Print formatted error
    println!("{}", theme.format_error(&error));
    
    // Install panic hook for consistent formatting
    theme.install_panic_hook();
    
    // This panic will be formatted consistently with other errors
    panic!("Something went wrong!");
}
```

## Advanced Usage

For more detailed documentation and advanced usage examples, refer to the [API Documentation](docs/API.md).