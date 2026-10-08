//! Compile-pass and compile-fail tests for `define_errors!`, `group!`
//! and `#[derive(ModError)]`.
//!
//! The expected compiler output in `tests/ui/fail/*.stderr` is recorded
//! with the stable toolchain CI uses. Regenerate it with
//! `TRYBUILD=overwrite cargo test --test ui_test` after a toolchain
//! change alters the diagnostics. The MSRV job skips this test.

#[test]
fn trybuild_ui_tests() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/define_errors_*.rs");
    cases.compile_fail("tests/ui/fail/define_errors_*.rs");
    if cfg!(feature = "derive") {
        cases.pass("tests/ui/pass/derive_*.rs");
        cases.compile_fail("tests/ui/fail/derive_*.rs");
    }
}
