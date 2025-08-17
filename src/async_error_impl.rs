#[cfg(feature = "async")]
use async_trait::async_trait;
use std::error::Error as StdError;

use crate::async_error::AsyncForgeError;
use crate::error::AppError;

/// Provides async implementations for the AppError type.
#[cfg(feature = "async")]
#[async_trait]
impl AsyncForgeError for AppError {
    fn kind(&self) -> &'static str {
        <Self as crate::error::ForgeError>::kind(self)
    }
    
    fn caption(&self) -> &'static str {
        <Self as crate::error::ForgeError>::caption(self)
    }
    
    fn is_retryable(&self) -> bool {
        <Self as crate::error::ForgeError>::is_retryable(self)
    }
    
    fn is_fatal(&self) -> bool {
        <Self as crate::error::ForgeError>::is_fatal(self)
    }
    
    fn status_code(&self) -> u16 {
        <Self as crate::error::ForgeError>::status_code(self)
    }
    
    fn exit_code(&self) -> i32 {
        <Self as crate::error::ForgeError>::exit_code(self)
    }
    
    fn user_message(&self) -> String {
        <Self as crate::error::ForgeError>::user_message(self)
    }
    
    fn dev_message(&self) -> String {
        <Self as crate::error::ForgeError>::dev_message(self)
    }
    
    async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        // Default async handling for AppError
        match self {
            AppError::Network { .. } => {
                // In a real implementation, this might attempt reconnection or other async recovery
                Ok(())
            },
            _ => {
                // For other error types, default to regular error handling
                Ok(())
            }
        }
    }
}

#[cfg(feature = "async")]
impl AppError {
    /// Create a new error from an async operation result
    pub async fn from_async_result<T, E>(result: Result<T, E>) -> Result<T, Self> 
    where
        E: StdError + Send + Sync + 'static
    {
        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(Self::other(e.to_string())
                .with_fatal(false)
                .with_retryable(true))
        }
    }
    
    /// Handle this error asynchronously using the AsyncForgeError trait
    pub async fn handle_async(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        <Self as AsyncForgeError>::async_handle(self).await
    }
    
    /// Wrap this error with async context
    pub fn with_async_context<C: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static>(
        self,
        context_generator: impl FnOnce() -> C + Send + 'static
    ) -> crate::context::ContextError<Self, C> {
        crate::context::ContextError::new(self, context_generator())
    }
}
