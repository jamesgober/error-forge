use std::fmt;
use std::error::Error;
use crate::error::ForgeError;

/// A collection of errors that can be accumulated and returned as a single result
#[derive(Debug, Default)]
pub struct ErrorCollector<E> {
    /// The collected errors
    errors: Vec<E>,
}

impl<E> ErrorCollector<E> {
    /// Create a new empty error collector
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }
    
    /// Add an error to the collection
    pub fn push(&mut self, error: E) {
        self.errors.push(error);
    }
    
    /// Add an error to the collection and return self for chaining
    pub fn with(mut self, error: E) -> Self {
        self.push(error);
        self
    }
    
    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
    
    /// Get the number of collected errors
    pub fn len(&self) -> usize {
        self.errors.len()
    }
    
    /// Return a result that is Ok if there are no errors, or Err with the collector otherwise
    pub fn into_result<T>(self, ok_value: T) -> Result<T, Self> {
        if self.is_empty() {
            Ok(ok_value)
        } else {
            Err(self)
        }
    }
    
    /// Return a result that is Ok if there are no errors, or Err with the collector otherwise
    pub fn result<T>(&self, ok_value: T) -> Result<T, &Self> {
        if self.is_empty() {
            Ok(ok_value)
        } else {
            Err(self)
        }
    }
    
    /// Consume the collector and return the vector of errors
    pub fn into_errors(self) -> Vec<E> {
        self.errors
    }
    
    /// Get a reference to the vector of errors
    pub fn errors(&self) -> &Vec<E> {
        &self.errors
    }
    
    /// Get a mutable reference to the vector of errors
    pub fn errors_mut(&mut self) -> &mut Vec<E> {
        &mut self.errors
    }
    
    /// Add all errors from another collector
    pub fn extend(&mut self, other: ErrorCollector<E>) {
        self.errors.extend(other.errors);
    }
    
    /// Try an operation that may return an error, collecting the error if it occurs
    pub fn try_collect<F, T>(&mut self, op: F) -> Option<T>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match op() {
            Ok(val) => Some(val),
            Err(e) => {
                self.push(e);
                None
            }
        }
    }
}

impl<E: fmt::Display> fmt::Display for ErrorCollector<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            write!(f, "No errors")
        } else if self.errors.len() == 1 {
            write!(f, "1 error: {}", self.errors[0])
        } else {
            writeln!(f, "{} errors:", self.errors.len())?;
            for (i, err) in self.errors.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, err)?;
            }
            Ok(())
        }
    }
}

impl<E: Error> Error for ErrorCollector<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.errors.first().and_then(|e| e.source())
    }
}

/// Extension trait for Result types to collect errors
pub trait CollectError<T, E> {
    /// Collect an error into an ErrorCollector if the result is an error
    fn collect_err(self, collector: &mut ErrorCollector<E>) -> Option<T>;
}

impl<T, E> CollectError<T, E> for Result<T, E> {
    fn collect_err(self, collector: &mut ErrorCollector<E>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(e) => {
                collector.push(e);
                None
            }
        }
    }
}

// Special implementation for ForgeError types to provide rich error collection
impl<E: ForgeError> ErrorCollector<E> {
    /// Return a summary of the collected errors using ForgeError traits
    pub fn summary(&self) -> String {
        if self.errors.is_empty() {
            return "No errors".to_string();
        }
        
        let mut result = String::new();
        let fatal_count = self.errors.iter().filter(|e| e.is_fatal()).count();
        let retryable_count = self.errors.iter().filter(|e| e.is_retryable()).count();
        
        result.push_str(&format!("{} errors collected ({} fatal, {} retryable):\n", 
            self.errors.len(), fatal_count, retryable_count));
        
        for (i, err) in self.errors.iter().enumerate() {
            result.push_str(&format!("  {}. [{}] {}\n", 
                i + 1, 
                err.kind(), 
                err.dev_message()));
        }
        
        result
    }
    
    /// Check if any of the collected errors is marked as fatal
    pub fn has_fatal(&self) -> bool {
        self.errors.iter().any(|e| e.is_fatal())
    }
    
    /// Check if all collected errors are retryable
    pub fn all_retryable(&self) -> bool {
        !self.errors.is_empty() && self.errors.iter().all(|e| e.is_retryable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;
    
    #[test]
    fn test_error_collector() {
        let mut collector = ErrorCollector::new();
        
        assert!(collector.is_empty());
        
        collector.push(AppError::config("Config error"));
        collector.push(AppError::filesystem("File not found", None));
        
        assert_eq!(collector.len(), 2);
        
        let result: Result<(), _> = collector.into_result(());
        assert!(result.is_err());
    }
    
    #[test]
    fn test_collect_error() {
        let mut collector = ErrorCollector::new();
        
        let result1: Result<i32, AppError> = Ok(42);
        let result2: Result<i32, AppError> = Err(AppError::network("Connection failed", None));
        
        let val1 = result1.collect_err(&mut collector);
        let val2 = result2.collect_err(&mut collector);
        
        assert_eq!(val1, Some(42));
        assert_eq!(val2, None);
        assert_eq!(collector.len(), 1);
    }
    
    #[test]
    fn test_forge_error_collector() {
        let mut collector = ErrorCollector::new();
        
        collector.push(AppError::config("Config error").with_fatal(true));
        collector.push(AppError::network("Connection failed", None).with_retryable(true));
        
        assert!(collector.has_fatal());
        assert!(!collector.all_retryable());
        
        let summary = collector.summary();
        assert!(summary.contains("2 errors collected (1 fatal, 1 retryable)"));
        assert!(summary.contains("[Config]"));
        assert!(summary.contains("[Network]"));
    }
}
