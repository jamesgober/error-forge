use crate::error::ForgeError;
use crate::recovery::backoff::{Backoff, ExponentialBackoff, FixedBackoff, LinearBackoff};
use std::marker::PhantomData;
use std::thread;
use std::time::Duration;

/// Predicate function to determine if an error is retryable
pub type RetryPredicate<E> = Box<dyn Fn(&E) -> bool + Send + Sync + 'static>;

/// Whether the exponential executors jitter by default: on when the
/// `jitter` cargo feature is enabled.
const DEFAULT_JITTER: bool = cfg!(feature = "jitter");

/// Enum to hold different backoff strategy types
pub enum BackoffStrategy {
    Exponential(ExponentialBackoff),
    Linear(LinearBackoff),
    Fixed(FixedBackoff),
    /// Any [`Backoff`], set with [`RetryExecutor::with_backoff`].
    Custom(Box<dyn Backoff>),
}

impl BackoffStrategy {
    fn next_delay(&self, attempt: usize) -> Duration {
        match self {
            BackoffStrategy::Exponential(b) => b.next_delay(attempt),
            BackoffStrategy::Linear(b) => b.next_delay(attempt),
            BackoffStrategy::Fixed(b) => b.next_delay(attempt),
            BackoffStrategy::Custom(b) => b.next_delay(attempt),
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
    E: std::error::Error + 'static,
{
    /// Create a new retry executor with an exponential backoff strategy.
    ///
    /// Delays start at 100 ms and double each attempt, up to 30 s. With
    /// the `jitter` cargo feature on, each delay is jittered by ±20%
    /// (see [`ExponentialBackoff::with_jitter`]) so retriers across a
    /// fleet do not wake in step; use [`with_jitter`](Self::with_jitter)
    /// or [`with_backoff`](Self::with_backoff) to choose explicitly.
    pub fn new_exponential() -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffStrategy::Exponential(
                ExponentialBackoff::default().with_jitter(DEFAULT_JITTER),
            ),
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

    /// Use `backoff` for the delay before each retry, replacing the
    /// built-in schedule. Any [`Backoff`] works, including a configured
    /// [`ExponentialBackoff`] (initial delay, factor, cap, jitter).
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::{ExponentialBackoff, RetryExecutor};
    ///
    /// let executor = RetryExecutor::<std::io::Error>::new_exponential().with_backoff(
    ///     ExponentialBackoff::new()
    ///         .with_initial_delay(1)
    ///         .with_max_delay(5)
    ///         .with_jitter(true),
    /// );
    /// let result = executor.retry(|| Ok::<_, std::io::Error>(7));
    /// assert_eq!(result.unwrap(), 7);
    /// ```
    #[must_use]
    pub fn with_backoff(mut self, backoff: impl Backoff) -> Self {
        self.backoff = BackoffStrategy::Custom(Box::new(backoff));
        self
    }

    /// Turn ±20% jitter on or off for an exponential executor (see
    /// [`ExponentialBackoff::with_jitter`]). Has no effect on linear,
    /// fixed, or custom backoff.
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::RetryExecutor;
    ///
    /// let executor = RetryExecutor::<std::io::Error>::new_exponential().with_jitter(true);
    /// assert!(executor.retry(|| Ok::<_, std::io::Error>(())).is_ok());
    /// ```
    #[must_use]
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        if let BackoffStrategy::Exponential(backoff) = &mut self.backoff {
            *backoff = backoff.clone().with_jitter(jitter);
        }
        self
    }

    /// Set a predicate to determine if an error should be retried
    pub fn with_retry_if<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        self.retry_if = Some(Box::new(predicate));
        self
    }

    /// Execute a fallible operation with retries
    pub fn retry<F, T>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
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
    jitter: bool,
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
            jitter: DEFAULT_JITTER,
        }
    }

    /// Create a new retry policy with linear backoff
    pub fn new_linear() -> Self {
        Self {
            max_retries: 3,
            backoff_type: BackoffType::Linear,
            jitter: DEFAULT_JITTER,
        }
    }

    /// Create a new retry policy with fixed backoff
    pub fn new_fixed(delay_ms: u64) -> Self {
        Self {
            max_retries: 3,
            backoff_type: BackoffType::Fixed(delay_ms),
            jitter: DEFAULT_JITTER,
        }
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Turn ±20% jitter on or off for an exponential policy (see
    /// [`ExponentialBackoff::with_jitter`]). On by default with the
    /// `jitter` cargo feature, off without it. Has no effect on linear
    /// or fixed backoff.
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::RetryPolicy;
    ///
    /// let policy = RetryPolicy::new_exponential().with_jitter(true);
    /// let result: Result<(), std::io::Error> = policy.retry(|| Ok(()));
    /// assert!(result.is_ok());
    /// ```
    #[must_use]
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Create a retry executor for the given error type
    pub fn executor<E>(&self) -> RetryExecutor<E>
    where
        E: std::error::Error + 'static,
    {
        let executor = match self.backoff_type {
            BackoffType::Exponential => RetryExecutor::new_exponential().with_jitter(self.jitter),
            BackoffType::Linear => RetryExecutor::new_linear(),
            BackoffType::Fixed(delay_ms) => RetryExecutor::new_fixed(delay_ms),
        };

        executor.with_max_retries(self.max_retries)
    }

    /// Create a retry executor specifically for ForgeError types
    pub fn forge_executor<E>(&self) -> RetryExecutor<E>
    where
        E: ForgeError,
    {
        self.executor::<E>().with_retry_if(|err| err.is_retryable())
    }

    /// Execute a fallible operation with retries
    pub fn retry<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::error::Error + 'static,
    {
        self.executor::<E>().retry(operation)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new_exponential()
    }
}
