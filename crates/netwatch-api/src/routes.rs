//! HTTP routes — System & Status group (API.md §2.1) + envelope fallbacks.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::error::ApiError;
use crate::state::AppState;

/// `GET /health` — S1, no auth, always 200 or connection refused.
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "api": "v1",
    }))
}

/// `GET /status` — S2, auth via socket peer-cred. Drives the DESIGN.md §7.5 chip.
///
/// Phase 1 note: the daemon publishes a stub "monitoring/attached" state
/// (PLAN.md P1.6); the collector lands in Phase 2 and will publish honest
/// states (ARCHITECTURE.md §10 honest-status principle).
pub async fn status(State(state): State<Arc<AppState>>) -> Response {
    let snap = state.status.borrow().clone();
    Json(json!({
        "daemon": snap.daemon,
        "ebpf": snap.ebpf,
        "recording": snap.recording,
        "ws_clients": state.client_count(),
        "uptime_s": state.started_at.elapsed().as_secs(),
        "drops": { "ring_buffer_dropped": 0 },
    }))
    .into_response()
}

/// `GET /connections/live` — C1: one snapshot of the live view; the WS
/// `live.connections` channel serves the same rows (P2.8: REST matches WS).
pub async fn connections_live(State(state): State<Arc<AppState>>) -> Response {
    let rows = (*state.live.snapshot.borrow().clone()).clone();
    Json(json!({
        "captured_at": epoch_now(),
        "frozen": false,
        "data": rows,
    }))
    .into_response()
}

/// Epoch seconds for serve-time stamps.
fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// 404 envelope for unknown paths (API.md §1.3: the envelope is everywhere).
pub async fn not_found() -> ApiError {
    ApiError::not_found()
}

/// 405 envelope for known paths with the wrong method.
pub async fn method_not_allowed() -> ApiError {
    ApiError::method_not_allowed()
}

/// WS upgrade for `/api/v1/stream` (API.md §3). On completion the upgraded
/// connection is handed to `stream::run_connection`.
async fn stream_upgrade(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| async move {
        crate::stream::run_connection(state, socket).await;
    })
}

/// Construct the v1 router. Socket-file permissions (root-owned Unix domain
/// socket) are the transport gate — the router stays address-agnostic.
pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/health", axum::routing::get(health))
        .route("/api/v1/status", axum::routing::get(status))
        .route("/api/v1/connections/live", axum::routing::get(connections_live))
        .route("/api/v1/stream", axum::routing::get(stream_upgrade))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(state)
}

/// Marker used by tests to assert the health payload shape.
pub const HEALTH_OK: &str = "ok";

/// HTTP status code used in the degraded 503 contract (Phase 2 will emit it
/// from the collector state; kept here so the mapping stays in one place).
pub const DEGRADED_STATUS: StatusCode = StatusCode::SERVICE_UNAVAILABLE;
