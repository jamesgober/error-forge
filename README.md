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

**Error Forge** is a comprehensive error management framework for Rust applications that combines robust error handling with resilience patterns. It provides both synchronous and asynchronous error management capabilities, recovery patterns like circuit breakers and retry policies with backoff strategies, and error collection mechanisms. Error Forge simplifies error handling through expressive macros, automatic trait implementations, extensible error hooks, and resilience patterns designed for enterprise-grade applications.

## Features

- **Rich Error Types**: Define expressive error types with minimal boilerplate
- **ForgeError Trait**: Unified interface for all error types with contextual metadata
- **Async Error Support**: First-class support for async applications with `AsyncForgeError` trait
- **Error Recovery Patterns**:
  - **Circuit Breaker**: Prevent cascading failures with intelligent state management
  - **Retry Policies**: Configurable retry mechanisms with predicate support
  - **Backoff Strategies**: Exponential, linear, and fixed backoff with optional jitter
- **Declarative Macros**: Generate complete error enums with the `define_errors!` macro
- **Error Composition**: Combine errors from multiple modules with the `group!` macro
- **Derive Macros**: Quickly implement errors with `#[derive(ModError)]`
- **Console Formatting**: ANSI color formatting for terminal output with `ConsoleTheme`
- **Error Hooks**: Thread-safe error hook system using `OnceLock`
- **Structured Context**: Error wrapping with context via `context()` and `with_context()` methods
- **Error Registry**: Support for error codes and documentation URLs
- **Non-fatal Error Collection**: Collect and process multiple errors with `ErrorCollector`
- **Logging Integration**: Optional integration with `log` and `tracing` crates
- **Cross-Platform**: Full support for Linux, macOS, and Windows
- **Zero Core Dependencies**: Core functionality has no third-party dependencies

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
error-forge = "0.9.6"
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

### Structured Context Support

```rust
use error_forge::{define_errors, context::ContextError};
use std::fs::File;

define_errors! {
    pub enum FileError {
        #[error(display = "Failed to open file")]
        OpenFailed,
        
        #[error(display = "Failed to read file")]
        ReadFailed,
    }
}

fn read_config_file(path: &str) -> Result<String, ContextError<FileError>> {
    // Open the file, adding context to any error
    let mut file = File::open(path)
        .map_err(|_| FileError::OpenFailed)
        .with_context(format!("Opening config file: {}", path))?;
        
    // Read the file, again with context
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|_| FileError::ReadFailed)
        .context("Reading configuration data")?;
        
    Ok(contents)
}

fn main() {
    match read_config_file("/etc/app/config.json") {
        Ok(config) => println!("Config loaded: {} bytes", config.len()),
        Err(e) => {
            // Prints both the context and the underlying error
            println!("Error: {}", e);
            
            // Access the original error
            println!("Original error: {}", e.source());
        }
    }
}
```

### Error Collection

```rust
use error_forge::{define_errors, collector::ErrorCollector};

define_errors! {
    pub enum ValidationError {
        #[error(display = "Field '{}' is required", field)]
        Required { field: String },
        
        #[error(display = "Field '{}' must be a valid email", field)]
        InvalidEmail { field: String },
    }
}

fn validate_form(data: &FormData) -> Result<(), ValidationError> {
    let mut collector = ErrorCollector::new();
    
    // Collect validation errors without returning early
    if data.name.is_empty() {
        collector.push(ValidationError::required("name"));
    }
    
    if data.email.is_empty() {
        collector.push(ValidationError::required("email"));
    } else if !is_valid_email(&data.email) {
        collector.push(ValidationError::invalid_email("email"));
    }
    
    // Return all errors at once
    collector.into_result()
}
```

## Advanced Usage

### Microservice Error Management with Resilience

This example demonstrates a complete error management system for a microservice architecture with resilience patterns.

