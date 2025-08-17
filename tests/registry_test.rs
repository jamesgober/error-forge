use error_forge::{AppError, registry::{register_error_code}, ForgeError};

#[test]
fn test_error_with_code() {
    let error = AppError::config("Invalid configuration")
        .with_code("CONFIG-001");
    
    assert_eq!(error.to_string(), "[CONFIG-001] ⚙️ Configuration Error: Invalid configuration");
    assert_eq!(error.kind(), "Config"); // Original error properties preserved
}

#[test]
fn test_register_and_retrieve_code() {
    // Register a new error code with metadata
    let _ = register_error_code(
        "TEST-001", 
        "Test error code", 
        Some("https://docs.example.com/errors/TEST-001"),
        true
    );
    
    // Use the registered error code
    let error = AppError::network("Connection timeout", None)
        .with_code("TEST-001");
    
    // Verify the code is correctly associated
    assert!(error.to_string().starts_with("[TEST-001]"));
    
    // The registry information should be used for certain properties
    assert!(error.is_retryable()); // Specified in the registry
}

#[test]
fn test_error_code_chaining() {
    // We can chain error codes with other error enhancements
    let error = AppError::filesystem("Permission denied", None)
        .with_status(403)
        .with_code("PERM-001")
        .with_fatal(true);
    
    assert!(error.to_string().contains("[PERM-001]"));
    assert!(error.is_fatal());
    assert_eq!(error.status_code(), 403);
}

#[test]
fn test_duplicate_registration() {
    // First registration should succeed
    let result1 = register_error_code(
        "UNIQUE-001", 
        "First registration", 
        None::<String>,
        false
    );
    
    // Second registration with same code should fail
    let result2 = register_error_code(
        "UNIQUE-001", 
        "Second registration", 
        None::<String>,
        true
    );
    
    assert!(result1.is_ok());
    assert!(result2.is_err());
    assert!(result2.unwrap_err().contains("already registered"));
}

#[test]
fn test_error_code_in_dev_message() {
    // Register with documentation URL
    let _ = register_error_code(
        "DOC-001", 
        "Error with documentation", 
        Some("https://example.com/docs/errors/DOC-001"),
        false
    );
    
    let error = AppError::config("Missing required field")
        .with_code("DOC-001");
    
    // The dev message should include the documentation URL
    let dev_message = error.dev_message();
    assert!(dev_message.contains("[DOC-001]"));
    assert!(dev_message.contains("https://example.com/docs/errors/DOC-001"));
}
