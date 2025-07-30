// Example of using the derive(ModError) proc-macro for simple error type creation
// Run this example with: cargo run --example derive_example --features derive
#[allow(unused_imports)]
use error_forge::ForgeError;

// Only available when the "derive" feature is enabled
#[cfg(feature = "derive")]
use error_forge::ModError;

// Define a basic enum error type using derive(ModError)
#[cfg(feature = "derive")]
#[derive(Debug, ModError)]
#[error_prefix = "DATABASE"]
pub enum SimpleDbError {
    // Simple unit variant
    #[error_display("Connection failed")]
    ConnectionFailed,
    
    // Simple tuple variant with one field
    #[error_display("Query failed: {0}")]
    QueryFailed(String),
    
    // Unit variant with additional metadata attributes
    #[error_display("Transaction failed")]
    #[error_retryable]
    #[error_http_status(400)]
    TransactionFailed,
}

// A simple struct error type
#[cfg(feature = "derive")]
#[derive(Debug, ModError)]
#[error_prefix("Config")]
pub struct SimpleConfigError;


fn main() {
    // Only compile this section when the "derive" feature is enabled
    #[cfg(feature = "derive")]
    {
        // Create some example errors
        let conn_err = SimpleDbError::ConnectionFailed;
        let query_err = SimpleDbError::QueryFailed("Syntax error in SQL".to_string());
        let tx_err = SimpleDbError::TransactionFailed;
        let config_err = SimpleConfigError;

        // Demonstrate basic ForgeError trait functionality
        println!("=== Derived Error Examples ===");
        
        println!("\n--- SimpleDbError::ConnectionFailed ---");
        println!("Display: {}", conn_err);
        println!("Raw display format: '{:?}'", format!("{}", conn_err));
        println!("Kind: {}", conn_err.kind());
        println!("Caption: {}", conn_err.caption());
        println!("Raw caption: '{:?}'", conn_err.caption());
        println!("Is retryable: {}", conn_err.is_retryable());
        println!("Status code: {}", conn_err.status_code());
        
        println!("\n--- SimpleDbError::QueryFailed ---");
        println!("Display: {}", query_err);
        println!("Kind: {}", query_err.kind());
        println!("Caption: {}", query_err.caption());
        println!("Is retryable: {}", query_err.is_retryable());
        println!("Status code: {}", query_err.status_code());
        
        println!("\n--- SimpleDbError::TransactionFailed ---");
        println!("Display: {}", tx_err);
        println!("Kind: {}", tx_err.kind());
        println!("Caption: {}", tx_err.caption());
        println!("Is retryable: {}", tx_err.is_retryable());
        println!("Status code: {}", tx_err.status_code());
        
        println!("\n--- SimpleConfigError ---");
        println!("Display: {}", config_err);
        println!("Kind: {}", config_err.kind());
        println!("Caption: {}", config_err.caption());
        println!("Is retryable: {}", config_err.is_retryable());
        println!("Status code: {}", config_err.status_code());
    }
    
    // When the "derive" feature is not enabled, show this message instead
    #[cfg(not(feature = "derive"))]
    {
        println!("This example requires the 'derive' feature to be enabled.");
        println!("Run it with: cargo run --example derive_example --features derive");
    }
}
