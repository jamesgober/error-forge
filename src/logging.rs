use std::sync::OnceLock;
use crate::error::ForgeError;
use crate::macros::ErrorLevel;

/// Trait for error logging adapters
/// 
/// Implement this trait to create a custom logger for error-forge
/// that integrates with your logging system
pub trait ErrorLogger: Send + Sync + 'static {
    /// Log an error with the given level
    fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel);
    
    /// Log a message with the given level
    fn log_message(&self, message: &str, level: ErrorLevel);
    
    /// Called when a panic occurs (if panic hook is registered)
    fn log_panic(&self, info: &std::panic::PanicHookInfo);
}

// The global error logger
static ERROR_LOGGER: OnceLock<Box<dyn ErrorLogger>> = OnceLock::new();

/// Register a logger for errors
/// 
/// Only one logger can be registered at a time.
/// If a logger is already registered, this will return an error.
pub fn register_logger(logger: impl ErrorLogger) -> Result<(), &'static str> {
    let boxed = Box::new(logger);
    match ERROR_LOGGER.set(boxed) {
        Ok(_) => Ok(()),
        Err(_) => Err("Error logger already registered"),
    }
}

/// Get the current logger, if one is registered
pub fn logger() -> Option<&'static dyn ErrorLogger> {
    ERROR_LOGGER.get().map(|boxed| boxed.as_ref())
}

/// Log an error with the appropriate level
pub fn log_error(error: &dyn ForgeError) {
    if let Some(logger) = logger() {
        let level = if error.is_fatal() {
            ErrorLevel::Critical
        } else if !error.is_retryable() {
            ErrorLevel::Error
        } else {
            ErrorLevel::Warning
        };
        
        logger.log_error(error, level);
    }
}

/// Standard logging implementation for common logging crates
#[cfg(feature = "log")]
pub mod log_impl {
    use super::*;
    use log::{error, warn, info, debug};
    
    /// A logger that uses the `log` crate
    pub struct LogAdapter;
    
    impl ErrorLogger for LogAdapter {
        fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel) {
            let kind = error.kind();
            let message = error.dev_message();
            match level {
                ErrorLevel::Critical => error!(target: "error-forge", "[CRITICAL] [{kind}] {message}"),
                ErrorLevel::Error => error!(target: "error-forge", "[ERROR] [{kind}] {message}"),
                ErrorLevel::Warning => warn!(target: "error-forge", "[WARNING] [{kind}] {message}"),
                ErrorLevel::Info => info!(target: "error-forge", "[INFO] [{kind}] {message}"),
                ErrorLevel::Debug => debug!(target: "error-forge", "[DEBUG] [{kind}] {message}"),
            }
        }
        
        fn log_message(&self, message: &str, level: ErrorLevel) {
            match level {
                ErrorLevel::Critical | ErrorLevel::Error => error!(target: "error-forge", "{message}"),
                ErrorLevel::Warning => warn!(target: "error-forge", "{message}"),
                ErrorLevel::Info => info!(target: "error-forge", "{message}"),
                ErrorLevel::Debug => debug!(target: "error-forge", "{message}"),
            }
        }
        
        fn log_panic(&self, info: &std::panic::PanicHookInfo) {
            error!(target: "error-forge", "PANIC: {info}");
        }
    }
    
    /// Initialize logging with the log crate adapter
    pub fn init() -> Result<(), &'static str> {
        register_logger(LogAdapter)
    }
}

#[cfg(feature = "tracing")]
pub mod tracing_impl {
    use super::*;
    use tracing::{error, warn, info, debug};
    
    /// A logger that uses the `tracing` crate
    pub struct TracingAdapter;
    
    impl ErrorLogger for TracingAdapter {
        fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel) {
            match level {
                ErrorLevel::Critical => error!(target: "error-forge", kind = %error.kind(), message = %error.dev_message(), "Critical error"),
                ErrorLevel::Error => error!(target: "error-forge", kind = %error.kind(), message = %error.dev_message(), "Error"),
                ErrorLevel::Warning => warn!(target: "error-forge", kind = %error.kind(), message = %error.dev_message(), "Warning"),
                ErrorLevel::Info => info!(target: "error-forge", kind = %error.kind(), message = %error.dev_message(), "Info"),
                ErrorLevel::Debug => debug!(target: "error-forge", kind = %error.kind(), message = %error.dev_message(), "Debug"),
            }
        }
        
        fn log_message(&self, message: &str, level: ErrorLevel) {
            match level {
                ErrorLevel::Critical | ErrorLevel::Error => error!(target: "error-forge", "{message}"),
                ErrorLevel::Warning => warn!(target: "error-forge", "{message}"),
                ErrorLevel::Info => info!(target: "error-forge", "{message}"),
                ErrorLevel::Debug => debug!(target: "error-forge", "{message}"),
            }
        }
        
