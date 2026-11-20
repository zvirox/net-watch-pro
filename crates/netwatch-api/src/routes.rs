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
    // Real eBPF counters from the collector's latest frame; all zero when no
    // collector runs (honest). ring_buffer_dropped stays 0 in Phase C: the
    // collector uses a map, not a ring buffer (API.md §2.1 S2).
    let bpf_stats = state.live.apps.borrow().stats;
    Json(json!({
        "daemon": snap.daemon,
        "ebpf": snap.ebpf,
        "recording": snap.recording,
        "ws_clients": state.client_count(),
        "uptime_s": state.started_at.elapsed().as_secs(),
        "drops": {
            "ring_buffer_dropped": 0,
            "bpf_packets_seen": bpf_stats.bpf_packets_seen,
            "bpf_loopback_skipped": bpf_stats.bpf_loopback_skipped,
            "bpf_map_full": bpf_stats.bpf_map_full,
        },
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

/// `GET /apps` — A1: ranked app list (API.md §2.4).
pub async fn apps(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::apps::apps_page(&state.db, now, raw.as_deref()).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /apps/{id}` — A2: one app, including soft-deleted history rows.
pub async fn app_detail(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let Ok(id) = id.parse::<i64>() else {
        return ApiError::not_found().into_response();
    };
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::apps::app_detail(&state.db, now, id).await {
        Ok(row) => Json(row).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /apps/{id}/icon` — registered icon bytes, keyed by ID (A2).
pub async fn app_icon(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let Ok(id) = id.parse::<i64>() else {
        return ApiError::not_found().into_response();
    };
    match crate::apps::app_icon(&state.db, id).await {
        Ok((content_type, bytes)) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            bytes,
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /apps/{id}/usage` — A3: app-scoped usage series.
pub async fn app_usage(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let Ok(id) = id.parse::<i64>() else {
        return ApiError::not_found().into_response();
    };
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::timeline::app_timeline(&state.db, now, id, raw.as_deref()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /apps/{id}/endpoints` — A4: ranked remote endpoints.
pub async fn app_endpoints(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let Ok(id) = id.parse::<i64>() else {
        return ApiError::not_found().into_response();
    };
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::apps::app_endpoints(&state.db, now, id, raw.as_deref()).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /connections` — C2 history episodes: keyset pagination, whitelist
/// filters/sort, raw window + beyond-window aggregate rows (API.md §2.2).
/// The raw query string is parsed by hand so every error uses the §1.3
/// envelope (axum's Query extractor rejects with a plain 400).
pub async fn connections(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::history::connections_page(&state.db, raw.as_deref().unwrap_or(""), now).await {
        Ok(envelope) => Json(envelope).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /connections/{id}` — C3: one raw episode (C1 shape + packets),
/// raw window only (API.md §2.2). Non-numeric ids answer 404 — the resource
/// cannot exist, and C3's error list has no 400.
pub async fn connection_by_id(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    let Ok(id) = id.parse::<i64>() else {
        return ApiError::new(
            netwatch_common::ErrorCode::NotFound,
            StatusCode::NOT_FOUND,
            format!("no connection episode with id {id}"),
        )
        .into_response();
    };
    match crate::history::connection_by_id(&state.db, id, now).await {
        Ok(row) => Json(row).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /usage/interfaces` — U3: per-interface totals, excluding `lo`.
pub async fn usage_interfaces(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::interfaces_usage::interfaces_usage(&state.db, now, raw.as_deref()).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /usage/rate-history` — U4: bounded last-hour rate samples (API.md §2.5).
///
/// The ring lives in process memory only; `in_memory:true` tells the UI to
/// label it as lost-on-restart (owner L-OQ1).
pub async fn usage_rate_history(State(state): State<Arc<AppState>>) -> Response {
    Json(json!({
        "samples": state.live.rate_history.samples(),
        "in_memory": true,
    }))
    .into_response()
}

/// `GET /usage/timeline` — U2: local-hour/local-day usage series (API.md §2.5).
pub async fn usage_timeline(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::timeline::timeline(&state.db, now, raw.as_deref()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /usage/summary` — U1: interface totals, attributed bytes, gap and
/// top talkers (API.md §2.5). Boundaries use the configured timezone and
/// billing cycle (D4a).
pub async fn usage_summary(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::usage::summary(&state.db, now, raw.as_deref()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /interfaces` — I1: all interfaces with metered/excluded flags.
pub async fn interfaces_list(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(_raw): axum::extract::RawQuery,
) -> Response {
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::interfaces::list(&state.db, now).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `PATCH /interfaces/{id}` — I2: update metered/excluded flags.
pub async fn interface_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    request: axum::extract::Request,
) -> Response {
    use axum::body::to_bytes;

    let Ok(id) = id.parse::<i64>() else {
        return ApiError::not_found().into_response();
    };
    let (_, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 16 * 1024).await else {
        return ApiError::query_invalid("Request body is too large").into_response();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return ApiError::query_invalid("Request body must be JSON").into_response();
    };
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::interfaces::update(&state.db, now, id, &value).await {
        Ok((row, exclusion_changed)) => {
            // API.md I2: exclusion changes totals semantics — wake WS clients.
            if exclusion_changed {
                state.status_tx.send_modify(|_| {});
            }
            Json(row).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// `GET /license` — L1: current tier and server-side entitlements.
pub async fn license_state(State(state): State<Arc<AppState>>) -> Response {
    match crate::license::current(&state.db).await {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `POST /license/activate` — L2: verify an offline-signed key.
pub async fn license_activate(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
) -> Response {
    use axum::body::to_bytes;

    let (_, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 16 * 1024).await else {
        return ApiError::query_invalid("Request body is too large").into_response();
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return ApiError::query_invalid("Request body must be JSON").into_response(),
    };
    let Some(key) = value.get("key").and_then(serde_json::Value::as_str) else {
        return ApiError::query_invalid("Missing license key").into_response();
    };
    let now = i64::try_from(epoch_now()).unwrap_or_default();
    match crate::license::activate(&state.db, key, now).await {
        Ok(payload) => {
            // PRD Flow 4: Pro features unlock instantly without a restart.
            state.status_tx.send_modify(|_| {});
            Json(payload).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// `DELETE /license` — L3: reset to free immediately.
pub async fn license_deactivate(State(state): State<Arc<AppState>>) -> Response {
    match crate::license::deactivate(&state.db).await {
        Ok(payload) => {
            state.status_tx.send_modify(|_| {});
            Json(payload).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// `GET /settings` — T1: typed settings with tier-aware bounds.
pub async fn settings_list(State(state): State<Arc<AppState>>) -> Response {
    match crate::settings::list(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `PUT /settings/{key}` — T2: validate and persist one setting.
pub async fn settings_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(key): axum::extract::Path<String>,
    request: axum::extract::Request,
) -> Response {
    use axum::body::to_bytes;

    let (_, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 16 * 1024).await else {
        return ApiError::query_invalid("Request body is too large").into_response();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return ApiError::query_invalid("Request body must be JSON").into_response();
    };
    let Some(new_value) = value.get("value") else {
        return ApiError::query_invalid("Missing setting value").into_response();
    };
    match crate::settings::update(&state.db, &key, new_value).await {
        Ok(entry) => Json(entry).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `POST /exports` — E1: create an export job (Pro only, API.md §2.3).
pub async fn export_create(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
) -> Response {
    use axum::body::to_bytes;

    let now = epoch_now();
    let (_, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 64 * 1024).await else {
        return ApiError::query_invalid("Request body is too large").into_response();
    };
    match crate::exports::create_job(
        &state.db,
        &state.exports,
        &state.export_create_limiter,
        &bytes,
        now,
    )
    .await
    {
        Ok(payload) => (StatusCode::ACCEPTED, Json(payload)).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /exports/{id}` — E2: job status.
pub async fn export_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    match crate::exports::job_status(&state.exports, &id) {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => err.into_response(),
    }
}

/// `GET /exports/{id}/content` — E3: download the generated file with a
/// `Content-Disposition` attachment header (API.md E3).
pub async fn export_content(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    match crate::exports::job_content(&state.exports, &state.export_download_limiter, &id) {
        Ok(content) => {
            let disposition = format!("attachment; filename=\"{}\"", content.filename);
            Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, content.content_type)
                .header(axum::http::header::CONTENT_DISPOSITION, disposition)
                .body(axum::body::Body::from(content.body))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(err) => err.into_response(),
    }
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
        .route(
            "/api/v1/connections/live",
            axum::routing::get(connections_live),
        )
        .route(
            "/api/v1/connections/{id}",
            axum::routing::get(connection_by_id),
        )
        .route("/api/v1/connections", axum::routing::get(connections))
        .route("/api/v1/apps", axum::routing::get(apps))
        .route("/api/v1/apps/{id}/icon", axum::routing::get(app_icon))
        .route("/api/v1/apps/{id}/usage", axum::routing::get(app_usage))
        .route(
            "/api/v1/apps/{id}/endpoints",
            axum::routing::get(app_endpoints),
        )
        .route("/api/v1/apps/{id}", axum::routing::get(app_detail))
        .route(
            "/api/v1/license",
            axum::routing::get(license_state).delete(license_deactivate),
        )
        .route(
            "/api/v1/license/activate",
            axum::routing::post(license_activate),
        )
        .route("/api/v1/interfaces", axum::routing::get(interfaces_list))
        .route(
            "/api/v1/interfaces/{id}",
            axum::routing::patch(interface_update),
        )
        .route("/api/v1/settings", axum::routing::get(settings_list))
        .route(
            "/api/v1/settings/{key}",
            axum::routing::put(settings_update),
        )
        .route("/api/v1/exports", axum::routing::post(export_create))
        .route("/api/v1/exports/{id}", axum::routing::get(export_status))
        .route(
            "/api/v1/exports/{id}/content",
            axum::routing::get(export_content),
        )
        .route("/api/v1/usage/summary", axum::routing::get(usage_summary))
        .route("/api/v1/usage/timeline", axum::routing::get(usage_timeline))
        .route(
            "/api/v1/usage/interfaces",
            axum::routing::get(usage_interfaces),
        )
        .route(
            "/api/v1/usage/rate-history",
            axum::routing::get(usage_rate_history),
        )
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