```rust
use error_forge::{define_errors, group, recovery::{CircuitBreaker, CircuitBreakerConfig, RetryPolicy, ForgeErrorRecovery}};
use std::time::Duration;

// Define domain-specific error types for different services
define_errors! {
    pub enum DatabaseError {
        #[error(display = "Failed to connect to database: {}", message)]
        #[kind(Connection, retryable = true, status = 503)]
        ConnectionFailed { message: String },
        
        #[error(display = "Query execution failed: {}", message)]
        #[kind(Query, retryable = true, status = 500)]
        QueryFailed { message: String },
        
        #[error(display = "Transaction failed: {}", message)]
        #[kind(Transaction, retryable = true, status = 500)]
        TransactionFailed { message: String },
    }
}

define_errors! {
    pub enum ApiError {
        #[error(display = "External API request to {} failed: {}", endpoint, message)]
        #[kind(Request, retryable = true, status = 502)]
        RequestFailed { endpoint: String, message: String },
        
        #[error(display = "Rate limit exceeded for endpoint: {}", endpoint)]
        #[kind(RateLimit, retryable = true, status = 429)]
        RateLimited { endpoint: String },
        
        #[error(display = "Authentication failed: {}", message)]
        #[kind(Auth, retryable = false, status = 401)]
        AuthFailed { message: String },
    }
}

define_errors! {
    pub enum CacheError {
        #[error(display = "Failed to connect to cache server: {}", message)]
        #[kind(Connection, retryable = true, status = 503)]
        ConnectionFailed { message: String },
        
        #[error(display = "Cache operation failed: {}", message)]
        #[kind(Operation, retryable = true, status = 500)]
        OperationFailed { message: String },
    }
}

// Group all service errors into a single application error type
group! {
    pub enum ServiceError {
        Database(DatabaseError),
        Api(ApiError),
        Cache(CacheError),
    }
}

// Implement extended recovery capabilities
impl ForgeErrorRecovery for ServiceError {}

async fn get_user_data(user_id: &str) -> Result<UserData, ServiceError> {
    // Create a circuit breaker for database operations
    let db_circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::default()
        .with_failure_threshold(3)
        .with_reset_timeout(Duration::from_secs(30)));
    
    // Create a retry policy for database operations
    let db_retry_policy = RetryPolicy::new_exponential()
        .with_max_retries(3)
        .with_initial_delay(100)
        .with_max_delay(2000)
        .with_jitter(0.1);
    
    // Execute database query with resilience patterns
    let user = db_circuit_breaker.execute(|| async {
        db_retry_policy.forge_executor().retry(|| async {
            match database::query_user(user_id).await {
                Ok(user) => Ok(user),
                Err(e) => Err(DatabaseError::query_failed(e.to_string()).into())
            }
        }).await
    }).await?;
    
    // Create a circuit breaker for API operations
    let api_circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::default()
        .with_failure_threshold(5)
        .with_reset_timeout(Duration::from_secs(60)));
        
    // Fetch additional user data from external API with resilience patterns
    let user_preferences = api_circuit_breaker.execute(|| async {
        // Create a retry policy for API calls with appropriate backoff
        let api_retry = RetryPolicy::new_exponential()
            .with_max_retries(3)
            .with_initial_delay(200)
            .with_max_delay(5000);
            
        api_retry.forge_executor().retry(|| async {
            match external_api::fetch_user_preferences(user_id).await {
                Ok(prefs) => Ok(prefs),
                Err(e) => {
                    if e.contains("rate limit") {
                        Err(ApiError::rate_limited("preferences-api.example.com").into())
                    } else {
                        Err(ApiError::request_failed(
                            "preferences-api.example.com",
                            e.to_string()
                        ).into())
                    }
                }
            }
        }).await
    }).await?;
    
    // Combine data and return
    Ok(UserData {
        profile: user,
        preferences: user_preferences,
    })
}

// Mock types and functions for the example
struct UserData {
    profile: User,
    preferences: UserPreferences,
}

struct User { /* user fields */ }
struct UserPreferences { /* preferences fields */ }

mod database {
    use super::*;
    pub async fn query_user(_id: &str) -> Result<User, String> {
        // Mock implementation
        Ok(User {})
    }
}

mod external_api {
    use super::*;
    pub async fn fetch_user_preferences(_id: &str) -> Result<UserPreferences, String> {
        // Mock implementation
        Ok(UserPreferences {})
    }
}
```

### Comprehensive Error Collection for Validation

This example shows how to use the error collector for complex data validation scenarios:

```rust
use error_forge::{define_errors, collector::ErrorCollector};
use std::collections::HashMap;

define_errors! {
    pub enum ValidationError {
        #[error(display = "Field '{}' is required", field)]
        #[kind(Required, status = 400)]
        Required { field: String },
        
        #[error(display = "Field '{}' has invalid format: {}", field, message)]
        #[kind(Format, status = 400)]
        InvalidFormat { field: String, message: String },
        
        #[error(display = "Field '{}' has invalid value: {}", field, message)]
        #[kind(Value, status = 400)]
        InvalidValue { field: String, message: String },
        
        #[error(display = "Reference to '{}' not found", reference)]
        #[kind(Reference, status = 400)]
        InvalidReference { reference: String },
    }
}

struct UserProfile {
    username: String,
    email: Option<String>,
    age: Option<u8>,
    country: Option<String>,
    preferences: HashMap<String, String>,
}

fn validate_user_profile(profile: &UserProfile) -> Result<(), ValidationError> {
    let mut collector = ErrorCollector::new();
    
    // Required field checks
    if profile.username.is_empty() {
        collector.push(ValidationError::required("username"));
    } else if profile.username.len() < 3 || profile.username.len() > 30 {
        collector.push(ValidationError::invalid_format(
            "username", 
            "Username must be between 3 and 30 characters long"
        ));
    }
    
    // Format validations
    if let Some(email) = &profile.email {
        if !email.contains('@') || !email.contains('.') {
            collector.push(ValidationError::invalid_format(
                "email", 
                "Email must contain '@' and domain"
            ));
        }
    }
    
    // Value range validations
    if let Some(age) = profile.age {
        if age < 13 {
            collector.push(ValidationError::invalid_value(
                "age", 
                "User must be at least 13 years old"
            ));
        }
    }
    
    // Reference validations
    if let Some(country) = &profile.country {
        let valid_countries = vec!["US", "CA", "UK", "AU", "DE", "FR"];
        if !valid_countries.contains(&country.as_str()) {
            collector.push(ValidationError::invalid_reference(
                format!("country '{}'", country)
            ));
        }
    }
    
    // Complex preference validations
    for (key, value) in &profile.preferences {
        match key.as_str() {
            "theme" => {
                let valid_themes = vec!["light", "dark", "system"];
                if !valid_themes.contains(&value.as_str()) {
                    collector.push(ValidationError::invalid_value(
                        "preferences.theme", 
                        format!("'{}' is not a valid theme", value)
                    ));
                }
            },
            "notifications" => {
                let valid_values = vec!["all", "important", "none"];
                if !valid_values.contains(&value.as_str()) {
                    collector.push(ValidationError::invalid_value(
                        "preferences.notifications", 
                        format!("'{}' is not a valid notification setting", value)
                    ));
                }
            },
            _ => {
                collector.push(ValidationError::invalid_reference(
                    format!("preference '{}'", key)
                ));
            }
        }
    }
    
    // Return all validation errors at once
    collector.into_result()
}
```

For more detailed documentation and additional advanced usage examples, refer to the [API Documentation](docs/API.md).