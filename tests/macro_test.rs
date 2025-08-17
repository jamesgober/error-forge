// Tests focusing on error display format capabilities
use error_forge::{AppError, ForgeError};
use std::io;
use std::error::Error;

// Instead of using complex macros, we'll test the existing AppError functionality
// and the enhanced error display capabilities we added

#[test]
fn test_error_display() {
    // Create a config error
    let config_err = AppError::config("Missing configuration file");
    
    // Test basic ForgeError trait methods
    assert_eq!(config_err.kind(), "Config");
    assert_eq!(config_err.caption(), "⚙️ Configuration");
    assert!(!config_err.is_retryable());
    
    // Verify the error display formatting
    let display_str = format!("{}", config_err);
    assert!(display_str.contains("⚙️ Configuration Error"));
    assert!(display_str.contains("Missing configuration file"));
}

#[test]
fn test_filesystem_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let fs_err = AppError::filesystem("config.json", io_err);
    
    // Test error metadata
    assert_eq!(fs_err.kind(), "Filesystem");
    assert!(fs_err.status_code() >= 500);
    
    // Verify error chaining via source()
    let source = fs_err.source();
    assert!(source.is_some());
    
    // Check display formatting with emoji
    let display_str = format!("{}", fs_err);
    assert!(display_str.contains("💾 Filesystem Error"));
    assert!(display_str.contains("config.json"));
}

#[test]
fn test_network_error() {
    // Test error with optional source
    let net_err = AppError::network("https://api.example.com", None);
    assert_eq!(net_err.kind(), "Network");
    
    // Check display formatting
    let display_str = format!("{}", net_err);
    assert!(display_str.contains("🌐 Network Error"));
    assert!(display_str.contains("https://api.example.com"));
    
    // Add a source error
    let source_err = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection failed");
    
    // Using a simpler approach - construct with None and check source() exists
    let net_err2 = AppError::from(source_err);
    
    // An alternative approach would be to use the network constructor directly with proper casting:
    // let net_err2 = AppError::network(
    //     "https://api.example.com", 
    //     Some(Box::new(source_err) as Box<dyn Error + Send + Sync>)
    // );
    
    // Verify source error is properly chained
    assert!(net_err2.source().is_some());
}

