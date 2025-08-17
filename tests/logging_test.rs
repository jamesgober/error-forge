use error_forge::{
    AppError, ForgeError, ErrorLogger,
    macros::ErrorLevel
};
use std::sync::{Arc, Mutex};
use std::panic::PanicHookInfo;

// Custom test logger for testing purposes
struct TestLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl ErrorLogger for TestLogger {
    fn log_error(&self, error: &dyn ForgeError, level: ErrorLevel) {
        let log = format!("{:?}: [{}] {}", level, error.kind(), error.dev_message());
        self.logs.lock().unwrap().push(log);
    }
    
    fn log_message(&self, message: &str, level: ErrorLevel) {
        let log = format!("{:?}: {}", level, message);
        self.logs.lock().unwrap().push(log);
    }
    
    fn log_panic(&self, info: &PanicHookInfo) {
        let log = format!("PANIC: {}", info);
        self.logs.lock().unwrap().push(log);
    }
}

// Direct testing of the logger without relying on the global registration
#[test]
fn test_custom_logger() {
    // Create shared storage for captured logs
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logger = TestLogger { logs: Arc::clone(&logs) };
    
    // Create an error
    let error = AppError::config("Missing configuration file")
        .with_fatal(true);
    
    // Directly use the logger without global registration
    logger.log_error(&error, ErrorLevel::Critical);
    
    // Check if the log was captured
    let captured = logs.lock().unwrap();
    assert!(!captured.is_empty(), "Log should have been captured");
    println!("Captured log: {}", captured[0]);
    assert!(captured[0].contains("[Config]"), "Log should contain 'Config' error kind");
    assert!(captured[0].contains("Missing configuration"), "Log should contain error message");
    assert!(captured[0].contains("Critical"), "Log should have Critical level");
}

#[test]
fn test_logger_builder() {
    use error_forge::logging::custom::ErrorLoggerBuilder;
    
    // Use the builder to create a custom logger
    let captured_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_clone = Arc::clone(&captured_errors);
    
    let logger = ErrorLoggerBuilder::new()
        .with_error_fn(move |err, level| {
            let log = format!("ERROR[{:?}]: {}", level, err.dev_message());
            captured_clone.lock().unwrap().push(log);
        })
        .with_message_fn(|msg, level| {
            println!("MESSAGE[{:?}]: {}", level, msg);
        })
        .build();
    
    // Create the error
    let error = AppError::network("Connection failed", None);
    
    // Directly use the logger - no global registration needed
    logger.log_error(&error, ErrorLevel::Warning);
    
    // Check if the log was captured by our custom logger
    let captured = captured_errors.lock().unwrap();
    assert!(!captured.is_empty(), "Builder log should have been captured");
    println!("Builder test log: {}", captured[0]);
    assert!(captured[0].contains("Connection failed"), "Log should contain error message");
}
