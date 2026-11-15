//! Bearer-token auth — `API.md` §1.2 / `AUTH_SECURITY.md` §2.3.
//!
//! The UI never gets broader file access; the Tauri shell reads the 0600
//! token file once at link time and sends `Authorization: Bearer <token>`.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::sync::Arc;

use crate::error::ApiError;
use crate::state::AppState;

/// Extractor that rejects requests without the correct bearer token.
pub struct RequireToken;

impl FromRequestParts<Arc<AppState>> for RequireToken {
    type Rejection = ApiError;

    // Signature is async per the trait; the check itself is synchronous.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let supplied = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        match supplied {
            Some(tok) if tok == state.token.as_str() => Ok(RequireToken),
            // Deliberately not constant-time: loopback-only surface; the
            // threat model (AUTH_SECURITY.md §2) has no remote attacker
            // position. Revisit if the bind address ever leaves 127.0.0.1.
            _ => Err(ApiError::auth_invalid()),
        }
    }
}
