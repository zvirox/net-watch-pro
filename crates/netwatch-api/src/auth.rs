//! Peer-credential authorization.
//!
//! Auth is performed at the socket level: the daemon checks the connecting
//! process's UID via `SO_PEERCRED` against a root-owned allowed-UID list.
//! This module is a placeholder for future authorization logic; the WS layer
//! trusts the socket's peer-auth result.

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::ApiError;
use crate::state::AppState;

/// Extractor that checks peer-credential authorization.
///
/// The actual check happens at the socket level (the daemon verifies the
/// connecting UID against the allowed-UID list). By the time a request
/// reaches this extractor, the peer has already been authorized.
pub struct Authorized;

impl FromRequestParts<Arc<AppState>> for Authorized {
    type Rejection = ApiError;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Auth already validated at the socket level via SO_PEERCRED.
        Ok(Authorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorized_is_unit() {
        let _ = Authorized;
    }
}
