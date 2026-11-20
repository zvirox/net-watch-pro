//! Shared server state: status broadcast, live-data feed, connection
//! tracking.
//!
//! ARCHITECTURE.md §6.2: the daemon publishes status and live data; the API
//! only serves it. No per-boot token (retired, D1a); authorization is via
//! socket peer-credential.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use netwatch_common::StatusSnapshot;
use sqlx::SqlitePool;
use tokio::sync::watch;

use crate::LiveFeed;
use crate::exports::ExportRegistry;
use crate::ratelimit::RateLimiter;
use netwatch_common::app_names::AppNames;

/// Whole-process state shared with all routes/extractors.
pub struct AppState {
    /// Latest status published by the daemon task.
    pub status: watch::Receiver<StatusSnapshot>,
    /// Sender used by license activation to trigger a status refresh (L2/L3).
    pub status_tx: watch::Sender<StatusSnapshot>,
    /// Live connection snapshot + delta feed (P2.6).
    pub live: LiveFeed,
    /// Cgroup-id -> app display name resolver (Phase C C4).
    pub app_names: AppNames,
    /// Read-side SQLite pool for query endpoints (history; API.md §2.2).
    /// The daemon owns the database file; this pool shares its WAL.
    pub db: SqlitePool,
    /// Connected WS clients (surfaced in `GET /status`).
    pub ws_clients: AtomicU64,
    /// Server start time, for `uptime_s` in `/status`.
    pub started_at: std::time::Instant,
    /// In-memory export jobs (API.md E2; process-global, no schema).
    pub exports: ExportRegistry,
    /// 5 exports created per minute (API.md §1.5).
    pub export_create_limiter: RateLimiter,
    /// 10 export downloads per minute (API.md §1.5).
    pub export_download_limiter: RateLimiter,
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
            app_names: AppNames::default(),
            db,
            ws_clients: AtomicU64::new(0),
            started_at: std::time::Instant::now(),
            exports: ExportRegistry::new(),
            export_create_limiter: RateLimiter::new(5, Duration::from_secs(60)),
            export_download_limiter: RateLimiter::new(10, Duration::from_secs(60)),
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
