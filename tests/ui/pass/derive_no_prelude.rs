// `#[derive(ModError)]` expands without relying on the prelude.
#![no_implicit_prelude]
#![deny(warnings)]

#[derive(Debug, ::error_forge::ModError)]
#[error_prefix("Db")]
pub enum DbError {
    #[error_display("connection to {0} failed")]
    #[error_retryable(false)]
    #[error_http_status = 503]
    Connection(::std::string::String),

    #[error_display("type {type} at {offset:>4}")]
    Typed { r#type: u8, offset: u32, unused: u8 },

    #[error_display("escaped {{x}}")]
    #[error_fatal]
    #[error_exit_code(-1)]
    Unit,
}

#[derive(Debug, ::error_forge::ModError)]
#[error_prefix = "S"]
pub struct StructError;

fn main() {
    use ::error_forge::ForgeError;
    let error = DbError::Connection(::std::string::ToString::to_string("primary"));
    ::std::assert_eq!(
        ::std::string::ToString::to_string(&error),
        "connection to primary failed"
    );
    ::std::assert!(!error.is_retryable());
    ::std::assert_eq!(error.status_code(), 503);
    let typed = DbError::Typed { r#type: 1, offset: 2, unused: 3 };
    ::std::assert_eq!(::std::string::ToString::to_string(&typed), "type 1 at    2");
    ::std::assert_eq!(::std::string::ToString::to_string(&DbError::Unit), "escaped {x}");
    ::std::assert_eq!(DbError::Unit.exit_code(), -1);
    ::std::assert_eq!(::std::format!("{:>6}", DbError::Unit), "escaped {x}");
    ::std::assert_eq!(::std::format!("{:>12}", StructError), "    S: Error");
}
