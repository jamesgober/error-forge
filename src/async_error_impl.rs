#[cfg(feature = "async")]
use async_trait::async_trait;
use std::error::Error as StdError;

use crate::async_error::AsyncForgeError;
use crate::error::AppError;

/// `AppError` participates in the `AsyncForgeError` surface so it can
/// be used wherever async-aware error metadata is required.
///
/// All sync metadata methods (`kind`, `caption`, `is_retryable`,
/// `is_fatal`, `status_code`, `exit_code`, `user_message`,
/// `dev_message`) delegate to the existing
/// [`ForgeError`](crate::error::ForgeError) implementation. The
/// async [`async_handle`](AsyncForgeError::async_handle) method uses
/// the trait's default no-op implementation — `AppError` has no
/// default async behaviour beyond carrying its metadata.
///
/// # Breaking change from `0.9.x`
///
/// `0.9.x` shipped a stub `async_handle` implementation here that
/// returned `Ok(())` regardless of input but matched on `AppError`
/// variants as if it were doing something. The stub is removed in
/// `1.0`; the trait now provides a no-op default and `AppError`
/// inherits it.
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

    // `async_handle` uses the trait default (no-op `Ok(())`).
}

#[cfg(feature = "async")]
impl AppError {
    /// Convert an `async` operation's `Result<T, E>` (where `E:
    /// std::error::Error + Send + Sync + 'static`) into a
    /// `Result<T, AppError>` by wrapping the error in
    /// `AppError::other(...)`, marked retryable and non-fatal.
    ///
    /// Convenience for the common `async fn -> Result<T, AppError>`
    /// pattern where the caller wants to flatten any
    /// `std::error::Error` into an `AppError` shape.
    pub async fn from_async_result<T, E>(result: Result<T, E>) -> Result<T, Self>
    where
        E: StdError + Send + Sync + 'static,
    {
        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(Self::other(e.to_string())
                .with_fatal(false)
                .with_retryable(true)),
        }
    }

    /// Run the async hook on this error.
    ///
    /// Equivalent to calling
    /// `<AppError as AsyncForgeError>::async_handle(&self).await`;
    /// `AppError` does not override the trait default, so this is a
    /// no-op `Ok(())`. Exists for symmetry with the other
    /// `*_async` builders on `AppError`.
    pub async fn handle_async(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        <Self as AsyncForgeError>::async_handle(self).await
    }
}
