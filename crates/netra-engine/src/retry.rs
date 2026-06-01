//! [`RetryPolicy`] — exponential-backoff retry for transient failures.

use std::time::Duration;

use netra_core::CoreError;

/// How a [`JobRunner`](crate::JobRunner) retries a failed attempt.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Total attempts, including the first. `1` disables retrying.
    pub max_attempts: u32,
    /// Backoff delay after the first failure; doubles each subsequent failure.
    pub base_delay: Duration,
    /// Upper bound on any single backoff delay.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// A policy that never retries.
    #[must_use]
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Self::default()
        }
    }

    /// The backoff delay before retry `attempt` (1-based: `delay_for(1)` is the
    /// wait after the first failure). Grows exponentially, capped at
    /// [`max_delay`](Self::max_delay).
    #[must_use]
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1);
        let multiplier = 2u32.saturating_pow(exponent);
        self.base_delay
            .saturating_mul(multiplier)
            .min(self.max_delay)
    }

    /// Whether `error` is a transient failure worth retrying.
    ///
    /// Only transport-level failures (a crashed or unreachable `pi` process)
    /// are retried; agent rejections, bad input, and cancellation are not.
    #[must_use]
    pub fn is_retryable(error: &CoreError) -> bool {
        matches!(error, CoreError::Transport(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_grows_exponentially() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
        };
        assert_eq!(policy.delay_for(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for(3), Duration::from_millis(400));
    }

    #[test]
    fn delay_is_capped_at_max() {
        let policy = RetryPolicy {
            max_attempts: 20,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        };
        assert_eq!(policy.delay_for(20), Duration::from_secs(10));
    }

    #[test]
    fn only_transport_errors_are_retryable() {
        assert!(RetryPolicy::is_retryable(&CoreError::Transport(
            "crash".into()
        )));
        assert!(!RetryPolicy::is_retryable(&CoreError::Agent(
            "rejected".into()
        )));
        assert!(!RetryPolicy::is_retryable(&CoreError::Cancelled));
        assert!(!RetryPolicy::is_retryable(&CoreError::Invalid(
            "bad".into()
        )));
    }
}
