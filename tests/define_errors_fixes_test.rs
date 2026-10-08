//! Regression tests for `define_errors!` fixes in 1.0.2.
#![deny(warnings)]

use error_forge::define_errors;
use std::path::PathBuf;

// F-H2: a custom display no longer requires `Display` or `Debug` on
// every field. Only the default format (no `#[error]`) formats fields.
define_errors! {
    pub enum PathError {
        #[error(display = "bad path {path:?}", path)]
        #[kind(Fs)]
        Bad { path: PathBuf },

        #[error(display = "{count} items rejected", count)]
        #[kind(Batch)]
        Batch { count: usize, items: Vec<String> },

        #[kind(Plain)]
        Plain { value: u32 },
    }
}

#[test]
fn custom_display_does_not_need_display_on_fields() {
    let err = PathError::bad(PathBuf::from("/x"));
    assert_eq!(err.to_string(), "bad path \"/x\"");

    let err = PathError::batch(2, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(err.to_string(), "2 items rejected");
}

#[test]
fn default_display_format_is_unchanged() {
    assert_eq!(PathError::plain(7).to_string(), "Plain: Plain | value = 7");
}

// Source types that do not implement `Display` (or `Serialize`, which
// the `serde` feature of this test crate would require).
#[cfg(not(feature = "serde"))]
mod optional_sources {
    use super::*;
    use std::error::Error;
    use std::io;

    define_errors! {
        pub enum SourceError {
            #[error(display = "io failed")]
            #[kind(Io, retryable = true)]
            Io { source: Option<io::Error> },

            #[error(display = "remote call to {endpoint} failed", endpoint)]
            #[kind(Remote, status = 502)]
            Remote {
                endpoint: String,
                source: Option<Box<dyn std::error::Error + Send + Sync>>,
            },

            #[kind(Direct)]
            Direct { source: io::Error },
        }
    }

    #[test]
    fn option_sources_work_with_custom_display() {
        let err = SourceError::io(None);
        assert_eq!(err.to_string(), "io failed");
        assert!(err.source().is_none());

        let err = SourceError::io(Some(io::Error::other("disk")));
        assert_eq!(err.source().unwrap().to_string(), "disk");

        let err = SourceError::remote("api".to_string(), Some("timeout".into()));
        assert_eq!(err.to_string(), "remote call to api failed");
        assert_eq!(err.source().unwrap().to_string(), "timeout");
        assert_eq!(err.status_code(), 502);
    }

    #[test]
    fn default_display_still_formats_source_with_display() {
        let err = SourceError::direct(io::Error::other("boom"));
        assert_eq!(err.to_string(), "Direct: Direct | source = boom");
    }
}

// F-L2: doc comments and other attributes on variants, either attribute
// order, and an optional trailing comma.
define_errors! {
    /// Enum-level docs.
    pub enum GrammarError {
        /// Doc comment above the attributes.
        #[error(display = "first {id}", id)]
        #[kind(First, status = 400)]
        First { id: u32 },

        #[kind(Second, retryable = true,)]
        #[error(display = "second")]
        Second,

        #[allow(dead_code)]
        #[kind(Third)]
        /// Doc comment between attributes.
        #[error(display = "third {name}", name)]
        Third { name: String, }
    }
}

#[test]
fn variant_docs_attributes_and_order_are_accepted() {
    assert_eq!(GrammarError::first(1).to_string(), "first 1");
    assert_eq!(GrammarError::first(1).status_code(), 400);
    assert_eq!(GrammarError::second().to_string(), "second");
    assert!(GrammarError::second().is_retryable());
    assert_eq!(GrammarError::third("x".to_string()).to_string(), "third x");
}

// F-L2: the display string is a `format!` string even without a field
// list, so escaped braces unescape and inline field names are captured.
define_errors! {
    pub enum FormatError {
        #[error(display = "Escaped {{braces}}")]
        #[kind(Escaped)]
        Escaped,

        #[error(display = "Config error: {message}")]
        #[kind(Config)]
        Config { message: String },

        #[error(display = "no placeholders here")]
        #[kind(Plain)]
        Plain { unused: u8 },
    }
}

#[test]
fn display_without_field_list_goes_through_format() {
    assert_eq!(FormatError::escaped().to_string(), "Escaped {braces}");
    assert_eq!(
        FormatError::config("boom".to_string()).to_string(),
        "Config error: boom"
    );
    assert_eq!(FormatError::plain(1).to_string(), "no placeholders here");
}

// Large enums in the 1.0.1 form must not hit the recursion limit.
define_errors! {
    pub enum Wide {
        #[error(display = "v0 {a}", a)] #[kind(K0, status = 418)] V0 { a: u8 },
        #[error(display = "v1 {a}", a)] #[kind(K1, status = 418)] V1 { a: u8 },
        #[error(display = "v2 {a}", a)] #[kind(K2, status = 418)] V2 { a: u8 },
        #[error(display = "v3 {a}", a)] #[kind(K3, status = 418)] V3 { a: u8 },
        #[error(display = "v4 {a}", a)] #[kind(K4, status = 418)] V4 { a: u8 },
        #[error(display = "v5 {a}", a)] #[kind(K5, status = 418)] V5 { a: u8 },
        #[error(display = "v6 {a}", a)] #[kind(K6, status = 418)] V6 { a: u8 },
        #[error(display = "v7 {a}", a)] #[kind(K7, status = 418)] V7 { a: u8 },
        #[error(display = "v8 {a}", a)] #[kind(K8, status = 418)] V8 { a: u8 },
        #[error(display = "v9 {a}", a)] #[kind(K9, status = 418)] V9 { a: u8 },
    }
}

#[test]
fn wide_enum_compiles_and_formats() {
    assert_eq!(Wide::v9(9).to_string(), "v9 9");
    assert_eq!(Wide::v0(0).kind(), "K0");
}
