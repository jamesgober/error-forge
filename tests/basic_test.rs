use error_forge::ForgeError;

// Define a custom database error
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

// Test basic error definition
#[test]
fn test_app_error() {
    // Use the built-in AppError
    use error_forge::AppError;
    
    let error = AppError::config("Missing configuration");
    assert_eq!(error.kind(), "Config");
    assert_eq!(error.status_code(), 500);
    assert!(error.to_string().contains("Missing configuration"));
}

// Test console theme
#[test]
fn test_console_theme() {
    use error_forge::AppError;
    use error_forge::console_theme::ConsoleTheme;
    
    let error = AppError::other("Test error");
    let theme = ConsoleTheme::default();
    let formatted = theme.format_error(&error);
    
    // Verify the error message is in the output
    assert!(formatted.contains("Test error"));
    
    // Verify retryable status is displayed
    assert!(formatted.contains("Retryable:"));
    
    // Since AppError::other doesn't have a source, there should be no "Caused by" line
    assert!(!formatted.contains("Caused by:"));
}
