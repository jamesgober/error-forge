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

const SERVICE: &str = "billing";

// Every variant here used to fail to compile: `format!` was handed all
// fields as arguments, so a display string (or the default display,
// the bare variant name) that skipped a field hit "argument never used"
// or "named argument never used".
#[derive(Debug, ModError)]
#[error_prefix("Partial")]
pub enum PartialDisplayError {
    NoDisplayTuple(String, u16),

    NoDisplayNamed {
        host: String,
        port: u16,
    },

    #[error_display("Transaction error")]
    SkipsAllTuple(u32),

    #[error_display("Port {port} refused")]
    SkipsSomeNamed {
        host: String,
        port: u16,
    },

    #[error_display("Second field only: {1}")]
    SkipsFirstTuple(String, u16),

    #[error_display("{SERVICE} unavailable ({0:?}), {{retry later}}")]
    CapturesConst(u8, String),
}

#[test]
fn test_derive_macro_display_may_skip_fields() {
    assert_eq!(
        PartialDisplayError::NoDisplayTuple("x".into(), 1).to_string(),
        "NoDisplayTuple"
    );
    assert_eq!(
        PartialDisplayError::NoDisplayNamed {
            host: "db".into(),
            port: 5432,
        }
        .to_string(),
        "NoDisplayNamed"
    );
    assert_eq!(
        PartialDisplayError::SkipsAllTuple(9).to_string(),
        "Transaction error"
    );
    assert_eq!(
        PartialDisplayError::SkipsSomeNamed {
            host: "db".into(),
            port: 5432,
        }
        .to_string(),
        "Port 5432 refused"
    );
    assert_eq!(
        PartialDisplayError::SkipsFirstTuple("ignored".into(), 7).to_string(),
        "Second field only: 7"
    );
    assert_eq!(
        PartialDisplayError::CapturesConst(3, "unused".into()).to_string(),
        "billing unavailable (3), {retry later}"
    );
}

#[derive(Debug, ModError)]
pub enum PositionalDisplayError {
    // Positional placeholders in a struct-like variant refer to the
    // fields in declaration order, as before.
    #[error_display("{}:{}")]
    Endpoint { host: String, port: u16 },

    #[error_display("{1} after {0} and {0:?}")]
    Reordered(u8, &'static str),
}

#[test]
fn test_derive_macro_positional_display_unchanged() {
    assert_eq!(
        PositionalDisplayError::Endpoint {
            host: "localhost".into(),
            port: 8080,
        }
        .to_string(),
        "localhost:8080"
    );
    assert_eq!(
        PositionalDisplayError::Reordered(1, "b").to_string(),
        "b after 1 and 1"
    );
}
