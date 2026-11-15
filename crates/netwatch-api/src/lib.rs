//! API server: WS routes, token auth, request/validation types.
//!
//! Binding contract: `../API.md` (+ `docs/api-contract.md` mirror).
//! Loopback-only, bearer-token, error envelope per API.md §1.3.

// Tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping in test
// assertions is standard and expected.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod error;
pub mod routes;
pub mod state;
pub mod stream;

use std::net::SocketAddr;
use std::sync::Arc;

use netwatch_common::StatusSnapshot;
use tokio::net::TcpListener;
use tokio::sync::watch;

pub use state::AppState;

/// Serve the API on `addr` until `shutdown` resolves.
///
/// # Errors
///
/// - Refuses non-loopback addresses (loopback-only contract, API.md §1.1).
/// - Propagates bind failures (port already in use, permission denied).
/// - Propagates serve-loop failures (axum server error).
///
/// Loopback-only by contract (API.md §1.1 / ARCHITECTURE.md §5.3): a
/// non-loopback address is a startup-class failure and refuses to bind.
pub async fn serve(
    addr: SocketAddr,
    token: Arc<String>,
    status: watch::Receiver<StatusSnapshot>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), crate::error::ApiError> {
    if !addr.ip().is_loopback() {
        return Err(crate::error::ApiError::new(
            netwatch_common::ErrorCode::PermissionDenied,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("API bind must be loopback-only, got {}", addr.ip()),
        ));
    }

    let state = Arc::new(AppState::new(token, status));
    let router = routes::router(state);
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        crate::error::ApiError::new(
            netwatch_common::ErrorCode::PermissionDenied,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to bind {addr}: {e}"),
        )
    })?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| {
            crate::error::ApiError::new(
                netwatch_common::ErrorCode::DaemonUnreachable,
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("server error: {e}"),
            )
        })?;
    Ok(())
}
