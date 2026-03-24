/// Error hook types for centralized error handling
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorLevel {
    /// Debug-level errors (for detailed debugging)
    Debug,
    /// Information-level errors (least severe)
    Info,
    /// Warning-level errors (moderate severity)
    Warning,
    /// Error-level errors (high severity)
    Error,
    /// Critical-level errors (most severe)
    Critical,
}

/// Error context passed to registered hooks
pub struct ErrorContext<'a> {
    /// The error caption
    pub caption: &'a str,
    /// The error kind
    pub kind: &'a str,
    /// The error level
    pub level: ErrorLevel,
    /// Whether the error is fatal
    pub is_fatal: bool,
    /// Whether the error can be retried
    pub is_retryable: bool,
}

use std::sync::OnceLock;

/// Global error hook for centralized error handling
static ERROR_HOOK: OnceLock<fn(ErrorContext)> = OnceLock::new();

#[doc(hidden)]
pub trait ErrorSource {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)>;
}

impl ErrorSource for std::io::Error {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self)
    }
}

impl ErrorSource for Box<dyn std::error::Error + Send + Sync> {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.as_ref())
    }
}

impl ErrorSource for Box<dyn std::error::Error> {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.as_ref())
    }
}

impl ErrorSource for Option<std::io::Error> {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

impl ErrorSource for Option<Box<dyn std::error::Error + Send + Sync>> {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.as_deref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

impl ErrorSource for Option<Box<dyn std::error::Error>> {
    fn as_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.as_deref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

/// Register a callback function to be called when errors are created
///
/// # Example
///
/// ```rust
/// use error_forge::{AppError, macros::{register_error_hook, ErrorLevel, ErrorContext}};
///
/// // Setup logging with different levels
/// register_error_hook(|ctx| {
///     match ctx.level {
///         ErrorLevel::Debug => println!("DEBUG: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Info => println!("INFO: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Warning => println!("WARNING: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Error => println!("ERROR: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Critical => println!("CRITICAL: {} ({})", ctx.caption, ctx.kind),
///     }
///     
///     // Optional: send notifications for critical errors
///     if ctx.level == ErrorLevel::Critical || ctx.is_fatal {
///         // send_notification("Critical error occurred", ctx.caption);
///     }
/// });
/// ```
///
pub fn register_error_hook(callback: fn(ErrorContext)) {
    let _ = try_register_error_hook(callback);
}

/// Attempt to register a callback function to be called when errors are created.
///
/// Returns an error if a hook is already registered.
pub fn try_register_error_hook(callback: fn(ErrorContext)) -> Result<(), &'static str> {
    ERROR_HOOK
        .set(callback)
        .map_err(|_| "Error hook already registered")
}

/// Call the registered error hook with error context if one is registered
#[doc(hidden)]
pub fn call_error_hook(caption: &str, kind: &str, is_fatal: bool, is_retryable: bool) {
    if let Some(hook) = ERROR_HOOK.get() {
        // Determine error level based on error properties
        let level = if is_fatal {
            ErrorLevel::Critical
        } else if !is_retryable {
            ErrorLevel::Error
        } else if kind == "Warning" {
            ErrorLevel::Warning
        } else if kind == "Debug" {
            ErrorLevel::Debug
        } else {
            ErrorLevel::Info
        };

        hook(ErrorContext {
            caption,
            kind,
            level,
            is_fatal,
            is_retryable,
        });
    }
}

#[macro_export]
macro_rules! define_errors {
    (
        $(
            $(#[$meta:meta])* $vis:vis enum $name:ident {
                $(
                   $(#[error(display = $display:literal $(, $($display_param:ident),* )?)])?
                   #[kind($kind:ident $(, $($tag:ident = $val:expr),* )?)]
                   $variant:ident $( { $($field:ident : $ftype:ty),* $(,)? } )?, )*
            }
        )*
    ) => {
        $(
            $(#[$meta])* #[derive(Debug)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            $vis enum $name {
                $( $variant $( { $($field : $ftype),* } )?, )*
            }

            impl $name {
                $(
                    paste::paste! {
                        pub fn [<$variant:lower>]($($($field : $ftype),*)?) -> Self {
                            let instance = Self::$variant $( { $($field),* } )?;
                            // Call the error hook - no need to directly access ERROR_HOOK here
                            $crate::macros::call_error_hook(
                                instance.caption(),
                                instance.kind(),
                                instance.is_fatal(),
                                instance.is_retryable()
                            );
                            instance
                        }
                    }
                )*

                pub fn caption(&self) -> &'static str {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_caption $kind $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn kind(&self) -> &'static str {
                    match self {
                        $( Self::$variant { .. } => {
                            stringify!($kind)
                        } ),*
                    }
                }

                pub fn is_retryable(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag retryable, false $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn is_fatal(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag fatal, false $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn status_code(&self) -> u16 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag status, 500 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn exit_code(&self) -> i32 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag exit, 1 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            $(
                                #[allow(unused_variables)]
                                if let Some(display) = define_errors!(@format_display $display $(, $($display_param),*)?) {
                                    return write!(f, "{}", display);
                                }
                            )?
                            // If no custom display format is provided, use a default format
                            write!(f, "{}: ", self.caption())?;
                            write!(f, stringify!($variant))?;
                            // Format each field with name=value
                            $( $(
                                write!(f, " | {} = ", stringify!($field))?
                                ;
                                match stringify!($field) {
                                    "source" => write!(f, "{}", $field)?,
                                    _ => write!(f, "{:?}", $field)?,
                                }
                            ; )* )?
                            Ok(())
                        } ),*
                    }
                }
            }

            impl std::error::Error for $name {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            define_errors!(@find_source $( $($field),* )? )
                        } ),*
                    }
                }
            }
        )*
    };

    (@find_source) => {
        None
    };

    (@find_source $field:ident $(, $rest:ident)*) => {
        define_errors!(@find_source_match $field, $field $(, $rest)*)
    };

    (@find_source_match source, $source_field:ident $(, $rest:ident)*) => {
        $crate::macros::ErrorSource::as_source($source_field)
    };

    (@find_source_match $field_name:ident, $field:ident $(, $rest:ident)*) => {
        define_errors!(@find_source $($rest),*)
    };

    (@get_caption $kind:ident) => {
        stringify!($kind)
    };

    (@get_caption $kind:ident, caption = $caption:expr $(, $($rest:tt)*)?) => {
        $caption
    };

    (@get_caption $kind:ident, $tag:ident = $val:expr $(, $($rest:tt)*)?) => {
        define_errors!(@get_caption $kind $(, $($rest)*)?)
    };

    (@get_tag $target:ident, $default:expr) => {
        $default
    };

    (@get_tag retryable, $default:expr, retryable = $val:expr $(, $($rest:tt)*)?) => {
        $val
    };

    (@get_tag fatal, $default:expr, fatal = $val:expr $(, $($rest:tt)*)?) => {
        $val
    };

    (@get_tag status, $default:expr, status = $val:expr $(, $($rest:tt)*)?) => {
        $val
    };

    (@get_tag exit, $default:expr, exit = $val:expr $(, $($rest:tt)*)?) => {
        $val
    };

    (@get_tag $target:ident, $default:expr, $tag:ident = $val:expr $(, $($rest:tt)*)?) => {
        define_errors!(@get_tag $target, $default $(, $($rest)*)?)
    };

    (@format_display $display:literal) => {
        Some($display.to_string())
    };

    (@format_display $display:literal, $($param:ident),+) => {
        Some(format!($display, $($param = $param),+))
    };

    // Support for nested field access in error display formatting
    (@format_display_field $field:ident) => {
        $field
    };

    (@format_display_field $field:ident . $($rest:ident).+) => {
        $field$(.$rest)+
    };
}
