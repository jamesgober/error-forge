//! Regression tests for the 1.2.0 review fixes. Each fails against 1.1.0.

use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use error_forge::recovery::{
    CircuitBreaker, CircuitBreakerConfig, CircuitOpenError, CircuitState, ExponentialBackoff,
    RetryExecutor, RetryPolicy,
};
use error_forge::{AppError, ForgeError, Redacted, ResultExt};

/// A probe that never returns must not keep the circuit failing every
/// call: once it is older than the probe timeout it counts as failed, and
/// a new probe is admitted after the next reset timeout.
#[test]
fn a_hung_probe_times_out_instead_of_locking_the_breaker() {
    let config = CircuitBreakerConfig::default()
        .with_failure_threshold(1)
        .with_reset_timeout_ms(20)
        .with_probe_timeout_ms(50);
    let breaker = Arc::new(CircuitBreaker::with_config("db", config));
    let _ = breaker.execute(|| Err::<(), _>(std::io::Error::other("down")));
    thread::sleep(Duration::from_millis(30));

    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let hung = Arc::clone(&breaker);
    let probe = thread::spawn(move || {
        hung.execute(|| {
            started_tx.send(()).unwrap();
            // A probe blocked on a socket with no timeout.
            release_rx.recv().unwrap();
            Ok::<(), std::io::Error>(())
        })
    });
    started_rx.recv().unwrap();
    assert!(breaker.is_probing());
    let early = breaker.execute(|| Ok::<u32, std::io::Error>(1));
    assert!(early.unwrap_err().is::<CircuitOpenError>());

    // Past the probe timeout plus a reset timeout: a new probe is admitted
    // and, succeeding, closes the circuit while the old one still hangs.
    thread::sleep(Duration::from_millis(120));
    assert!(!breaker.is_probing());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    let recovered = breaker.execute(|| Ok::<u32, std::io::Error>(2));
    assert_eq!(recovered.unwrap(), 2);
    assert_eq!(breaker.state(), CircuitState::Closed);

    // The hung probe's late result changes nothing.
    release_tx.send(()).unwrap();
    probe.join().unwrap().unwrap();
    assert_eq!(breaker.state(), CircuitState::Closed);
}

/// A timed-out probe reads as open (a failed probe) until the next reset
/// timeout, not as half-open.
#[test]
fn a_timed_out_probe_reads_as_open() {
    let config = CircuitBreakerConfig::default()
        .with_failure_threshold(1)
        .with_reset_timeout_ms(200)
        .with_probe_timeout_ms(20);
    let breaker = Arc::new(CircuitBreaker::with_config("cache", config));
    let _ = breaker.execute(|| Err::<(), _>(std::io::Error::other("down")));
    thread::sleep(Duration::from_millis(210));

    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let hung = Arc::clone(&breaker);
    let probe = thread::spawn(move || {
        hung.execute(|| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok::<(), std::io::Error>(())
        })
    });
    started_rx.recv().unwrap();
    thread::sleep(Duration::from_millis(40));
    assert_eq!(breaker.state(), CircuitState::Open);
    release_tx.send(()).unwrap();
    probe.join().unwrap().unwrap();
    assert_eq!(breaker.state(), CircuitState::Open);
}

/// The retry executor takes any backoff, and jitter reaches it.
#[test]
fn retry_executor_uses_a_custom_backoff_and_jitter() {
    let executor = RetryExecutor::<std::io::Error>::new_exponential()
        .with_max_retries(3)
        .with_backoff(
            ExponentialBackoff::new()
                .with_initial_delay(1)
                .with_max_delay(4),
        );
    let mut delays = Vec::new();
    let _ = executor.retry_with_handler(
        || Err::<(), _>(std::io::Error::other("x")),
        |_, _, delay| delays.push(delay),
    );
    assert_eq!(
        delays,
        vec![
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(4)
        ]
    );

    // With jitter on, schedules differ from run to run.
    let mut schedules = HashSet::new();
    for _ in 0..20 {
        let executor = RetryPolicy::new_fixed(0)
            .executor::<std::io::Error>()
            .with_max_retries(4)
            .with_backoff(
                ExponentialBackoff::new()
                    .with_initial_delay(10)
                    .with_max_delay(40)
                    .with_jitter(true),
            );
        let mut delays = Vec::new();
        let start = Instant::now();
        let _ = executor.retry_with_handler(
            || Err::<(), _>(std::io::Error::other("x")),
            |_, _, delay| delays.push(delay),
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        let _ = schedules.insert(delays);
    }
    assert!(schedules.len() > 1, "every jittered schedule was identical");

    // `RetryPolicy::with_jitter` reaches the executor it builds.
    let policy = RetryPolicy::new_exponential()
        .with_jitter(true)
        .with_max_retries(1);
    let mut first = Vec::new();
    let _ = policy.executor::<std::io::Error>().retry_with_handler(
        || Err::<(), _>(std::io::Error::other("x")),
        |_, _, delay| first.push(delay),
    );
    assert_eq!(first.len(), 1);
    assert!(first[0] >= Duration::from_millis(80) && first[0] <= Duration::from_millis(120));
}

/// `user_message`, documented as safe for end users, must not carry the
/// error's data.
#[test]
fn user_message_carries_no_error_data() {
    let fs = AppError::filesystem_with_source(
        "/home/alice/.config/app/secrets.toml",
        std::io::Error::other("permission denied"),
    );
    let net = AppError::network("https://svc:hunter2@db.internal:5432/prod", None);
    let config = AppError::config("token=abc123 rejected");
    for error in [&fs, &net, &config] {
        let shown = error.user_message();
        for secret in ["alice", "secrets.toml", "hunter2", "db.internal", "abc123"] {
            assert!(
                !shown.contains(secret),
                "user_message {shown:?} leaks {secret}"
            );
        }
        assert_eq!(shown, error.caption());
    }
    // Developers still get the detail.
    assert!(net.dev_message().contains("db.internal"));

    // A wrapper's context stays out of the user message as well.
    let wrapped: Result<(), AppError> = Err(AppError::config("bad port"));
    let wrapped = wrapped.context("loading /etc/app/secret.conf").unwrap_err();
    assert!(!wrapped.user_message().contains("secret.conf"));
}

/// A `Redacted` field never prints its value.
#[test]
fn redacted_values_stay_out_of_error_text() {
    #[derive(Debug)]
    struct Login {
        user: String,
        password: Redacted<String>,
    }
    let login = Login {
        user: "alice".into(),
        password: Redacted::new("hunter2".into()),
    };
    let text = format!("{login:?}");
    assert_eq!(login.user, "alice");
    assert!(text.contains("alice"));
    assert!(!text.contains("hunter2"));
    assert_eq!(login.password.expose(), "hunter2");
}

/// With a large threshold the breaker keeps at most `failure_threshold`
/// failure times, so each failure costs the same however many came before.
#[test]
fn failure_tracking_stays_linear_under_a_large_threshold() {
    fn run(failures: usize) -> Duration {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(usize::MAX)
            .with_failure_window_ms(3_600_000);
        let breaker = CircuitBreaker::with_config("bulk", config);
        let start = Instant::now();
        for _ in 0..failures {
            let _ = breaker.execute(|| Err::<(), _>(std::io::Error::other("x")));
        }
        start.elapsed()
    }
    let small = run(10_000);
    let large = run(40_000);
    // Quadratic growth (1.1.0) made 4x the failures cost about 16x the
    // time; linear growth costs about 4x.
    assert!(
        large < small * 10,
        "40k failures took {large:?}, 10k took {small:?}"
    );
}
