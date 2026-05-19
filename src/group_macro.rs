/// Provides macros for grouping errors and generating automatic
/// conversions.
// `StdError` is referenced from generated code in the macro body.
#[allow(unused_imports)]
use std::error::Error as StdError;

/// Macro for composing multi-error enums with automatic
/// `From<OtherError>` conversions and full [`ForgeError`] delegation.
///
/// Every variant must wrap exactly one source type that itself
/// implements [`ForgeError`]. The macro generates:
///
/// - the enum declaration,
/// - `Display` and `Error` implementations that forward to the
///   wrapped source,
/// - `From<T>` for each wrapped type,
/// - a [`ForgeError`] impl whose methods delegate directly to the
///   wrapped source's `ForgeError` methods (no type-erased
///   downcast, no fallback values).
///
/// # Example
///
/// ```
/// use error_forge::{group, AppError};
///
/// // `AppError` already implements `ForgeError`, so it can be
/// // wrapped directly. Other types you wrap with `group!` must
/// // also implement `ForgeError` (use `define_errors!` or
/// // `#[derive(ModError)]` to produce them).
/// group! {
///     #[derive(Debug)]
///     pub enum ServiceError {
///         App(AppError),
///     }
/// }
///
/// // `From<AppError>` is generated, so `?` works against any
/// // function that returns an `AppError`.
/// let _err: ServiceError = AppError::config("missing").into();
/// ```
///
/// # `ForgeError` requirement
///
/// Each wrapped source type must implement [`ForgeError`]. If you
/// need to compose with a type that does not (e.g. `std::io::Error`),
/// wrap it once in a `define_errors!` enum variant or
/// `#[derive(ModError)]` enum and then group the result.
///
/// This is a **breaking change from `0.9.x`**, where `group!`
/// accepted any wrapped type but the resulting `ForgeError` impl
/// was silently incorrect — every method returned default
/// fallback values regardless of the wrapped type. The `1.0`
/// rewrite removes the type-erased downcast and trades it for a
/// trait bound the compiler can verify.
///
/// [`ForgeError`]: crate::error::ForgeError
#[macro_export]
macro_rules! group {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident($source_type:ty)
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant($source_type),
            )*
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $(
                        Self::$variant(source) => ::std::fmt::Display::fmt(source, f),
                    )*
                }
            }
        }

        impl ::std::error::Error for $name {
            fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    $(
                        Self::$variant(source) => {
                            ::std::option::Option::Some(source as &(dyn ::std::error::Error + 'static))
                        }
                    )*
                }
            }
        }

        $(
            impl ::std::convert::From<$source_type> for $name {
                fn from(source: $source_type) -> Self {
                    Self::$variant(source)
                }
            }
        )*

        impl $crate::error::ForgeError for $name {
            fn kind(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::kind(source),
                    )*
                }
            }

            fn caption(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::caption(source),
                    )*
                }
            }

            fn is_retryable(&self) -> bool {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::is_retryable(source),
                    )*
                }
            }

            fn is_fatal(&self) -> bool {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::is_fatal(source),
                    )*
                }
            }

            fn status_code(&self) -> u16 {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::status_code(source),
                    )*
                }
            }

            fn exit_code(&self) -> i32 {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::exit_code(source),
                    )*
                }
            }

            fn user_message(&self) -> ::std::string::String {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::user_message(source),
                    )*
                }
            }

            fn dev_message(&self) -> ::std::string::String {
                match self {
                    $(
                        Self::$variant(source) => $crate::error::ForgeError::dev_message(source),
                    )*
                }
            }
        }
    };
}
