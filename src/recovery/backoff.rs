use std::cell::Cell;
use std::cmp::min;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

/// A uniform value in `[0, 1)` from a per-thread xorshift64* generator.
///
/// Delays only need to differ between processes and threads, not to be
/// unpredictable, so no cryptographic source is needed. Each thread seeds
/// its generator once from the standard library's randomly keyed hasher
/// (seeded from the operating system once per process), so two processes
/// or threads do not share a schedule.
fn unit_random() -> f64 {
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|state| {
        let mut x = state.get();
        if x == 0 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_usize(state as *const Cell<u64> as usize);
            x = hasher.finish() | 1;
        }
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        state.set(x);
        // The top 53 bits of the scrambled value, as a fraction.
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// Trait for backoff strategies used in retry mechanisms
pub trait Backoff: Send + Sync + 'static {
    /// Get the next delay duration based on the current attempt
    fn next_delay(&self, attempt: usize) -> Duration;

    /// Reset the backoff state
    fn reset(&mut self) {}

    /// Create a clone of this backoff strategy
    fn box_clone(&self) -> Box<dyn Backoff>;
}

/// Exponential backoff strategy with optional jitter
///
/// This strategy increases the delay exponentially with each attempt,
/// and can add random jitter to prevent multiple retries from synchronizing.
#[derive(Clone)]
pub struct ExponentialBackoff {
    initial_delay_ms: u64,
    max_delay_ms: u64,
    factor: f64,
    jitter: bool,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff with default settings
    ///
    /// Defaults:
    /// - Initial delay: 100ms
    /// - Max delay: 30000ms (30 seconds)
    /// - Factor: 2.0 (doubles with each attempt)
    /// - Jitter: false
    pub fn new() -> Self {
        Self {
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            factor: 2.0,
            jitter: false,
        }
    }

    /// Set the initial delay in milliseconds
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Set the maximum delay in milliseconds
    pub fn with_max_delay(mut self, delay_ms: u64) -> Self {
        self.max_delay_ms = delay_ms;
        self
    }

    /// Set the multiplication factor for each attempt
    pub fn with_factor(mut self, factor: f64) -> Self {
        self.factor = factor;
        self
    }

    /// Enable or disable ±20% jitter on each calculated delay.
    ///
    /// Jitter applies to every attempt, including the first. Each delay
    /// is drawn uniformly from 80% to 120% of the calculated delay, and
    /// never above the maximum delay: once the calculated delay nears the
    /// maximum, the upper end of the range is the maximum itself, so
    /// delays at the cap stay spread out instead of landing on it.
    ///
    /// Off by default. Since 1.2.0 jitter works with or without the
    /// `jitter` cargo feature (it no longer needs `rand`); the feature
    /// now turns jitter on by default for the exponential
    /// [`RetryExecutor`](crate::recovery::RetryExecutor) and
    /// [`RetryPolicy`](crate::recovery::RetryPolicy).
    ///
    /// # Example
    ///
    /// ```
    /// use error_forge::recovery::{Backoff, ExponentialBackoff};
    /// use std::time::Duration;
    ///
    /// let backoff = ExponentialBackoff::new().with_jitter(true);
    /// let delay = backoff.next_delay(0); // around the 100 ms initial delay
    /// assert!(delay >= Duration::from_millis(80) && delay <= Duration::from_millis(120));
    /// ```
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }
}

impl Backoff for ExponentialBackoff {
    fn next_delay(&self, attempt: usize) -> Duration {
        // Attempt 0 is the initial delay, kept in integer arithmetic so
        // large values do not lose precision. Like every other attempt
        // it is capped at `max_delay_ms`.
        let calculated_delay = if attempt == 0 {
            self.initial_delay_ms
        } else {
            // Attempts beyond `i32::MAX` clamp instead of wrapping to a
            // negative exponent. The float-to-int cast saturates, so an
            // infinite product lands on the cap.
            let exponent = i32::try_from(attempt).unwrap_or(i32::MAX);
            let exp_factor = self.factor.powi(exponent);
            (self.initial_delay_ms as f64 * exp_factor) as u64
        };
        let capped_delay = min(calculated_delay, self.max_delay_ms);

        if self.jitter {
            // Uniform over [0.8 x delay, min(1.2 x delay, max)]: below the
            // cap rather than clamped onto it, so delays at the cap do not
            // pile up on one instant.
            let low = capped_delay as f64 * 0.8;
            let high = (capped_delay as f64 * 1.2).min(self.max_delay_ms as f64);
            let jittered = low + (high - low).max(0.0) * unit_random();
            return Duration::from_millis(min(jittered as u64, self.max_delay_ms));
        }

        Duration::from_millis(capped_delay)
    }

