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

/// 404 envelope for unknown paths (API.md §1.3: the envelope is everywhere).
pub async fn not_found() -> ApiError {
    ApiError::not_found()
}

/// 405 envelope for known paths with the wrong method.
pub async fn method_not_allowed() -> ApiError {
    ApiError::method_not_allowed()
}

/// Construct the v1 router. Loopback-only bind is enforced by the caller
/// (`serve_unix`), not here — the router must stay address-agnostic.
pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/health", axum::routing::get(health))
        .route("/api/v1/status", axum::routing::get(status))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(state)
}

/// Marker used by tests to assert the health payload shape.
pub const HEALTH_OK: &str = "ok";

/// HTTP status code used in the degraded 503 contract (Phase 2 will emit it
/// from the collector state; kept here so the mapping stays in one place).
pub const DEGRADED_STATUS: StatusCode = StatusCode::SERVICE_UNAVAILABLE;
