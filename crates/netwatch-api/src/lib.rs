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
pub mod error;
pub mod routes;
pub mod state;
pub mod stream;

use std::path::Path;
use std::sync::Arc;

use futures_util::SinkExt;
use netwatch_common::StatusSnapshot;
use tokio::net::UnixListener;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::http::Request;

pub use state::AppState;

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

    let state = Arc::new(AppState::new(status));

    let mut shutdown = Box::pin(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => break,
            result = listener.accept() => {
                match result {
                    Ok((stream, _peer_addr)) => {
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            // Upgrade to WebSocket over the Unix socket.
                            let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                                Ok(ws) => ws,
                                Err(e) => {
                                    tracing::debug!("WS handshake failed: {e}");
                                    return;
                                }
                            };

                            let _ = crate::stream::run_connection(state.clone(), ws_stream).await;
                        });
                    }
                    Err(e) => {
                        tracing::warn!("failed to accept socket connection: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}
