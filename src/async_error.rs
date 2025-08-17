use std::error::Error as StdError;
use std::backtrace::Backtrace;

#[cfg(feature = "async")]
use async_trait::async_trait;

/// An async-compatible version of the ForgeError trait.
///
/// This trait extends the standard error capabilities with async support,
/// allowing for async error handling in futures and async functions.
///
/// # Example
///
/// ```rust,ignore
/// use error_forge::async_error::AsyncForgeError;
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct MyAsyncError { message: String }
///
/// impl std::fmt::Display for MyAsyncError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.message)
///     }
/// }
///
/// impl std::error::Error for MyAsyncError {}
///
/// #[async_trait]
/// impl AsyncForgeError for MyAsyncError {
///     fn kind(&self) -> &'static str {
///         "AsyncExample"
///     }
///     
///     fn caption(&self) -> &'static str {
///         "Async Example Error"
///     }
///     
///     async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
///         // Perform async error handling here
///         println!("Handling async error: {}", self);
///         Ok(())
///     }
/// }
/// ```
#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncForgeError: StdError + Send + Sync + 'static {
    /// Returns the kind of error, typically matching the enum variant
    fn kind(&self) -> &'static str;
    
    /// Returns a human-readable caption for the error
    fn caption(&self) -> &'static str;
    
    /// Returns true if the operation can be retried
    fn is_retryable(&self) -> bool {
        false
    }
    
    /// Returns true if the error is fatal and should terminate the program
    fn is_fatal(&self) -> bool {
        false
    }
    
    /// Returns an appropriate HTTP status code for the error
    fn status_code(&self) -> u16 {
        500
    }
    
    /// Returns an appropriate process exit code for the error
    fn exit_code(&self) -> i32 {
        1
    }
    
    /// Returns a user-facing message that can be shown to end users
    fn user_message(&self) -> String {
        self.to_string()
    }
    
    /// Returns a detailed technical message for developers/logs
    fn dev_message(&self) -> String {
        format!("[{}] {}", self.kind(), self)
    }
    
    /// Returns a backtrace if available
    fn backtrace(&self) -> Option<&Backtrace> {
        None
    }
    
    /// Async method to handle the error. This allows implementing custom
    /// async error handling logic.
    async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>>;
    
    /// Registers the error with the central error registry
    fn register(&self) {
        crate::macros::call_error_hook(self.caption(), self.kind(), self.is_fatal(), self.is_retryable());
    }
}

/// Type alias for async error-forge results.
#[cfg(feature = "async")]
pub type AsyncResult<T, E> = std::result::Result<T, E>;
