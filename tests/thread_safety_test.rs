//! Multi-threaded tests for the shared, process-wide pieces of the
//! crate: the error hook, the error-code registry, collectors and the
//! circuit breaker.

use error_forge::recovery::{CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState};
use error_forge::{
    define_errors, macros::try_register_error_hook, register_error_code, AppError, ErrorCollector,
    ErrorRegistry, ForgeError,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const THREADS: usize = 8;

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn shared_types_are_send_and_sync() {
    assert_send_sync::<AppError>();
    assert_send_sync::<ErrorCollector<AppError>>();
    assert_send_sync::<ErrorRegistry>();
    assert_send_sync::<CircuitBreaker>();
    assert_send_sync::<CircuitOpenError>();
}

define_errors! {
    pub enum ThreadProbe {
        #[error(display = "probe {id}", id)]
        #[kind(ThreadProbe)]
        Probe { id: usize },
    }
}

/// Counts hook calls for `ThreadProbe` errors only, so errors created
/// by other tests running in parallel do not affect the count.
static PROBE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn hook_fires_once_per_error_from_every_thread() {
    try_register_error_hook(|ctx| {
        if ctx.kind == "ThreadProbe" {
            PROBE_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    })
    .expect("only this test registers a hook in this binary");

    const PER_THREAD: usize = 250;
    let barrier = Arc::new(Barrier::new(THREADS));
    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for i in 0..PER_THREAD {
                    let error = ThreadProbe::probe(t * PER_THREAD + i);
                    assert_eq!(error.kind(), "ThreadProbe");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(
        PROBE_HOOK_CALLS.load(Ordering::SeqCst),
        THREADS * PER_THREAD
    );
}

#[test]
fn registry_accepts_each_code_exactly_once_under_contention() {
    let barrier = Arc::new(Barrier::new(THREADS));
    let accepted = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let barrier = Arc::clone(&barrier);
            let accepted = Arc::clone(&accepted);
            thread::spawn(move || {
                barrier.wait();
                for code in 0..50 {
                    // Every thread races to register the same 50 codes.
                    if register_error_code(
                        format!("THREAD-{code:03}"),
                        format!("registered by thread {t}"),
                        None::<String>,
                        code % 2 == 0,
                    )
                    .is_ok()
                    {
                        accepted.fetch_add(1, Ordering::SeqCst);
                    }
                    // Concurrent reads see a complete entry.
                    let info = ErrorRegistry::global()
                        .get_code_info(&format!("THREAD-{code:03}"))
                        .expect("registered by this thread or another");
                    assert_eq!(info.retryable, code % 2 == 0);
                    let coded = AppError::other("x").with_code(format!("THREAD-{code:03}"));
                    assert_eq!(coded.is_retryable(), code % 2 == 0);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(accepted.load(Ordering::SeqCst), 50);
}

#[test]
fn collectors_merge_errors_from_worker_threads() {
    let shared = Arc::new(Mutex::new(ErrorCollector::new()));
    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                // Each worker collects locally, then merges once.
                let mut local = ErrorCollector::default();
                for i in 0..10 {
                    local.push(AppError::other(format!("worker {t} error {i}")));
                }
                shared.lock().unwrap().extend(local);
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let collector = Arc::try_unwrap(shared).unwrap().into_inner().unwrap();
    assert_eq!(collector.len(), THREADS * 10);
    assert!(!collector.has_fatal());
}

#[test]
fn circuit_breaker_trips_once_under_concurrent_failures() {
    let breaker = Arc::new(CircuitBreaker::with_config(
        "concurrent",
        CircuitBreakerConfig::new(THREADS, 60_000, 60_000),
    ));
    let barrier = Arc::new(Barrier::new(THREADS));
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let breaker = Arc::clone(&breaker);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                breaker.execute(|| Err::<(), _>(std::io::Error::other("down")))
            })
        })
        .collect();
    for handle in handles {
        let error = handle.join().unwrap().unwrap_err();
        // Every call was admitted while closed and ran.
        assert!(error.downcast_ref::<std::io::Error>().is_some());
    }
    assert_eq!(breaker.state(), CircuitState::Open);

    // While open, every concurrent caller fails fast without running.
    let ran = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let breaker = Arc::clone(&breaker);
            let ran = Arc::clone(&ran);
            thread::spawn(move || {
                breaker
                    .execute(|| {
                        ran.fetch_add(1, Ordering::SeqCst);
                        Ok::<(), std::io::Error>(())
                    })
                    .unwrap_err()
                    .is::<CircuitOpenError>()
            })
        })
        .collect();
    for handle in handles {
        assert!(handle.join().unwrap());
    }
    assert_eq!(ran.load(Ordering::SeqCst), 0);
}

#[test]
fn circuit_breaker_admits_one_probe_under_contention() {
    let reset_timeout = Duration::from_millis(50);
    let breaker = Arc::new(CircuitBreaker::with_config(
        "probe-race",
        CircuitBreakerConfig::new(1, 60_000, 50),
    ));
    let _ = breaker.execute(|| Err::<(), _>(std::io::Error::other("trip")));
    thread::sleep(reset_timeout + Duration::from_millis(30));
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    let admitted = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(THREADS));
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let breaker = Arc::clone(&breaker);
            let admitted = Arc::clone(&admitted);
            let rejected = Arc::clone(&rejected);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let result = breaker.execute(|| {
                    admitted.fetch_add(1, Ordering::SeqCst);
                    // Hold the probe until every other caller has been
                    // turned away (bounded so a bug cannot hang the test).
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while rejected.load(Ordering::SeqCst) < THREADS - 1 && Instant::now() < deadline
                    {
                        thread::yield_now();
                    }
                    Ok::<(), std::io::Error>(())
                });
                if let Err(error) = result {
                    assert!(error.is::<CircuitOpenError>());
                    rejected.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(admitted.load(Ordering::SeqCst), 1);
    assert_eq!(rejected.load(Ordering::SeqCst), THREADS - 1);
    assert_eq!(breaker.state(), CircuitState::Closed);
}
