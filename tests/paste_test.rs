//! The in-house identifier pasting behind `define_errors!` (which
//! replaced the `pastey` crate in 1.1.0) names constructors exactly as
//! `pastey` did: the variant name lowercased with `str::to_lowercase`,
//! a leading `r#` dropped.
#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]

use error_forge::define_errors;

define_errors! {
    pub enum NamingError {
        #[kind(Http)]
        HTTPError,

        #[kind(Snake)]
        Under_Score { code: u16 },

        #[kind(Raw)]
        r#Pending,

        #[kind(Unicode)]
        Ünicode,

        #[kind(Digits)]
        Retry2Later { attempts: u32 },
    }
}

#[test]
fn constructors_are_lowercased_variant_names() {
    assert_eq!(NamingError::httperror().kind(), "Http");
    assert_eq!(NamingError::under_score(7).kind(), "Snake");
    assert_eq!(NamingError::pending().kind(), "Raw");
    assert_eq!(NamingError::ünicode().kind(), "Unicode");
    assert_eq!(NamingError::retry2later(3).kind(), "Digits");
}

// The macro itself is internal (`error_forge::__private::paste!`); these
// cases pin the behaviour `define_errors!` relies on.
error_forge::__private::paste! {
    const [<Upper:upper>]: u8 = 1;
    const [<Mixed:lower _ Seg:upper>]: u8 = 2;
    const [<r#Raw:lower>]: u8 = 3;
    mod nested {
        pub fn [<Inner:lower>]() -> [u8; 2] {
            [1, 2]
        }
    }
}

macro_rules! via_fragment {
    ($name:ident, $ty:ty) => {
        error_forge::__private::paste! {
            fn [<$name:lower>]() -> $ty {
                ::core::default::Default::default()
            }
        }
    };
}

via_fragment!(FromFragment, Vec<[u8; 4]>);

#[test]
fn paste_handles_case_modifiers_segments_and_nesting() {
    assert_eq!(UPPER, 1);
    assert_eq!(mixed_SEG, 2);
    assert_eq!(raw, 3);
    assert_eq!(nested::inner(), [1, 2]);
    assert!(fromfragment().is_empty());
}
