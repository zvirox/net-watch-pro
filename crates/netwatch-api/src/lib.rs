//! API server: Unix-socket JSON routes, peer-cred auth, request/validation types.
//!
//! Binding contract: `../API.md` (§1–§3). This file mirrors it for implementation
//! reference; drift fails CI (P0.8 check).
//!
//! Transport: Unix domain socket (`NETWATCH_API_SOCKET`), peer-credential
//! (`SO_PEERCRED`) authorization against a root-owned allowed-UID list.
//! Loopback TCP + bearer token retired (D1a).

// Tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping in test
// assertions is standard and expected.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod app_names;
pub mod apps;
pub mod auth;
pub mod budgets;
pub mod calendar;
pub mod error;
pub mod exports;
pub mod history;
pub mod interfaces;
pub mod interfaces_usage;
pub mod license;
pub mod ratelimit;
pub mod routes;
pub mod settings;
pub mod state;
pub mod stream;
pub mod timeline;
pub mod usage;

use std::path::Path;
use std::sync::Arc;

use netwatch_common::{AppUsage, RateHistory, RateSnapshot, StatusSnapshot, UsageTotals};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch};

pub use state::AppState;

/// Receiving halves of the daemon's publishers (live, usage totals): the
/// current live snapshot plus deltas, and the latest headline totals.
/// Cloned freely; each WS connection derives its own delta receiver.
#[derive(Clone)]
pub struct LiveFeed {
    /// Latest live snapshot, replaced once per sample (≤ 1/s).
    pub snapshot: watch::Receiver<Arc<Vec<netwatch_common::LiveConnectionRow>>>,
    /// Delta frames broadcast to every subscribed client (API.md §3).
    pub deltas: broadcast::Sender<netwatch_common::LiveDelta>,
    /// Latest headline totals, replaced once per second (API.md §3).
    pub totals: watch::Receiver<Arc<UsageTotals>>,
    /// Latest live PC rates, replaced once per second (API.md §3 `net.rates`).
    pub rates: watch::Receiver<Arc<RateSnapshot>>,
    /// Latest per-app bytes/rates from the eBPF collector (API.md §3 `live.apps`).
    pub apps: watch::Receiver<Arc<Vec<AppUsage>>>,
    /// Bounded last-hour rate ring filled by the daemon's 1 Hz task (API.md
    /// U4, owner L-OQ1). Process memory only; lost on restart.
    pub rate_history: Arc<RateHistory>,
}

/// Serve the API on a Unix domain socket until `shutdown` resolves.
///
/// # Errors
///
/// - Refuses to bind if the socket path is invalid or already in use.
/// - Propagates serve-loop failures (axum server error).
///
/// Socket auth: peer-credential (`SO_PEERCRED`) check against a root-owned
/// allowed-UID list. One admin prompt at onboarding, none per launch (D1a).
pub async fn serve_unix(
    socket_path: &Path,
    api_gid: Option<u32>,
    status_tx: watch::Sender<StatusSnapshot>,
    live: LiveFeed,
    db: sqlx::SqlitePool,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), crate::error::ApiError> {
    // Remove stale socket file if present.
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path).map_err(|e| {
        crate::error::ApiError::new(
            netwatch_common::ErrorCode::PermissionDenied,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("cannot bind Unix socket {}: {e}", socket_path.display()),
        )
    })?;

    configure_socket_access(socket_path, api_gid)?;

    let status_rx = status_tx.subscribe();
    let state = Arc::new(AppState::new(status_tx, status_rx, live, db));
    let app = crate::routes::router(state).into_make_service();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| {
            crate::error::ApiError::new(
                netwatch_common::ErrorCode::DaemonUnreachable,
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("serve loop failed: {e}"),
            )
        })?;

    Ok(())
}

/// Restrict the API socket to root + the configured desktop group.
///
/// A Unix socket requires *write* permission to connect, so the default
/// `0755` from `bind()` is unusable by the unprivileged UI. `0660` with the
/// caller's primary group closes that gap until the peer-credential UID
/// allowlist (D1a) is implemented in full.
fn configure_socket_access(path: &Path, gid: Option<u32>) -> Result<(), crate::error::ApiError> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(gid) = gid {
        std::os::unix::fs::chown(path, None, Some(gid)).map_err(|error| {
            crate::error::ApiError::new(
                netwatch_common::ErrorCode::PermissionDenied,
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("cannot set socket group: {error}"),
            )
        })?;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660)).map_err(|error| {
        crate::error::ApiError::new(
            netwatch_common::ErrorCode::PermissionDenied,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("cannot set socket permissions: {error}"),
        )
    })
}
