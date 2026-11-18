//! Typed API errors and the error envelope — binding contract: `../API.md`
//! §1.3 (`docs/api-contract.md` mirror). Every error response, on every
//! endpoint, uses this envelope; unknown codes are a contract violation.
//!
//! The `code` strings are the contract's stable identifiers; see
//! `netwatch_common::ErrorCode` for the full set.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use netwatch_common::ErrorCode;
use serde_json::{Map, Value, json};

/// API error with a stable code and HTTP mapping (API.md §1.3 table).
#[derive(Debug, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct ApiError {
    code: ErrorCode,
    status: StatusCode,
    message: String,
    /// Structured detail fields rendered under `error.details` (API.md §1.3).
    details: Map<String, Value>,
    /// When set, emitted as a `Retry-After` response header (API.md §1.5).
    retry_after_s: Option<u64>,
}

impl ApiError {
    /// Build an error with an explicit HTTP status (used for degraded 503s).
    pub fn new(code: ErrorCode, status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            status,
            message: message.into(),
            details: Map::new(),
            retry_after_s: None,
        }
    }

    /// 429 — a per-process rate limit was exceeded (API.md §1.5). Carries
    /// `details.retry_after_s` and a `Retry-After` header so clients can back off.
    #[must_use]
    pub fn rate_limited(retry_after_s: u64) -> Self {
        let mut details = Map::new();
        details.insert("retry_after_s".to_string(), json!(retry_after_s));
        Self {
            code: ErrorCode::RateLimited,
            status: StatusCode::TOO_MANY_REQUESTS,
            message: format!("Rate limit exceeded; retry in {retry_after_s}s"),
            details,
            retry_after_s: Some(retry_after_s),
        }
    }

    /// Stable contract code (API.md §1.3) — used by callers and tests to
    /// assert the exact error without matching on message text.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// 401 — missing or invalid bearer token.
    #[must_use]
    pub fn auth_invalid() -> Self {
        Self::new(
            ErrorCode::AuthInvalid,
            StatusCode::UNAUTHORIZED,
            "Missing or invalid bearer token",
        )
    }

    /// 404 — unknown route or pruned resource.
    #[must_use]
    pub fn not_found() -> Self {
        Self::new(
            ErrorCode::NotFound,
            StatusCode::NOT_FOUND,
            "Resource not found",
        )
    }

    /// 405 — method not allowed on a known route.
    #[must_use]
    pub fn method_not_allowed() -> Self {
        Self::new(
            ErrorCode::MethodNotAllowed,
            StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed",
        )
    }

    /// 400 — malformed query/parameter.
    #[must_use]
    pub fn query_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::QueryInvalid, StatusCode::BAD_REQUEST, message)
    }

    /// 403 — offline license key rejected (bad signature, payload, or expiry).
    #[must_use]
    pub fn license_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::LicenseInvalid, StatusCode::FORBIDDEN, message)
    }

    /// 409 — conflicting state, e.g. activating an already-Pro license.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, StatusCode::CONFLICT, message)
    }
}

impl IntoResponse for ApiError {
    /// Render the §1.3 envelope. `details` is always an object; `request_id`
    /// is an opaque short hex id (never a secret) for log correlation.
    fn into_response(self) -> Response {
        let Self {
            code,
            status,
            message,
            details,
            retry_after_s,
        } = self;
        let body = json!({
            "error": {
                "code": code,
                "message": message,
                "details": details,
                "request_id": request_id(),
            }
        });
        let mut response = (status, Json(body)).into_response();
        if let Some(seconds) = retry_after_s
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

/// Short opaque correlation id: 8 random bytes, hex-encoded (16 chars).
fn request_id() -> String {
    use std::fmt::Write as _;
    let mut bytes = [0u8; 8];
    // CSPRNG failure degrades gracefully inside an error-response path:
    // a static correlation id instead of a panic (§7.1: errors never crash).
    if getrandom::fill(&mut bytes).is_err() {
        return "none".to_string();
    }
    let mut id = String::with_capacity(bytes.len() * 2);
    for b in &bytes {
        let _ = write!(id, "{b:02x}");
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn envelope_has_contract_fields() {
        let resp = ApiError::auth_invalid().into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], "AUTH_INVALID");
        assert_eq!(v["error"]["details"], serde_json::json!({}));
        assert_eq!(v["error"]["request_id"].as_str().unwrap().len(), 16);
    }

    #[tokio::test]
    async fn rate_limited_has_details_and_retry_after() {
        let resp = ApiError::rate_limited(42).into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(resp.headers()[header::RETRY_AFTER], "42");
        let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], "RATE_LIMITED");
        assert_eq!(v["error"]["details"]["retry_after_s"], 42);
    }
}
