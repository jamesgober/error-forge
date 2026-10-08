//! F-M2: the error hook is not re-entered, and a panicking hook does not
//! unwind through error constructors.
//!
//! The hook is process-global, so this file registers its own hook and
//! keeps every check in one test to avoid ordering between tests.

use error_forge::{macros::try_register_error_hook, AppError};
use std::sync::atomic::{AtomicUsize, Ordering};

static CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn hook_that_creates_errors_or_panics_is_contained() {
    try_register_error_hook(|ctx| {
        CALLS.fetch_add(1, Ordering::SeqCst);
        // A hook that creates an error (for example because its log sink
        // failed) used to recurse until the stack overflowed.
        let _ = AppError::other("log sink failed");
        if ctx.kind == "Network" {
            panic!("hook panicked");
        }
    })
    .expect("no other hook is registered in this test binary");

    let _ = AppError::config("x");
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        1,
        "nested error must not re-fire"
    );

    // The hook panics for network errors; the constructor still returns.
    let error = AppError::network("db", None);
    assert_eq!(error_forge::ForgeError::kind(&error), "Network");
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);

    // After a panic the re-entrancy flag is cleared, so the hook keeps
    // firing for later errors.
    let _ = AppError::config("y");
    assert_eq!(CALLS.load(Ordering::SeqCst), 3);

    // Other threads have their own flag.
    std::thread::spawn(|| {
        let _ = AppError::config("z");
    })
    .join()
    .unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 4);
}
