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

pub mod auth;
pub mod calendar;
pub mod error;
pub mod history;
pub mod interfaces_usage;
pub mod routes;
pub mod state;
pub mod stream;
pub mod timeline;
pub mod usage;

use std::path::Path;
use std::sync::Arc;

use netwatch_common::StatusSnapshot;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch};

pub use state::AppState;

/// Receiving halves of the daemon's live-data publishers
/// (`netwatchd::live`): the current snapshot plus a broadcast of deltas.
/// Cloned freely; each WS connection derives its own delta receiver.
#[derive(Clone)]
pub struct LiveFeed {
    /// Latest live snapshot, replaced once per sample (≤ 1/s).
    pub snapshot: watch::Receiver<Arc<Vec<netwatch_common::LiveConnectionRow>>>,
    /// Delta frames broadcast to every subscribed client (API.md §3).
    pub deltas: broadcast::Sender<netwatch_common::LiveDelta>,
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
    status: watch::Receiver<StatusSnapshot>,
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
            format!("cannot bind Unix socket {socket_path:?}: {e}"),
        )
    })?;

    let state = Arc::new(AppState::new(status, live, db));
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
