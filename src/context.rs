use std::fmt;
use crate::error::ForgeError;

/// A wrapper error type that attaches contextual information to an error
#[derive(Debug)]
pub struct ContextError<E, C> {
    /// The original error
    pub error: E,
    /// The context attached to the error
    pub context: C,
}

impl<E, C> ContextError<E, C> {
    /// Create a new context error wrapping the original error
    pub fn new(error: E, context: C) -> Self {
        Self { error, context }
    }
    
    /// Extract the original error, discarding the context
    pub fn into_error(self) -> E {
        self.error
    }
    
    /// Map the context to a new type using the provided function
    pub fn map_context<D, F>(self, f: F) -> ContextError<E, D>
    where
        F: FnOnce(C) -> D,
    {
        ContextError {
            error: self.error,
            context: f(self.context),
        }
    }
    
    /// Add another layer of context to this error
    pub fn context<D>(self, context: D) -> ContextError<Self, D>
    where
        D: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        ContextError::new(self, context)
    }
}

impl<E: fmt::Display, C: fmt::Display> fmt::Display for ContextError<E, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.error)
    }
}

impl<E: std::error::Error + 'static, C: fmt::Display + fmt::Debug + Send + Sync + 'static> std::error::Error for ContextError<E, C> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Extension trait for Result types to add context to errors
pub trait ResultExt<T, E> {
    /// Adds context to the error variant of the Result
    fn context<C>(self, context: C) -> Result<T, ContextError<E, C>>;
    
    /// Adds context to the error variant using a closure that is only called on error
    fn with_context<C, F>(self, f: F) -> Result<T, ContextError<E, C>>
    where
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn context<C>(self, context: C) -> Result<T, ContextError<E, C>> {
        self.map_err(|error| ContextError::new(error, context))
    }
    
    fn with_context<C, F>(self, f: F) -> Result<T, ContextError<E, C>>
    where
        F: FnOnce() -> C,
    {
        self.map_err(|error| ContextError::new(error, f()))
    }
}

// Implement ForgeError for ContextError when the inner error implements ForgeError
impl<E: ForgeError, C: fmt::Display + fmt::Debug + Send + Sync + 'static> ForgeError for ContextError<E, C> {
    fn kind(&self) -> &'static str {
        self.error.kind()
    }
    
    fn caption(&self) -> &'static str {
        self.error.caption()
    }
    
    fn is_retryable(&self) -> bool {
        self.error.is_retryable()
    }
    
    fn is_fatal(&self) -> bool {
        self.error.is_fatal()
    }
    
    fn status_code(&self) -> u16 {
        self.error.status_code()
    }
    
    fn exit_code(&self) -> i32 {
        self.error.exit_code()
    }
    
    fn user_message(&self) -> String {
        format!("{}: {}", self.context, self.error.user_message())
    }
    
    fn dev_message(&self) -> String {
        format!("{}: {}", self.context, self.error.dev_message())
    }
    
    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        self.error.backtrace()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;
    
    #[test]
    fn test_context_error() {
        let error = AppError::config("Invalid config");
        let ctx_error = error.context("Failed to load settings");
        
        assert_eq!(ctx_error.to_string(), "Failed to load settings: ⚙️ Configuration Error: Invalid config");
        assert_eq!(ctx_error.kind(), "Config");
        assert_eq!(ctx_error.caption(), "⚙️ Configuration");
    }
    
    #[test]
    fn test_result_context() {
        let result: Result<(), AppError> = Err(AppError::config("Invalid config"));
        let ctx_result = result.context("Failed to load settings");
        
        assert!(ctx_result.is_err());
        let err = ctx_result.unwrap_err();
        assert_eq!(err.to_string(), "Failed to load settings: ⚙️ Configuration Error: Invalid config");
    }
}
