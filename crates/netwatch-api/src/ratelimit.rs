//! Minimal fixed-window rate limiter for API.md §1.5.
//!
//! The API serves one trusted local client over a Unix socket, so limits exist
//! mainly to stop a runaway UI loop rather than to defend against an attacker.
//! A process-global fixed window is therefore sufficient and keeps state tiny;
//! each limiter instance is one (limit, window) pair stored in `AppState`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A fixed-window counter. `try_acquire` consumes one slot or returns the
/// number of seconds until the window resets.
#[derive(Debug)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    state: Mutex<Window>,
}

#[derive(Debug)]
struct Window {
    started_at: Instant,
    used: u32,
}

impl RateLimiter {
    /// Build a limiter allowing `limit` acquisitions per `window`.
    #[must_use]
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            state: Mutex::new(Window {
                started_at: Instant::now(),
                used: 0,
            }),
        }
    }

    /// Consume one slot.
    ///
    /// # Errors
    ///
    /// Returns `Err(retry_after_s)` when the window is exhausted. A poisoned
    /// mutex is recovered rather than propagated (ARCHITECTURE.md §7.1).
    pub fn try_acquire(&self) -> Result<(), u64> {
        // Recover from poisoning: a panic elsewhere must not wedge the API.
        let mut window = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(window.started_at);
        if elapsed >= self.window {
            window.started_at = now;
            window.used = 0;
        }
        if window.used >= self.limit {
            let remaining = self
                .window
                .saturating_sub(now.saturating_duration_since(window.started_at));
            return Err(remaining.as_secs().max(1));
        }
        window.used += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    #[test]
    fn limit_then_reset() {
        let limiter = RateLimiter::new(2, Duration::from_millis(40));
        assert!(limiter.try_acquire().is_ok());
        assert!(limiter.try_acquire().is_ok());
        let retry = limiter.try_acquire().unwrap_err();
        assert!(retry >= 1);
        std::thread::sleep(Duration::from_millis(50));
        assert!(limiter.try_acquire().is_ok());
    }
}
