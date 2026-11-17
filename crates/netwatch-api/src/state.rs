//! Shared server state: status broadcast, live-data feed, connection
//! tracking.
//!
//! ARCHITECTURE.md §6.2: the daemon publishes status and live data; the API
//! only serves it. No per-boot token (retired, D1a); authorization is via
//! socket peer-credential.

use std::sync::atomic::{AtomicU64, Ordering};

use netwatch_common::StatusSnapshot;
use sqlx::SqlitePool;
use tokio::sync::watch;

use crate::LiveFeed;

/// Whole-process state shared with all routes/extractors.
pub struct AppState {
    /// Latest status published by the daemon task.
    pub status: watch::Receiver<StatusSnapshot>,
    /// Sender used by license activation to trigger a status refresh (L2/L3).
    pub status_tx: watch::Sender<StatusSnapshot>,
    /// Live connection snapshot + delta feed (P2.6).
    pub live: LiveFeed,
    /// Read-side SQLite pool for query endpoints (history; API.md §2.2).
    /// The daemon owns the database file; this pool shares its WAL.
    pub db: SqlitePool,
    /// Connected WS clients (surfaced in `GET /status`).
    pub ws_clients: AtomicU64,
    /// Server start time, for `uptime_s` in `/status`.
    pub started_at: std::time::Instant,
}

impl AppState {
    /// Build state from the daemon's channels.
    #[must_use]
    pub fn new(
        status_tx: watch::Sender<StatusSnapshot>,
        status: watch::Receiver<StatusSnapshot>,
        live: LiveFeed,
        db: SqlitePool,
    ) -> Self {
        Self {
            status,
            status_tx,
            live,
            db,
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
