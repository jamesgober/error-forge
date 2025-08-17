use std::marker::PhantomData;
use std::time::Duration;
use std::thread;
use crate::recovery::backoff::{Backoff, ExponentialBackoff, FixedBackoff, LinearBackoff};
use crate::error::ForgeError;

/// Predicate function to determine if an error is retryable
pub type RetryPredicate<E> = Box<dyn Fn(&E) -> bool + Send + Sync + 'static>;

/// Enum to hold different backoff strategy types
pub enum BackoffStrategy {
    Exponential(ExponentialBackoff),
    Linear(LinearBackoff),
    Fixed(FixedBackoff),
}

impl BackoffStrategy {
    fn next_delay(&self, attempt: usize) -> Duration {
        match self {
            BackoffStrategy::Exponential(b) => b.next_delay(attempt),
            BackoffStrategy::Linear(b) => b.next_delay(attempt),
            BackoffStrategy::Fixed(b) => b.next_delay(attempt),
        }
    }
}

/// Executor for retry operations
pub struct RetryExecutor<E> {
    max_retries: usize,
    backoff: BackoffStrategy,
    retry_if: Option<RetryPredicate<E>>,
    _marker: PhantomData<E>,
}

impl<E> RetryExecutor<E> 
where 
    E: std::error::Error + 'static
{
    /// Create a new retry executor with an exponential backoff strategy
    pub fn new_exponential() -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffStrategy::Exponential(ExponentialBackoff::default()),
            retry_if: None,
            _marker: PhantomData,
        }
    }
    
    /// Create a new retry executor with a linear backoff strategy
    pub fn new_linear() -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffStrategy::Linear(LinearBackoff::default()),
            retry_if: None,
            _marker: PhantomData,
        }
    }
    
    /// Create a new retry executor with a fixed backoff strategy
    pub fn new_fixed(delay_ms: u64) -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffStrategy::Fixed(FixedBackoff::new(delay_ms)),
            retry_if: None,
            _marker: PhantomData,
        }
    }
    
    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// Set a predicate to determine if an error should be retried
    pub fn with_retry_if<F>(mut self, predicate: F) -> Self 
    where 
        F: Fn(&E) -> bool + Send + Sync + 'static
    {
        self.retry_if = Some(Box::new(predicate));
        self
    }
    
    /// Execute a fallible operation with retries
    pub fn retry<F, T>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>
    {
        let mut attempt = 0;
        loop {
            match operation() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    // Check if we've reached max retries
                    if attempt >= self.max_retries {
                        return Err(err);
                    }
                    
                    // Check if this error is retryable
                    let should_retry = match &self.retry_if {
                        Some(predicate) => predicate(&err),
                        None => true,
                    };
                    
                    if !should_retry {
                        return Err(err);
                    }
                    
                    // Wait according to backoff strategy
                    let delay = self.backoff.next_delay(attempt);
                    thread::sleep(delay);
                    
                    attempt += 1;
                }
            }
        }
    }
    
    /// Execute a fallible operation with retries using a custom error handler
    pub fn retry_with_handler<F, H, T>(&self, mut operation: F, mut on_error: H) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        H: FnMut(&E, usize, Duration),
    {
        let mut attempt = 0;
        loop {
            match operation() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    // Check if we've reached max retries
                    if attempt >= self.max_retries {
                        return Err(err);
                    }
                    
                    // Check if this error is retryable
                    let should_retry = match &self.retry_if {
                        Some(predicate) => predicate(&err),
                        None => true,
                    };
                    
                    if !should_retry {
                        return Err(err);
                    }
                    
                    // Get the delay for this attempt
                    let delay = self.backoff.next_delay(attempt);
                    
                    // Call the error handler
                    on_error(&err, attempt, delay);
                    
                    // Wait according to backoff strategy
                    thread::sleep(delay);
                    
                    attempt += 1;
                }
            }
        }
    }
}

/// Policy for retrying operations
pub struct RetryPolicy {
    max_retries: usize,
    backoff_type: BackoffType,
}

/// Available backoff types for retry policy
pub enum BackoffType {
    Exponential,
    Linear,
    Fixed(u64),
}

impl RetryPolicy {
    /// Create a new retry policy with exponential backoff
    pub fn new_exponential() -> Self {
        Self {
            max_retries: 3,
            backoff_type: BackoffType::Exponential,
        }
    }
    
    /// Create a new retry policy with linear backoff
    pub fn new_linear() -> Self {
        Self {
            max_retries: 3,
            backoff_type: BackoffType::Linear,
        }
    }
    
    /// Create a new retry policy with fixed backoff
    pub fn new_fixed(delay_ms: u64) -> Self {
        Self {
            max_retries: 3,
            backoff_type: BackoffType::Fixed(delay_ms),
        }
    }
    
    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// Create a retry executor for the given error type
    pub fn executor<E>(&self) -> RetryExecutor<E>
    where
        E: std::error::Error + 'static
    {
        let executor = match self.backoff_type {
            BackoffType::Exponential => RetryExecutor::new_exponential(),
            BackoffType::Linear => RetryExecutor::new_linear(),
            BackoffType::Fixed(delay_ms) => RetryExecutor::new_fixed(delay_ms),
        };
        
        executor.with_max_retries(self.max_retries)
    }
    
    /// Create a retry executor specifically for ForgeError types
    pub fn forge_executor<E>(&self) -> RetryExecutor<E>
    where
        E: ForgeError
    {
        self.executor::<E>()
            .with_retry_if(|err| err.is_retryable())
    }
    
    /// Execute a fallible operation with retries
    pub fn retry<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::error::Error + 'static
    {
        self.executor::<E>().retry(operation)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new_exponential()
    }
}
