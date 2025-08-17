use std::time::Duration;
use std::cmp::min;
use rand::Rng;

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
    
    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }
}

impl Backoff for ExponentialBackoff {
    fn next_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.initial_delay_ms);
        }
        
        // Calculate exponential delay
        let exp_factor = self.factor.powi(attempt as i32);
        let calculated_delay = (self.initial_delay_ms as f64 * exp_factor) as u64;
        let capped_delay = min(calculated_delay, self.max_delay_ms);
        
        if self.jitter {
            // Apply jitter (±20%)
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.8..1.2);
            let jittered_delay = (capped_delay as f64 * jitter_factor) as u64;
            Duration::from_millis(jittered_delay)
        } else {
            Duration::from_millis(capped_delay)
        }
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
        let delay_ms = self.initial_delay_ms + (attempt as u64 * self.increment_ms);
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
