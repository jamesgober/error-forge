// `define_errors!` and `group!` expand without relying on the prelude.
#![no_implicit_prelude]
#![deny(warnings)]

::error_forge::define_errors! {
    /// Documented enum.
    pub enum StoreError {
        /// Documented variant, `#[error]` after `#[kind]`.
        #[kind(Read, retryable = true, status = 503)]
        #[error(display = "cannot read {path:?}", path)]
        Read { path: ::std::path::PathBuf, attempts: u32 },

        #[error(display = "literal {{braces}}")]
        #[kind(Braces)]
        Braces,

        #[allow(dead_code)]
        #[kind(Plain)]
        Plain { value: u8 }
    }
}

::error_forge::group! {
    #[derive(Debug)]
    pub enum AnyError {
        App(::error_forge::AppError),
    }
}

fn main() {
    let error = StoreError::read(::std::convert::From::from("db"), 3);
    ::std::assert_eq!(::std::string::ToString::to_string(&error), "cannot read \"db\"");
    ::std::assert!(error.is_retryable());
    ::std::assert_eq!(
        ::std::string::ToString::to_string(&StoreError::braces()),
        "literal {braces}"
    );
    ::std::assert_eq!(
        ::std::string::ToString::to_string(&StoreError::plain(1)),
        "Plain: Plain | value = 1"
    );
    let grouped: AnyError = ::std::convert::From::from(::error_forge::AppError::other("x"));
    ::std::assert_eq!(::error_forge::ForgeError::kind(&grouped), "Other");
}
