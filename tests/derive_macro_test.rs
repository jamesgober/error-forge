#![cfg(feature = "derive")]

use error_forge::{ForgeError, ModError};

#[derive(Debug, ModError)]
#[error_prefix("Database")]
pub enum DerivedDbError {
    #[error_display("Connection failed: {0}")]
    #[error_kind("DbConnection")]
    #[error_caption("Database Connection Error")]
    #[error_retryable]
    #[error_http_status(503)]
    #[error_exit_code(12)]
    ConnectionFailed(String),

    #[error_display("Query failed for {query}")]
    QueryFailed { query: String },

    #[error_display("Permission denied")]
    #[error_fatal]
    PermissionDenied,
}

#[derive(Debug, ModError)]
#[error_prefix = "Config"]
pub struct DerivedConfigError;

#[test]
fn test_derive_macro_tuple_variant_metadata() {
    let error = DerivedDbError::ConnectionFailed("primary-db".to_string());

    assert_eq!(error.to_string(), "Connection failed: primary-db");
    assert_eq!(error.kind(), "DbConnection");
    assert_eq!(error.caption(), "Database Connection Error");
    assert!(error.is_retryable());
    assert_eq!(error.status_code(), 503);
    assert_eq!(error.exit_code(), 12);
    assert!(!error.is_fatal());
}

#[test]
fn test_derive_macro_named_variant_formatting() {
    let error = DerivedDbError::QueryFailed {
        query: "SELECT 1".to_string(),
    };

    assert_eq!(error.to_string(), "Query failed for SELECT 1");
    assert_eq!(error.kind(), "QueryFailed");
    assert_eq!(error.caption(), "Database: Error");
    assert_eq!(error.status_code(), 500);
}

#[test]
fn test_derive_macro_fatal_flag_and_struct_prefix() {
    let fatal_error = DerivedDbError::PermissionDenied;
    let config_error = DerivedConfigError;

    assert!(fatal_error.is_fatal());
    assert_eq!(fatal_error.to_string(), "Permission denied");
    assert_eq!(config_error.to_string(), "Config: Error");
    assert_eq!(config_error.caption(), "Config: Error");
}
