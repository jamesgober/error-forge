/// Provides macros for grouping errors and generating automatic conversions
// StdError used in generated code from macros
#[allow(unused_imports)]
use std::error::Error as StdError;

/// Macro for composing multi-error enums with automatic `From<OtherError>` conversions.
/// 
/// This macro allows you to create a parent error type that can wrap multiple other error types,
/// automatically implementing From conversions for each of them.
/// 
/// # Example
/// 
/// ```ignore
/// use error_forge::{group, AppError};
/// use std::io;
/// 
/// // Define a custom error type for a specific module
/// #[derive(Debug, thiserror::Error)]
/// pub enum DatabaseError {
///     #[error("Connection failed: {0}")]
///     ConnectionFailed(String),
///     
///     #[error("Query failed: {0}")]
///     QueryFailed(String),
/// }
/// 
/// // Group multiple error types into a parent error
/// group! {
///     #[derive(Debug)]
///     pub enum ServiceError {
///         // Include the AppError
///         App(AppError),
///         
///         // Include io::Error
///         Io(io::Error),
///         
///         // Include the custom DatabaseError
///         Database(DatabaseError),
///     }
/// }
/// ```
#[macro_export]
macro_rules! group {
    // First pattern - simple wrapped errors without extra variants
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident($source_type:ty)
            ),* $(,)?
        }
    ) => {
        group!(@with_impl
            $(#[$meta])* $vis enum $name {
                $(
                    $(#[$vmeta])*
                    $variant($source_type),
                )*
            }
            $(
                $variant $source_type
            )*
        )
    };
    
    // Internal implementation with all necessary impls
    (@with_impl
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident($source_type:ty),
            )*
            $(
                $(#[$vmeta_extra:meta])*
                $variant_extra:ident $({$($field:ident: $field_type:ty),*})?,
            )*
        }
        $($impl_variant:ident $impl_type:ty)*
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant($source_type),
            )*
            $(
                $(#[$vmeta_extra])*
                $variant_extra $({$($field: $field_type),*})?,
            )*
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$variant(source) => write!(f, "{}", source),
                    )*
                    $(
                        Self::$variant_extra $({$($field),*})? => {
                            let error_name = stringify!($variant_extra);
                            write!(f, "{} error", error_name)
                        }
                    )*
                }
            }
        }

        impl std::error::Error for $name {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    $(
                        Self::$variant(source) => Some(source as &(dyn std::error::Error + 'static)),
                    )*
                    _ => None,
                }
            }
        }

        $(
            impl From<$source_type> for $name {
                fn from(source: $source_type) -> Self {
                    Self::$variant(source)
                }
            }
        )*

        // Implement ForgeError trait for our grouped error enum
        impl $crate::error::ForgeError for $name {
            fn kind(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.kind();
                            }
                            stringify!($variant)
                        },
                    )*
                    $(
                        Self::$variant_extra $({$($field),*})? => stringify!($variant_extra),
                    )*
                }
            }
            
            fn user_message(&self) -> String {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.user_message();
                            }
                            source.to_string()
                        },
                    )*
                    $(
                        Self::$variant_extra $({$($field),*})? => self.to_string(),
                    )*
                }
            }
            
            fn caption(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.caption();
                            }
                            concat!(stringify!($variant), ": Error")
                        },
                    )*
                    $(
                        Self::$variant_extra $({$($field),*})? => {
                            concat!(stringify!($variant_extra), ": Error")
                        },
                    )*
                }
            }
            
            fn is_retryable(&self) -> bool {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.is_retryable();
                            }
                            false
                        },
                    )*
                    _ => false,
                }
            }
            
            fn status_code(&self) -> u16 {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.status_code();
                            }
                            500
                        },
                    )*
                    _ => 500,
                }
            }
            
            fn exit_code(&self) -> i32 {
                match self {
                    $(
                        Self::$variant(source) => {
                            if let Some(forge_err) = (source as &dyn std::any::Any)
                                .downcast_ref::<&(dyn $crate::error::ForgeError)>() 
                            {
                                return forge_err.exit_code();
                            }
                            1
                        },
                    )*
                    _ => 1,
                }
            }
        }
    };
}
