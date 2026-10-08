//! Regression tests for `#[derive(ModError)]` fixes in 1.0.2.
#![cfg(feature = "derive")]
#![deny(warnings)]

use error_forge::{ForgeError, ModError};

// F-H3: explicit `false` on a flag attribute is honoured.
#[derive(Debug, ModError)]
pub enum FlagError {
    #[error_retryable(false)]
    #[error_fatal = false]
    ExplicitFalse,

    #[error_retryable = true]
    #[error_fatal(true)]
    ExplicitTrue,

    #[error_retryable]
    #[error_fatal]
    Bare,

    #[error_http_status = 404]
    #[error_exit_code(-2)]
    Codes,
}

#[test]
fn flag_attributes_honour_explicit_values() {
    assert!(!FlagError::ExplicitFalse.is_retryable());
    assert!(!FlagError::ExplicitFalse.is_fatal());
    assert!(FlagError::ExplicitTrue.is_retryable());
    assert!(FlagError::ExplicitTrue.is_fatal());
    assert!(FlagError::Bare.is_retryable());
    assert!(FlagError::Bare.is_fatal());
    assert_eq!(FlagError::Codes.status_code(), 404);
    assert_eq!(FlagError::Codes.exit_code(), -2);
}

// F-L3: unit variants go through the same formatting as the others.
#[derive(Debug, ModError)]
pub enum EscapeError {
    #[error_display("use {{x}} here")]
    Unit,
    #[error_display("use {{x}} here {0}")]
    Tuple(u8),
}

#[test]
fn unit_variant_display_unescapes_braces() {
    assert_eq!(EscapeError::Unit.to_string(), "use {x} here");
    assert_eq!(EscapeError::Tuple(1).to_string(), "use {x} here 1");
}

// F-L3: raw-identifier fields, by either spelling.
#[derive(Debug, ModError)]
pub enum RawError {
    #[error_display("type={type}")]
    Plain { r#type: String },
    #[error_display("type={r#type}")]
    Raw { r#type: String },
}

#[test]
fn raw_identifier_fields_compile_and_format() {
    let plain = RawError::Plain {
        r#type: "a".to_string(),
    };
    let raw = RawError::Raw {
        r#type: "b".to_string(),
    };
    assert_eq!(plain.to_string(), "type=a");
    assert_eq!(raw.to_string(), "type=b");
}

// F-L3: width and precision given by the caller apply to the message.
#[derive(Debug, ModError)]
pub enum WidthError {
    #[error_display("abc")]
    Short,
    #[error_display("conn {host}:{port}")]
    Conn { host: String, port: u16 },
    #[error_display("{value:>width$}")]
    Padded { value: u8, width: usize },
}

#[test]
fn width_and_precision_are_honoured() {
    assert_eq!(format!("[{:>10}]", WidthError::Short), "[       abc]");
    assert_eq!(format!("[{:<6.2}]", WidthError::Short), "[ab    ]");
    let conn = WidthError::Conn {
        host: "h".to_string(),
        port: 1,
    };
    assert_eq!(format!("{conn:-^12}"), "--conn h:1--");
    // Without a width the output is unchanged.
    assert_eq!(conn.to_string(), "conn h:1");
    // Width arguments inside the display string still work.
    let padded = WidthError::Padded { value: 7, width: 3 };
    assert_eq!(padded.to_string(), "  7");
}

#[derive(Debug, ModError)]
#[error_prefix("Cfg")]
pub struct StructError;

#[test]
fn struct_display_is_unchanged_and_honours_width() {
    assert_eq!(StructError.to_string(), "Cfg: Error");
    assert_eq!(format!("{StructError:>12}"), "  Cfg: Error");
    assert_eq!(StructError.kind(), "StructError");
    assert_eq!(StructError.caption(), "Cfg: Error");
}

// Fields the display string does not use are not bound, so they cause
// no unused-variable warnings (this file denies warnings).
#[derive(Debug, ModError)]
pub enum PartialError {
    #[error_display("only {used}")]
    Named { used: u8, unused: u8 },
    #[error_display("only {1}")]
    Tuple(u8, u8),
}

#[test]
fn unused_fields_are_ignored() {
    assert_eq!(
        PartialError::Named { used: 1, unused: 2 }.to_string(),
        "only 1"
    );
    assert_eq!(PartialError::Tuple(1, 2).to_string(), "only 2");
}
