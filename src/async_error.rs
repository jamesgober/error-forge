use std::backtrace::Backtrace;
use std::error::Error as StdError;

#[cfg(feature = "async")]
use async_trait::async_trait;

/// Async-aware companion trait to [`ForgeError`](crate::error::ForgeError).
///
/// Carries the same sync metadata methods as `ForgeError` plus a single
/// async hook, [`async_handle`](Self::async_handle), that callers can
/// override to run an `await` step (logging, telemetry, cleanup) when
/// an error surfaces in an async context. The default implementation
/// is a no-op — implementors who do not need an async hook can ignore
/// the method entirely.
///
/// # Example
///
/// Requires the `async` cargo feature (which pulls in `async-trait`).
/// The hidden `#[cfg(feature = "async")]` gate means this doctest is
/// only compiled when the feature is enabled — under
/// `cargo test --all-features` it runs; otherwise it is silently
/// skipped.
///
/// ```
/// # #[cfg(feature = "async")] {
/// use error_forge::async_error::AsyncForgeError;
/// use async_trait::async_trait;
/// use std::error::Error as StdError;
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
///     fn kind(&self) -> &'static str { "AsyncExample" }
///     fn caption(&self) -> &'static str { "Async Example Error" }
///
///     // Override the no-op default if you want async behaviour:
///     async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
///         // Telemetry, cleanup, etc. would go here.
///         Ok(())
///     }
/// }
/// # }
/// ```
///
/// # Breaking change from `0.9.x`
///
/// In `0.9.x`, [`async_handle`](Self::async_handle) was a required
/// method with no default body, and the [`AppError`](crate::error::AppError)
/// implementation provided a stub that did nothing. In `1.0`,
/// [`async_handle`](Self::async_handle) gains a default no-op body
/// and the stub `AppError` implementation is removed. Implementors
/// who actually want async behaviour override the default; everyone
/// else can derive the trait without writing the method.
#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncForgeError: StdError + Send + Sync + 'static {
    /// Returns the kind of error, typically matching the enum variant.
    fn kind(&self) -> &'static str;

    /// Returns a human-readable caption for the error.
    fn caption(&self) -> &'static str;

    /// Returns true if the operation can be retried.
    fn is_retryable(&self) -> bool {
        false
    }

    /// Returns true if the error is fatal and should terminate the program.
    fn is_fatal(&self) -> bool {
        false
    }

    /// Returns an appropriate HTTP status code for the error.
    fn status_code(&self) -> u16 {
        500
    }

    /// Returns an appropriate process exit code for the error.
    fn exit_code(&self) -> i32 {
        1
    }

    /// Returns a user-facing message that can be shown to end users.
    fn user_message(&self) -> String {
        self.to_string()
    }

    /// Returns a detailed technical message for developers/logs.
    fn dev_message(&self) -> String {
        format!("[{}] {}", self.kind(), self)
    }

    /// Returns a backtrace if available.
    fn backtrace(&self) -> Option<&Backtrace> {
        None
    }

    /// Async hook called when the implementor wants to run an
    /// `await` step (logging, cleanup, telemetry) on error surfacing.
    /// The default is a no-op; override if you need behaviour.
    async fn async_handle(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        Ok(())
    }

    /// Registers the error with the central error hook (if any).
    fn register(&self) {
        crate::macros::call_error_hook(
            self.caption(),
            self.kind(),
            self.is_fatal(),
            self.is_retryable(),
        );
    }
}

/// Type alias for async error-forge results.
#[cfg(feature = "async")]
pub type AsyncResult<T, E> = std::result::Result<T, E>;
