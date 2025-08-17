use error_forge::{AppError, collector::{ErrorCollector, CollectError}};

#[test]
fn test_empty_collector() {
    let collector: ErrorCollector<AppError> = ErrorCollector::new();
    
    assert!(collector.is_empty());
    assert_eq!(collector.len(), 0);
    
    // An empty collector should return Ok for into_result
    let result: Result<(), _> = collector.into_result(());
    assert!(result.is_ok());
}

#[test]
fn test_collector_with_errors() {
    let mut collector = ErrorCollector::new();
    
    collector.push(AppError::config("Missing config file"));
    collector.push(AppError::network("Connection timeout", None));
    
    assert!(!collector.is_empty());
    assert_eq!(collector.len(), 2);
    
    // A collector with errors should return Err for into_result
    let result: Result<i32, _> = collector.into_result(42);
    assert!(result.is_err());
}

#[test]
fn test_collector_summary() {
    let mut collector = ErrorCollector::new();
    
    collector.push(AppError::config("Invalid setting").with_fatal(true));
    collector.push(AppError::network("Connection timeout", None).with_retryable(true));
    
    let summary = collector.summary();
    
    // Summary should contain error counts and details
    assert!(summary.contains("2 errors collected"));
    assert!(summary.contains("1 fatal"));
    assert!(summary.contains("1 retryable"));
    assert!(summary.contains("[Config]"));
    assert!(summary.contains("[Network]"));
}

#[test]
fn test_collector_display() {
    let mut collector = ErrorCollector::new();
    
    // Test with one error
    collector.push(AppError::config("Missing config"));
    assert!(collector.to_string().contains("1 error:"));
    assert!(collector.to_string().contains("Missing config"));
    
    // Test with multiple errors
    collector.push(AppError::filesystem("File not found", None));
    let display = collector.to_string();
    assert!(display.contains("2 errors:"));
    assert!(display.contains("1. "));
    assert!(display.contains("2. "));
}

#[test]
fn test_collect_error_extension() {
    let mut collector = ErrorCollector::new();
    
    // Test successful result
    let ok_result: Result<i32, AppError> = Ok(42);
    let value = ok_result.collect_err(&mut collector);
    assert_eq!(value, Some(42));
    assert!(collector.is_empty());
    
    // Test error result
    let err_result: Result<i32, AppError> = Err(AppError::config("Failed"));
    let value = err_result.collect_err(&mut collector);
    assert_eq!(value, None);
    assert_eq!(collector.len(), 1);
}

#[test]
fn test_try_collect_method() {
    let mut collector = ErrorCollector::new();
    
    // Test successful operation
    let success = collector.try_collect(|| Ok::<_, AppError>(42));
    assert_eq!(success, Some(42));
    
    // Test failed operation
    let failure = collector.try_collect(|| Err::<i32, _>(AppError::config("Failed")));
    assert_eq!(failure, None);
    
    assert_eq!(collector.len(), 1);
}

#[test]
fn test_fatal_and_retryable_detection() {
    let mut collector = ErrorCollector::new();
    
    // Add non-fatal, retryable error
    collector.push(AppError::network("Timeout", None).with_retryable(true));
    assert!(!collector.has_fatal());
    assert!(collector.all_retryable());
    
    // Add fatal error
    collector.push(AppError::config("Critical error").with_fatal(true));
    assert!(collector.has_fatal());
    assert!(!collector.all_retryable());
}
