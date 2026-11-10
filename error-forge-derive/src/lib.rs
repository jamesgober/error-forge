//! Procedural macros for the `error-forge` crate.
//!
//! - `#[derive(ModError)]`, behind the default `derive` feature (pulls
//!   in `syn`, `quote` and `proc-macro2`).
//! - An internal identifier-pasting macro used by `define_errors!`,
//!   always available and built on the compiler's `proc_macro` API
//!   alone. It is not part of the public API.
//!
//! Use these through `error-forge`; see its documentation.

extern crate proc_macro;

#[cfg(feature = "derive")]
mod mod_error;
mod paste;

use proc_macro::TokenStream;

/// Derive macro for ModError
///
/// This macro automatically implements the ForgeError trait and common
/// error handling functionality for a struct or enum, allowing for
/// "lazy mode" error creation with minimal boilerplate.
///
/// # Example
///
/// When the macro is used in your application where error-forge is a dependency:
///
/// ```ignore
/// use error_forge::ModError;
///
/// #[derive(Debug, ModError)]
/// #[error_prefix("Database")]
/// pub enum DbError {
///     #[error_display("Connection to {0} failed")]
///     ConnectionFailed(String),
///
///     #[error_display("Query execution failed: {reason}")]
///     QueryFailed { reason: String },
///
///     #[error_display("Transaction error")]
///     #[error_http_status(400)]
///     TransactionError,
/// }
/// ```
///
/// `#[error_display("...")]` is optional; without it a variant displays
/// as its name. The string is a `format!` string and may reference any
/// subset of the variant's fields: by name for struct-like variants
/// (a raw identifier such as `r#type` is written `{type}`), by position
/// (`{0}`, `{}`) for tuple variants. Fields it does not mention are
/// ignored, `{{` and `}}` produce literal braces, and a width or
/// precision given where the error is formatted (`{:>20}`) pads or
/// truncates the whole message.
///
/// # Attributes
///
/// On the enum (or struct):
///
/// - `#[error_prefix("...")]`: caption prefix, giving `"<prefix>: Error"`.
///
/// On each enum variant:
///
/// - `#[error_display("...")]`, `#[error_kind("...")]`,
///   `#[error_caption("...")]`: string literals.
/// - `#[error_retryable]`, `#[error_fatal]`: flags. A bare flag means
///   `true`; an explicit value (`#[error_retryable(false)]` or
///   `#[error_retryable = false]`) is honoured.
/// - `#[error_http_status(404)]`: an integer that fits in `u16`.
/// - `#[error_exit_code(2)]`: an integer that fits in `i32`.
///
/// Each attribute accepts both the list form (`#[name(value)]`) and the
/// name-value form (`#[name = value]`). A value of the wrong type or out
/// of range is a compile error.
///
/// # Structs
///
/// On a struct only `#[error_prefix]` is read. The struct displays as
/// `"<prefix>: Error"`, its `kind()` is the struct name, and the other
/// metadata methods use the `ForgeError` defaults. The variant
/// attributes listed above (`#[error_display]`, `#[error_retryable]`,
/// and so on) are currently ignored when placed on a struct.
///
/// Note: This is a procedural macro that is re-exported by the `error-forge` crate.
/// When using in your application, import it from the main crate with `use error_forge::ModError;`.
#[cfg(feature = "derive")]
#[proc_macro_derive(
    ModError,
    attributes(
        error_prefix,
        error_display,
        error_kind,
        error_caption,
        error_retryable,
        error_http_status,
        error_exit_code,
        error_fatal
    )
)]
pub fn derive_mod_error(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    TokenStream::from(mod_error::expand(&input))
}

/// Identifier pasting for `error-forge`'s `macro_rules!` macros.
///
/// Not public API: `error-forge` re-exports it as
/// `error_forge::__private::paste!` for `define_errors!`, and it may
/// change in any release. Each `[< ... >]` bracket in the input is
/// replaced by one identifier built from the segments inside it:
/// identifiers (a leading `r#` is dropped), each optionally followed by
/// `:lower` or `:upper`. All other tokens pass through unchanged.
///
/// # Example
///
/// ```ignore
/// error_forge::__private::paste! {
///     fn [<NotFound:lower>]() {}
/// }
/// // expands to `fn notfound() {}`
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __paste(input: TokenStream) -> TokenStream {
    paste::expand(input)
}
