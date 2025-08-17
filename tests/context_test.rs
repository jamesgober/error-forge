use error_forge::{AppError, ForgeError, context::{ContextError, ResultExt}};

#[test]
fn test_basic_context() {
    let error = AppError::config("Missing config file");
    let with_context = error.context("Failed to load application settings");
    
    assert_eq!(with_context.to_string(), "Failed to load application settings: ⚙️ Configuration Error: Missing config file");
}

#[test]
fn test_context_preserves_error_properties() {
    let error = AppError::config("Database connection failed")
        .with_status(503)
        .with_retryable(true)
        .with_fatal(false);
    
    let with_context = error.context("Config error");
    
    // Context wrapper should preserve properties of the original error
    assert_eq!(with_context.status_code(), 503);
    assert!(with_context.is_retryable());
    assert!(!with_context.is_fatal());
    assert_eq!(with_context.kind(), "Config");
}

#[test]
fn test_result_context_methods() {
    // Test the context() method on Result
    let result: Result<(), AppError> = Err(AppError::config("Invalid config"));
    let with_context = result.context("Failed to load config");
    
    assert!(with_context.is_err());
    assert_eq!(
        with_context.unwrap_err().to_string(), 
        "Failed to load config: ⚙️ Configuration Error: Invalid config"
    );
    
    // Test the with_context() method on Result
    let result: Result<(), AppError> = Err(AppError::network("Connection failed", None));
    let with_context = result.with_context(|| format!("Failed to connect at {}", "2025-08-17"));
    
    assert!(with_context.is_err());
    let err = with_context.unwrap_err();
    assert!(err.to_string().contains("Failed to connect at 2025-08-17"));
    assert!(err.to_string().contains("Connection failed"));
}

#[test]
fn test_nested_context() {
    // Create error with context - first level
    let error_with_first_context = AppError::config("Missing key")
        .context("Failed to parse config");
        
    // Add another layer of context - direct implementation
    let error = ContextError::new(error_with_first_context, "Failed to initialize application");
    
    // The context messages should be chained in the display representation
    assert_eq!(
        error.to_string(),
        "Failed to initialize application: Failed to parse config: ⚙️ Configuration Error: Missing key"
    );
    
    // But the original error properties should still be accessible
    assert_eq!(error.kind(), "Config");
}

#[test]
fn test_map_context() {
    let error = AppError::filesystem("File not found", None)
        .context("Could not read file");
    
    // We can transform the context to a new type
    let new_context = error.map_context(|ctx| format!("{} (at 2025-08-17)", ctx));
    
    assert_eq!(
        new_context.to_string(),
        "Could not read file (at 2025-08-17): 💾 Filesystem Error at \"File not found\": File operation failed"
    );
}
