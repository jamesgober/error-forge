//! F-L1: the macros resolve every path they use absolutely, so user
//! items named like prelude or `std` items do not break them or change
//! their output.
#![allow(dead_code, non_camel_case_types, unused_macros, unused_imports)]

mod shadowed {
    // Items shadowing names the expansions used to reference bare.
    pub struct Option;
    pub struct Some;
    pub struct None;
    pub struct Result;
    pub struct String;
    pub struct Box;
    pub struct Ok;
    pub mod std {}
    pub mod core {}
    pub mod fmt {}
    macro_rules! format {
        ($($t:tt)*) => {
            42u8
        };
    }
    macro_rules! write {
        ($($t:tt)*) => {
            ::core::result::Result::Ok(())
        };
    }
    macro_rules! stringify {
        ($($t:tt)*) => {
            "shadowed"
        };
    }
    macro_rules! concat {
        ($($t:tt)*) => {
            "shadowed"
        };
    }

    use error_forge::{define_errors, group, AppError};

    define_errors! {
        pub enum DeclaredError {
            #[error(display = "declared {a}", a)]
            #[kind(Declared, status = 418)]
            Declared { a: u32 },

            #[error(display = "inline {b}")]
            #[kind(Inline)]
            Inline { b: u32 },

            #[kind(Default)]
            Default { c: u32 },

            #[kind(Unit)]
            Unit,
        }
    }

    group! {
        #[derive(Debug)]
        pub enum Grouped {
            App(AppError),
        }
    }

    pub fn declared() -> ::std::vec::Vec<::std::string::String> {
        ::std::vec![
            ::std::string::ToString::to_string(&DeclaredError::declared(1)),
            ::std::string::ToString::to_string(&DeclaredError::inline(2)),
            ::std::string::ToString::to_string(&DeclaredError::default(3)),
            ::std::string::ToString::to_string(&DeclaredError::unit()),
            ::std::string::ToString::to_string(DeclaredError::unit().kind()),
        ]
    }

    pub fn grouped() -> ::std::string::String {
        let error: Grouped = AppError::config("cfg").into();
        ::std::string::ToString::to_string(&error)
    }

    #[cfg(feature = "derive")]
    pub mod derived {
        pub struct Option;
        pub struct String;
        pub mod std {}
        macro_rules! format {
            ($($t:tt)*) => {
                42u8
            };
        }
        macro_rules! write {
            ($($t:tt)*) => {
                ::core::result::Result::Ok(())
            };
        }
        macro_rules! concat {
            ($($t:tt)*) => {
                "shadowed"
            };
        }

        use error_forge::ModError;

        #[derive(Debug, ModError)]
        #[error_prefix("Db")]
        pub enum DerivedError {
            #[error_display("x {0}")]
            A(u8),
            #[error_display("y {f}")]
            B {
                f: u8,
            },
            C,
        }

        #[derive(Debug, ModError)]
        #[error_prefix("S")]
        pub struct DerivedStruct;

        pub fn show() -> ::std::vec::Vec<::std::string::String> {
            ::std::vec![
                ::std::string::ToString::to_string(&DerivedError::A(7)),
                ::std::string::ToString::to_string(&DerivedError::B { f: 8 }),
                ::std::string::ToString::to_string(&DerivedError::C),
                ::std::string::ToString::to_string(&DerivedStruct),
                ::std::string::ToString::to_string(error_forge::ForgeError::caption(
                    &DerivedStruct,
                )),
            ]
        }
    }
}

#[test]
fn define_errors_ignores_shadowing_items() {
    assert_eq!(
        shadowed::declared(),
        vec![
            "declared 1",
            "inline 2",
            "Default: Default | c = 3",
            "Unit: Unit",
            "Unit",
        ]
    );
}

#[test]
fn group_ignores_shadowing_items() {
    assert_eq!(
        shadowed::grouped(),
        "\u{2699}\u{fe0f} Configuration Error: cfg"
    );
}

#[cfg(feature = "derive")]
#[test]
fn derive_ignores_shadowing_items() {
    assert_eq!(
        shadowed::derived::show(),
        vec!["x 7", "y 8", "C", "S: Error", "S: Error"]
    );
}
