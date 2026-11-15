//! Shared server state: the per-boot token and the status broadcast.
//!
//! ARCHITECTURE.md §6.2: the daemon publishes status; the API only serves
//! it. The token is the single secret in the product (ARCHITECTURE.md §8.3).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use netwatch_common::StatusSnapshot;
use tokio::sync::watch;

/// Whole-process state shared with all routes/extractors.
pub struct AppState {
    /// Per-boot bearer token (plaintext in memory; on disk it is 0600).
    pub token: Arc<String>,
    /// Latest status published by the daemon task.
    pub status: watch::Receiver<StatusSnapshot>,
    /// Connected WS clients (surfaced in `GET /status`).
    pub ws_clients: AtomicU64,
    /// Server start time, for `uptime_s` in `/status`.
    pub started_at: std::time::Instant,
}

impl AppState {
    /// Build state from a token and the daemon's status channel.
    #[must_use]
    pub fn new(token: Arc<String>, status: watch::Receiver<StatusSnapshot>) -> Self {
        Self {
            token,
            status,
            ws_clients: AtomicU64::new(0),
            started_at: std::time::Instant::now(),
        }
    }

    /// Bump the WS client counter (called on connect/disconnect).
    pub fn client_delta(&self, delta: i64) {
        let magnitude = delta.unsigned_abs();
        if delta >= 0 {
            self.ws_clients.fetch_add(magnitude, Ordering::Relaxed);
        } else {
            self.ws_clients.fetch_sub(magnitude, Ordering::Relaxed);
        }
    }

    /// Current WS client count.
    #[must_use]
    pub fn client_count(&self) -> u64 {
        self.ws_clients.load(Ordering::Relaxed)
    }
}
