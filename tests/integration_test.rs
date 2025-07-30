//! Integration tests for error-forge library
//! This tests the main features of the library including error trait implementations,
//! error handling, and console theming using built-in AppError.

use error_forge::{AppError, ForgeError};
use std::io;
use std::error::Error;

// Define a custom database error for testing external error types
#[derive(Debug)]
pub enum DbError {
    ConnectionFailed(String),
    QueryFailed(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
        }
    }
}

impl std::error::Error for DbError {}

// Test modules
mod tests {
    use super::*;

    // Test basic error functionality
    #[test]
    fn test_error_display() {
        let error = AppError::config("Missing database URL");
        assert_eq!(error.kind(), "Config");
        assert_eq!(error.caption(), "⚙️ Configuration");
        assert!(!error.is_retryable());
        
        // Verify display formatting
        let display = format!("{}", error);
        assert!(display.contains("⚙️ Configuration Error"));
        assert!(display.contains("Missing database URL"));
    }
    
    #[test]
    fn test_source_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = AppError::filesystem("config.json", io_error);
        
        assert_eq!(error.kind(), "Filesystem");
        assert!(error.status_code() >= 500);
        
        // Verify display formatting with emoji
        let display = format!("{}", error);
        assert!(display.contains("💾 Filesystem Error"));
        assert!(display.contains("config.json"));
        
        // Verify source error
        let source = error.source().unwrap();
        assert!(source.to_string().contains("File not found"));
    }
    
    #[test]
    fn test_error_conversion() {
        // Test error with string message
        let db_err = DbError::ConnectionFailed("Failed to connect to database".to_string());
        let app_err = AppError::other(db_err.to_string());
        
        // Check that the error message is properly stored
        assert!(app_err.to_string().contains("Failed to connect to database"));
    }
    
    // Test console theme 
    #[test]
    fn test_themed_errors() {
        let error = AppError::network("https://api.example.com", None);
        
        // Check that themed error display contains expected elements
        let themed_output = format!("{}", error);
        assert!(themed_output.contains("🌐 Network Error"));
        assert!(themed_output.contains("https://api.example.com"));
    }
    
    #[test]
    fn test_error_chain() {
        // Test nested error handling with the source pattern
        // First create an io::Error as the root cause
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
        
        // Wrap it in an AppError::filesystem that stores the io_err as its source
        let fs_err = AppError::filesystem("data/config.json", io_err);
        
        // Check the filesystem error's formatted output
        let fs_display = format!("{}", fs_err);
        assert!(fs_display.contains("Filesystem Error"));
        assert!(fs_display.contains("data/config.json"));
        
        // Verify we can access the source error
        let source = fs_err.source().unwrap();
        assert!(source.to_string().contains("Permission denied"));
        
        // Test another approach to error chaining by combining errors in the message
        let wrapped_err = AppError::other(format!("Wrapped error: {}", fs_err));
        assert!(wrapped_err.to_string().contains("Wrapped error"));
        assert!(wrapped_err.to_string().contains("Filesystem Error"));
    }
}
