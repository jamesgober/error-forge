/// Error severity level passed to a registered hook callback.
///
/// Marked `#[non_exhaustive]` so future minor releases can add new
/// severity variants (e.g. `Notice`, `Trace`) without breaking
/// existing `match` statements.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
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

/// Error context passed to registered hooks.
///
/// Marked `#[non_exhaustive]` so future minor releases can add new
/// fields without breaking callers that destructure the struct.
/// Construct via [`ErrorContext::new`] (rather than struct-literal
/// syntax) from outside the crate.
#[non_exhaustive]
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

impl<'a> ErrorContext<'a> {
    /// Construct an [`ErrorContext`] from its components.
    ///
    /// Provided so external callers (tests, custom hook wiring) can
    /// build the struct without depending on its field list, which
    /// may grow over the `1.x` line.
    pub fn new(
        caption: &'a str,
        kind: &'a str,
        level: ErrorLevel,
        is_fatal: bool,
        is_retryable: bool,
    ) -> Self {
        Self {
            caption,
            kind,
            level,
            is_fatal,
            is_retryable,
        }
    }
}

use std::sync::OnceLock;

/// Hook callback type.
///
/// Stored as a boxed `Fn` so callers can capture environment in a
/// closure (a `Write`-implementing buffer, a thread-safe logger
/// handle, an `Arc<Config>`, etc.). The `Send + Sync` bounds let
/// the hook fire from any thread.
type ErrorHookFn = Box<dyn Fn(ErrorContext<'_>) + Send + Sync + 'static>;

/// Global error hook for centralized error handling.
static ERROR_HOOK: OnceLock<ErrorHookFn> = OnceLock::new();

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

/// Register a callback to be called when errors are created.
///
/// **Deprecated since `1.0.0`.** This variant silently discards
/// the registration failure when a hook is already installed.
/// Use [`try_register_error_hook`] instead — it returns the
/// failure explicitly so callers can decide how to handle the
/// double-registration case.
///
/// # Example
///
/// ```
/// use error_forge::macros::{try_register_error_hook, ErrorLevel};
///
/// let _ = try_register_error_hook(|ctx| {
///     match ctx.level {
///         ErrorLevel::Debug => println!("DEBUG: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Info => println!("INFO: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Warning => println!("WARNING: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Error => println!("ERROR: {} ({})", ctx.caption, ctx.kind),
///         ErrorLevel::Critical => println!("CRITICAL: {} ({})", ctx.caption, ctx.kind),
///         // `ErrorLevel` is `#[non_exhaustive]` — minor releases
///         // may add new severity levels.
///         _ => println!("OTHER: {} ({})", ctx.caption, ctx.kind),
///     }
/// });
/// ```
#[deprecated(
    since = "1.0.0",
    note = "register_error_hook silently drops registration failures; use \
            try_register_error_hook instead"
)]
pub fn register_error_hook<F>(callback: F)
where
    F: Fn(ErrorContext<'_>) + Send + Sync + 'static,
{
    let _ = try_register_error_hook(callback);
}

/// Attempt to register a callback to be called when errors are
/// created.
///
/// The callback may be a function pointer or a closure capturing
/// thread-safe state. Only one hook can be registered per process;
/// subsequent calls return `Err("Error hook already registered")`.
///
/// # Example
///
/// ```
/// use error_forge::macros::try_register_error_hook;
/// use std::sync::{Arc, Mutex};
///
/// // Closures that capture state work too — not just function pointers.
/// let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
/// let log_for_hook = Arc::clone(&log);
/// let _ = try_register_error_hook(move |ctx| {
///     log_for_hook
///         .lock()
///         .unwrap()
///         .push(format!("{}: {}", ctx.kind, ctx.caption));
/// });
/// ```
pub fn try_register_error_hook<F>(callback: F) -> Result<(), &'static str>
where
    F: Fn(ErrorContext<'_>) + Send + Sync + 'static,
{
    ERROR_HOOK
        .set(Box::new(callback))
        .map_err(|_| "Error hook already registered")
}

thread_local! {
    /// Set while the registered hook runs on this thread. An error
    /// created from inside the hook would otherwise call the hook
    /// again and recurse until the stack overflows.
    static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Clears [`IN_HOOK`] when the hook call ends, including by unwinding.
struct InHookGuard;

impl Drop for InHookGuard {
    fn drop(&mut self) {
        IN_HOOK.with(|flag| flag.set(false));
    }
}

/// Call the registered error hook with error context if one is registered.
///
/// The hook is not re-entered: errors created while the hook runs on the
/// same thread (for example by a logging sink that fails) do not call it
/// again. A panic inside the hook is caught here so it does not unwind
/// through the error constructor that fired it; the process panic hook
/// still reports it.
#[doc(hidden)]
pub fn call_error_hook(caption: &str, kind: &str, is_fatal: bool, is_retryable: bool) {
    let Some(hook) = ERROR_HOOK.get() else {
        return;
    };
    if IN_HOOK.with(|flag| flag.replace(true)) {
        return;
    }
    let _guard = InHookGuard;

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

    let context = ErrorContext {
        caption,
        kind,
        level,
        is_fatal,
        is_retryable,
    };

    // `AssertUnwindSafe`: the closure only borrows the shared hook and
    // `&str`s. Nothing this function owns is left half-updated by an
    // unwind (the re-entrancy flag is reset by `_guard` either way), and
    // any state the hook keeps for itself is the hook's own concern, as
    // it would be if the panic had reached the caller.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hook(context)));
    if let Err(payload) = outcome {
        // Dropping a payload can itself panic; never let that escape.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || drop(payload)));
    }
}

/// Declare an error enum with constructors, metadata methods,
/// `Display` and `std::error::Error`.
///
/// Each variant needs a `#[kind(Name, tag = value, ...)]` attribute and
/// may carry one `#[error(display = "...", field, ...)]` attribute. The
/// two may appear in either order, and doc comments or other attributes
/// (`#[allow(...)]`, `#[serde(...)]`, ...) are passed through to the
/// variant. The trailing comma after the last variant is optional.
///
/// Recognised `#[kind]` tags are `caption`, `retryable`, `fatal`,
/// `status` and `exit`; any other tag is a compile error.
///
/// The macro generates, for each enum:
///
/// - the enum itself with `#[derive(Debug)]`;
/// - a constructor per variant named after the lowercased variant
///   (`Config` becomes `config(...)`), which fires the registered
///   error hook;
/// - inherent `kind`, `caption`, `is_retryable`, `is_fatal`,
///   `status_code` and `exit_code` methods (it does **not** implement
///   [`ForgeError`](crate::ForgeError); see the [`group!`](crate::group)
///   docs for the delegating impl);
/// - `Display`, using the `display` string when one is given, or
///   `"<caption>: <Variant> | field = value ..."` otherwise. Only the
///   default format needs the fields to implement `Debug` (and
///   `Display` for a field named `source`); a custom display string
///   only needs what it formats;
/// - `std::error::Error`, whose `source()` returns a field named
///   `source` through [`ErrorSource`].
///
/// The display string is a `format!` string. Fields listed after it are
/// passed as named arguments, and fields it names inline (`{path:?}`)
/// are captured directly, so `{{` and `}}` produce literal braces.
///
/// # Example
///
/// ```
/// use error_forge::define_errors;
/// use std::path::PathBuf;
///
/// define_errors! {
///     pub enum StoreError {
///         /// The store file could not be read.
///         #[error(display = "cannot read {path:?}")]
///         #[kind(Io, retryable = true, status = 503)]
///         Read { path: PathBuf },
///
///         #[kind(Corrupt, fatal = true, exit = 3)]
///         Corrupt
///     }
/// }
///
/// let err = StoreError::read(PathBuf::from("db.bin"));
/// assert_eq!(err.to_string(), "cannot read \"db.bin\"");
/// assert!(err.is_retryable());
/// assert_eq!(StoreError::corrupt().exit_code(), 3);
/// assert_eq!(StoreError::corrupt().to_string(), "Corrupt: Corrupt");
/// ```
#[macro_export]
macro_rules! define_errors {
    // ------------------------------------------------------------------
    // Code generation from the normalised variant list.
    //
    // Each variant arrives as
    // `{ [attrs] [display] [kind, tags] Variant {fields}? }`.
    // ------------------------------------------------------------------
    (@emit [[$(#[$meta:meta])*] [$vis:vis] $name:ident]
        $({
            [$($vattr:tt)*]
            [$($disp:tt)*]
            [$kind:ident $(, $tag:ident = $val:expr)*]
            $variant:ident $({ $($field:ident : $ftype:ty),* })?
        })*
    ) => {
        $(#[$meta])* #[derive(::core::fmt::Debug)]
        #[cfg_attr(feature = "serde", derive(::serde::Serialize))]
        $vis enum $name {
            $( $($vattr)* $variant $( { $($field : $ftype),* } )?, )*
        }

        // Rejects misspelled `#[kind]` tags, which would otherwise be
        // ignored silently.
        const _: () = {
            $( $( $crate::define_errors!(@check_tag $tag); )* )*
        };

        impl $name {
            $(
                $crate::__private::pastey::paste! {
                    pub fn [<$variant:lower>]($($($field : $ftype),*)?) -> Self {
                        let instance = Self::$variant $( { $($field),* } )?;
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
                        $crate::define_errors!(@get_caption $kind $(, $tag = $val)*)
                    } ),*
                }
            }

            pub fn kind(&self) -> &'static str {
                match self {
                    $( Self::$variant { .. } => {
                        ::core::stringify!($kind)
                    } ),*
                }
            }

            pub fn is_retryable(&self) -> bool {
                match self {
                    $( Self::$variant { .. } => {
                        $crate::define_errors!(@get_tag retryable, false $(, $tag = $val)*)
                    } ),*
                }
            }

            pub fn is_fatal(&self) -> bool {
                match self {
                    $( Self::$variant { .. } => {
                        $crate::define_errors!(@get_tag fatal, false $(, $tag = $val)*)
                    } ),*
                }
            }

            pub fn status_code(&self) -> u16 {
                match self {
                    $( Self::$variant { .. } => {
                        $crate::define_errors!(@get_tag status, 500 $(, $tag = $val)*)
                    } ),*
                }
            }

            pub fn exit_code(&self) -> i32 {
                match self {
                    $( Self::$variant { .. } => {
                        $crate::define_errors!(@get_tag exit, 1 $(, $tag = $val)*)
                    } ),*
                }
            }
        }

        impl ::core::fmt::Display for $name {
            #[allow(unused_variables)]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $( Self::$variant $( { $($field),* } )? => {
                        $crate::define_errors!(
                            @display f [$($disp)*] self.caption(); $variant $($($field)*)?
                        )
                    } ),*
                }
            }
        }

        impl ::std::error::Error for $name {
            #[allow(unused_variables)]
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    $( Self::$variant $( { $($field),* } )? => {
                        $crate::define_errors!(@find_source $( $($field),* )? )
                    } ),*
                }
            }
        }
    };

    // `Display` body for one variant: the custom string when given ...
    (@display $f:ident [$display:literal $(, $param:ident)*] $caption:expr; $variant:ident $($field:ident)*) => {
        ::core::write!($f, $display $(, $param = $param)*)
    };
    // ... otherwise `<caption>: <Variant> | field = value ...`. The
    // default format is only generated for variants without a custom
    // string, so it is the only place the fields need `Debug`.
    (@display $f:ident [] $caption:expr; $variant:ident $($field:ident)*) => {{
        ::core::write!($f, "{}: ", $caption)?;
        $f.write_str(::core::stringify!($variant))?;
        $(
            ::core::write!($f, " | {} = ", ::core::stringify!($field))?;
            $crate::define_errors!(@fmt_field $f, $field, $field)?;
        )*
        ::core::result::Result::Ok(())
    }};
    (@display $($rest:tt)*) => {
        ::core::compile_error!(
            "define_errors!: a variant may carry only one #[error(display = \"...\")] attribute"
        )
    };

    // A field named `source` is shown with `Display`, every other field
    // with `Debug`. Matching the name here (rather than a runtime `match`
    // on `stringify!`) keeps the unused branch out of the generated code.
    (@fmt_field $f:ident, source, $field:ident) => {
        ::core::write!($f, "{}", $field)
    };
    (@fmt_field $f:ident, $name:ident, $field:ident) => {
        ::core::write!($f, "{:?}", $field)
    };

    (@check_tag caption) => {};
    (@check_tag retryable) => {};
    (@check_tag fatal) => {};
    (@check_tag status) => {};
    (@check_tag exit) => {};
    (@check_tag $other:ident) => {
        ::core::compile_error!(::core::concat!(
            "define_errors!: unknown #[kind] tag `",
            ::core::stringify!($other),
            "`; expected one of `caption`, `retryable`, `fatal`, `status`, `exit`"
        ));
    };

    // ------------------------------------------------------------------
    // General variant parser, used when the fast form below does not
    // match (attributes other than doc comments, doc comments placed
    // between `#[error]` and `#[kind]`, or malformed input that deserves
    // a precise error). It walks one attribute or variant per step, so
    // very large enums written this way may need a higher
    // `#![recursion_limit]`.
    //
    // State: `$hdr [done variants] [pending attrs] [display] [kind]`.
    // ------------------------------------------------------------------
    (@munch $hdr:tt [$($done:tt)*] [] [] []) => {
        $crate::define_errors!(@emit $hdr $($done)*);
    };
    (@munch $hdr:tt $done:tt [$($attrs:tt)*] [] $kind:tt
        #[error(display = $display:literal $(, $param:ident)* $(,)?)] $($rest:tt)*
    ) => {
        $crate::define_errors!(@munch $hdr $done [$($attrs)*] [$display $(, $param)*] $kind $($rest)*);
    };
    (@munch $hdr:tt $done:tt $attrs:tt $disp:tt $kind:tt #[error $($bad:tt)*] $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "define_errors!: expected a single `#[error(display = \"...\", field, ...)]` per variant, found `#[error",
            ::core::stringify!($($bad)*),
            "]`"
        ));
    };
    (@munch $hdr:tt $done:tt [$($attrs:tt)*] $disp:tt []
        #[kind($kind:ident $(, $tag:ident = $val:expr)* $(,)?)] $($rest:tt)*
    ) => {
        $crate::define_errors!(@munch $hdr $done [$($attrs)*] $disp [$kind $(, $tag = $val)*] $($rest)*);
    };
    (@munch $hdr:tt $done:tt $attrs:tt $disp:tt $kind:tt #[kind $($bad:tt)*] $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "define_errors!: expected a single `#[kind(Name, tag = value, ...)]` per variant, found `#[kind",
            ::core::stringify!($($bad)*),
            "]`"
        ));
    };
    (@munch $hdr:tt $done:tt [$($attrs:tt)*] $disp:tt $kind:tt #[$($attr:tt)*] $($rest:tt)*) => {
        $crate::define_errors!(@munch $hdr $done [$($attrs)* #[$($attr)*]] $disp $kind $($rest)*);
    };
    (@munch $hdr:tt [$($done:tt)*] [$($attrs:tt)*] [$($disp:tt)*] [$($kind:tt)+]
        $variant:ident { $($field:ident : $ftype:ty),* $(,)? } $(, $($rest:tt)*)?
    ) => {
        $crate::define_errors!(@munch $hdr
            [$($done)* { [$($attrs)*] [$($disp)*] [$($kind)+] $variant { $($field : $ftype),* } }]
            [] [] [] $($($rest)*)?);
    };
    (@munch $hdr:tt [$($done:tt)*] [$($attrs:tt)*] [$($disp:tt)*] [$($kind:tt)+]
        $variant:ident $(, $($rest:tt)*)?
    ) => {
        $crate::define_errors!(@munch $hdr
            [$($done)* { [$($attrs)*] [$($disp)*] [$($kind)+] $variant }]
            [] [] [] $($($rest)*)?);
    };
    (@munch $hdr:tt $done:tt $attrs:tt $disp:tt [] $variant:ident $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "define_errors!: variant `",
            ::core::stringify!($variant),
            "` needs a `#[kind(Name, ...)]` attribute"
        ));
    };
    (@munch $hdr:tt $done:tt $attrs:tt $disp:tt $kind:tt) => {
        ::core::compile_error!("define_errors!: attributes must be followed by a variant");
    };
    (@munch $hdr:tt $done:tt $attrs:tt $disp:tt $kind:tt $($rest:tt)+) => {
        ::core::compile_error!(::core::concat!(
            "define_errors!: expected a unit or struct-like variant (`Name` or `Name { field: Type }`), found `",
            ::core::stringify!($($rest)+),
            "`"
        ));
    };

    // ------------------------------------------------------------------
    // Helpers kept from 1.0.x.
    // ------------------------------------------------------------------
    (@find_source) => {
        ::core::option::Option::None
    };

    (@find_source $field:ident $(, $rest:ident)*) => {
        $crate::define_errors!(@find_source_match $field, $field $(, $rest)*)
    };

    (@find_source_match source, $source_field:ident $(, $rest:ident)*) => {
        $crate::macros::ErrorSource::as_source($source_field)
    };

    (@find_source_match $field_name:ident, $field:ident $(, $rest:ident)*) => {
        $crate::define_errors!(@find_source $($rest),*)
    };

    (@get_caption $kind:ident) => {
        ::core::stringify!($kind)
    };

    (@get_caption $kind:ident, caption = $caption:expr $(, $($rest:tt)*)?) => {
        $caption
    };

    (@get_caption $kind:ident, $tag:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::define_errors!(@get_caption $kind $(, $($rest)*)?)
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
        $crate::define_errors!(@get_tag $target, $default $(, $($rest)*)?)
    };

    (@format_display $display:literal) => {
        ::core::option::Option::Some(::std::format!($display))
    };

    (@format_display $display:literal, $($param:ident),+) => {
        ::core::option::Option::Some(::std::format!($display, $($param = $param),+))
    };

    (@format_display_field $field:ident) => {
        $field
    };

    (@format_display_field $field:ident . $($rest:ident).+) => {
        $field$(.$rest)+
    };

    // ------------------------------------------------------------------
    // Entry points.
    // ------------------------------------------------------------------

    // Fast form: optional doc comments, then `#[kind]` with an optional
    // `#[error]` before or after it. Matched without recursion, so enums
    // of any size work. Every input accepted by 1.0.1 takes this path.
    (
        $(
            $(#[$meta:meta])* $vis:vis enum $name:ident {
                $(
                    $(#[doc = $vdoc:expr])*
                    $(#[error(display = $display_a:literal $(, $param_a:ident)* $(,)?)])?
                    #[kind($kind:ident $(, $tag:ident = $val:expr)* $(,)?)]
                    $(#[error(display = $display_b:literal $(, $param_b:ident)* $(,)?)])?
                    $variant:ident $( { $($field:ident : $ftype:ty),* $(,)? } )?
                ),* $(,)?
            }
        )*
    ) => {
        $(
            $crate::define_errors!(@emit [[$(#[$meta])*] [$vis] $name]
                $({
                    [$(#[doc = $vdoc])*]
                    [
                        $($display_a $(, $param_a)*)?
                        $($display_b $(, $param_b)*)?
                    ]
                    [$kind $(, $tag = $val)*]
                    $variant $( { $($field : $ftype),* } )?
                })*
            );
        )*
    };

    // General form: any attributes in any order.
    (
        $(
            $(#[$meta:meta])* $vis:vis enum $name:ident { $($body:tt)* }
        )*
    ) => {
        $(
            $crate::define_errors!(@munch [[$(#[$meta])*] [$vis] $name] [] [] [] [] $($body)*);
        )*
    };
}