        fn log_panic(&self, info: &std::panic::PanicHookInfo) {
            error!(target: "error-forge", panic = %info, "Panic occurred");
        }
    }
    
    /// Initialize logging with the tracing adapter
    pub fn init() -> Result<(), &'static str> {
        register_logger(TracingAdapter)
    }
}

/// Build your own error logger - example implementation
pub mod custom {
    use super::*;
    
    // Type aliases for complex types
    /// Function type for error logging
    type ErrorFn = Box<dyn Fn(&dyn ForgeError, ErrorLevel) + Send + Sync + 'static>;
    /// Function type for message logging
    type MessageFn = Box<dyn Fn(&str, ErrorLevel) + Send + Sync + 'static>;
    /// Function type for panic logging
    type PanicFn = Box<dyn Fn(&std::panic::PanicHookInfo) + Send + Sync + 'static>;
    
    /// Builder for creating a custom error logger
    #[derive(Default)]
    pub struct ErrorLoggerBuilder {
        error_fn: Option<ErrorFn>,
        message_fn: Option<MessageFn>,
        panic_fn: Option<PanicFn>,
    }

    impl ErrorLoggerBuilder {
        /// Create a new error logger builder
        pub fn new() -> Self {
            Self::default()
        }
        
        /// Set the function to use for logging errors
        pub fn with_error_fn<F>(mut self, f: F) -> Self
        where
            F: Fn(&dyn ForgeError, ErrorLevel) + Send + Sync + 'static,
        {
            self.error_fn = Some(Box::new(f));
            self
        }
        
        /// Set the function to use for logging messages
        pub fn with_message_fn<F>(mut self, f: F) -> Self
        where
            F: Fn(&str, ErrorLevel) + Send + Sync + 'static,
        {
            self.message_fn = Some(Box::new(f));
            self
        }
        
        /// Set the function to use for logging panics
        pub fn with_panic_fn<F>(mut self, f: F) -> Self
        where
            F: Fn(&std::panic::PanicHookInfo) + Send + Sync + 'static,
        {
            self.panic_fn = Some(Box::new(f));
            self
        }
        
        /// Build the error logger
        pub fn build(self) -> CustomErrorLogger {
            CustomErrorLogger {
                error_fn: self.error_fn,
                message_fn: self.message_fn,
                panic_fn: self.panic_fn,
            }
        }
    }
    
    /// A custom error logger that uses user-provided functions
    pub struct CustomErrorLogger {
        error_fn: Option<ErrorFn>,
        message_fn: Option<MessageFn>,
        panic_fn: Option<PanicFn>,
    }
    
    impl ErrorLogger for CustomErrorLogger {
        fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel) {
            if let Some(error_fn) = &self.error_fn {
                error_fn(error, level);
            }
        }
        
        fn log_message(&self, message: &str, level: ErrorLevel) {
            if let Some(message_fn) = &self.message_fn {
                message_fn(message, level);
            }
        }
        
        fn log_panic(&self, info: &std::panic::PanicHookInfo) {
            if let Some(panic_fn) = &self.panic_fn {
                panic_fn(info);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;
    use std::sync::Mutex;
    use std::sync::Arc;
    
    #[test]
    fn test_custom_logger() {
        // A simple test logger that captures logs in a Vec
        struct TestLogger {
            logs: Arc<Mutex<Vec<String>>>,
        }
        
        impl ErrorLogger for TestLogger {
            fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel) {
                let kind = error.kind();
                let message = error.dev_message();
                let log = format!("{level:?}: [{kind}] {message}");
                self.logs.lock().unwrap().push(log);
            }
            
            fn log_message(&self, message: &str, level: ErrorLevel) {
                let log = format!("{level:?}: {message}");
                self.logs.lock().unwrap().push(log);
            }
            
            fn log_panic(&self, info: &std::panic::PanicHookInfo) {
                let log = format!("PANIC: {info}");
                self.logs.lock().unwrap().push(log);
            }
        }
        
        // Create and register the logger
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logger = TestLogger { logs: Arc::clone(&logs) };
        
        // We need to make sure we have a fresh state for this test
        // In a real app, you'd only register once at startup
        let _ = register_logger(logger);
        
        // Log an error
        let error = AppError::config("Test error");
        log_error(&error);
        
        // Check that the log was captured
        let captured_logs = logs.lock().unwrap();
        assert!(!captured_logs.is_empty());
        assert!(captured_logs[0].contains("[Config]"));
        assert!(captured_logs[0].contains("Test error"));
    }
}
