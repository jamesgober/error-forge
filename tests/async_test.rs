#![cfg(feature = "async")]

use error_forge::{AppError, ForgeError, AsyncForgeError};
use async_trait::async_trait;
use std::error::Error as StdError;

#[tokio::test]
async fn test_async_error_trait() {
    let error = AppError::network("api.example.com", None::<Box<dyn StdError + Send + Sync>>);
    
    // Test that the AsyncForgeError trait implementation works
    assert_eq!(<AppError as AsyncForgeError>::kind(&error), "Network");
    assert_eq!(<AppError as AsyncForgeError>::caption(&error), "🌐 Network");
    assert!(<AppError as AsyncForgeError>::is_retryable(&error));
    assert!(!<AppError as AsyncForgeError>::is_fatal(&error));
    assert_eq!(<AppError as AsyncForgeError>::status_code(&error), 503);
    
    // Test async_handle method
    let result = error.async_handle().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_from_async_result() {
    // Test successful conversion
    let success: Result<i32, std::io::Error> = Ok(42);
    let result = AppError::from_async_result(success).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    
    // Test error conversion
    use std::io::{Error as IoError, ErrorKind};
    let error: Result<i32, std::io::Error> = Err(IoError::new(ErrorKind::NotFound, "Not found"));
    let result = AppError::from_async_result(error).await;
    assert!(result.is_err());
    
    let app_error = result.unwrap_err();
    assert_eq!(<AppError as AsyncForgeError>::kind(&app_error), "Other");
    assert!(<AppError as AsyncForgeError>::is_retryable(&app_error));
    assert!(!<AppError as AsyncForgeError>::is_fatal(&app_error));
}

// Custom error type for testing
#[derive(Debug)]
struct CustomAsyncError {
    message: String,
}

impl std::fmt::Display for CustomAsyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom async error: {}", self.message)
    }
}

impl StdError for CustomAsyncError {}

#[async_trait]
impl AsyncForgeError for CustomAsyncError {
    fn kind(&self) -> &'static str {
        "CustomAsync"
    }
    
    fn caption(&self) -> &'static str {
        "Custom Async Error"
    }
    
    fn is_retryable(&self) -> bool {
        true
    }
    
    async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        // Simulate some async recovery logic
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(())
    }
}

#[tokio::test]
async fn test_custom_async_error_implementation() {
    let custom_error = CustomAsyncError { 
        message: "Test error".to_string() 
    };
    
    assert_eq!(custom_error.kind(), "CustomAsync");
    assert_eq!(custom_error.caption(), "Custom Async Error");
    assert!(custom_error.is_retryable());
    
    // Test async handling
    let handle_result = custom_error.async_handle().await;
    assert!(handle_result.is_ok());
}
