//! Typed API errors and the error envelope — binding contract: `../API.md`
//! §1.3 (`docs/api-contract.md` mirror). Every error response, on every
//! endpoint, uses this envelope; unknown codes are a contract violation.
//!
//! The `code` strings are the contract's stable identifiers; see
//! `netwatch_common::ErrorCode` for the full set.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use netwatch_common::ErrorCode;
use serde_json::json;

/// API error with a stable code and HTTP mapping (API.md §1.3 table).
#[derive(Debug, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct ApiError {
    code: ErrorCode,
    status: StatusCode,
    message: String,
}

impl ApiError {
    /// Build an error with an explicit HTTP status (used for degraded 503s).
    pub fn new(code: ErrorCode, status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            status,
            message: message.into(),
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
}

impl IntoResponse for ApiError {
    /// Render the §1.3 envelope. `details` is always an object; `request_id`
    /// is an opaque short hex id (never a secret) for log correlation.
    fn into_response(self) -> Response {
        let body = json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "details": {},
                "request_id": request_id(),
            }
        });
        (self.status, Json(body)).into_response()
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
}