    fn box_clone(&self) -> Box<dyn Backoff> {
        Box::new(self.clone())
    }
}

/// Linear backoff strategy
///
/// Increases delay linearly by adding a fixed increment with each attempt.
#[derive(Clone)]
pub struct LinearBackoff {
    initial_delay_ms: u64,
    increment_ms: u64,
    max_delay_ms: u64,
}

impl LinearBackoff {
    /// Create a new linear backoff with default settings
    ///
    /// Defaults:
    /// - Initial delay: 100ms
    /// - Increment: 100ms (adds 100ms per attempt)
    /// - Max delay: 10000ms (10 seconds)
    pub fn new() -> Self {
        Self {
            initial_delay_ms: 100,
            increment_ms: 100,
            max_delay_ms: 10000,
        }
    }

    /// Set the initial delay in milliseconds
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Set the increment in milliseconds
    pub fn with_increment(mut self, increment_ms: u64) -> Self {
        self.increment_ms = increment_ms;
        self
    }

    /// Set the maximum delay in milliseconds
    pub fn with_max_delay(mut self, delay_ms: u64) -> Self {
        self.max_delay_ms = delay_ms;
        self
    }
}

impl Backoff for LinearBackoff {
    fn next_delay(&self, attempt: usize) -> Duration {
        // Saturate so large attempts or increments hit the cap instead
        // of overflowing (a panic in debug builds, a wrap in release).
        let attempt = u64::try_from(attempt).unwrap_or(u64::MAX);
        let delay_ms = self
            .initial_delay_ms
            .saturating_add(attempt.saturating_mul(self.increment_ms));
        let capped_delay = min(delay_ms, self.max_delay_ms);
        Duration::from_millis(capped_delay)
    }

    fn box_clone(&self) -> Box<dyn Backoff> {
        Box::new(self.clone())
    }
}

/// Fixed backoff strategy
///
/// Uses the same delay for all retry attempts.
#[derive(Clone)]
pub struct FixedBackoff {
    delay_ms: u64,
}

impl FixedBackoff {
    /// Create a new fixed backoff with the given delay
    pub fn new(delay_ms: u64) -> Self {
        Self { delay_ms }
    }
}

impl Backoff for FixedBackoff {
    fn next_delay(&self, _attempt: usize) -> Duration {
        Duration::from_millis(self.delay_ms)
    }

    fn box_clone(&self) -> Box<dyn Backoff> {
        Box::new(self.clone())
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LinearBackoff {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Backoff for Box<dyn Backoff> to enable boxed trait objects
impl Backoff for Box<dyn Backoff> {
    fn next_delay(&self, attempt: usize) -> Duration {
        (**self).next_delay(attempt)
    }

    fn reset(&mut self) {
        (**self).reset()
    }

    fn box_clone(&self) -> Box<dyn Backoff> {
        (**self).box_clone()
    }
}

#[cfg(test)]
mod jitter_tests {
    use super::*;

    #[test]
    fn jitter_spreads_below_the_cap_instead_of_piling_on_it() {
        let backoff = ExponentialBackoff::new()
            .with_max_delay(30_000)
            .with_jitter(true);
        let draws: Vec<Duration> = (0..10_000).map(|_| backoff.next_delay(20)).collect();
        let at_cap = draws
            .iter()
            .filter(|d| **d == Duration::from_millis(30_000))
            .count();
        // 1.1.0 put about half of all draws exactly on the cap.
        assert!(at_cap < 100, "{at_cap} of 10000 draws exactly at the cap");
        assert!(draws
            .iter()
            .all(|d| *d >= Duration::from_millis(24_000) && *d <= Duration::from_millis(30_000)));
    }

    #[test]
    fn jitter_varies_between_draws_and_stays_in_band() {
        let backoff = ExponentialBackoff::new().with_jitter(true);
        let draws: std::collections::HashSet<Duration> =
            (0..200).map(|_| backoff.next_delay(1)).collect();
        assert!(
            draws.len() > 10,
            "jitter produced {} distinct delays",
            draws.len()
        );
        for d in draws {
            assert!(d >= Duration::from_millis(160) && d <= Duration::from_millis(240));
        }
    }

    #[test]
    fn unit_random_is_in_range() {
        for _ in 0..10_000 {
            let x = unit_random();
            assert!((0.0..1.0).contains(&x));
        }
    }
}
