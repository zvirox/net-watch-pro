//! Bearer-token auth — `API.md` §1.2 / `AUTH_SECURITY.md` §3.
//!
//! Validate canonical 256-bit hex tokens before comparing equal-length bytes
//! in constant time. The UI obtains the token through the native helper.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::sync::Arc;

use subtle::ConstantTimeEq;

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

        if supplied.is_some_and(|token| token_matches(token, state.token.as_str())) {
            Ok(RequireToken)
        } else {
            Err(ApiError::auth_invalid())
        }
    }
}

/// Validate the canonical token shape, then compare decoded token bytes in constant time.
#[must_use]
pub fn token_matches(supplied: &str, expected: &str) -> bool {
    let Some(supplied) = decode_token(supplied) else {
        return false;
    };
    let Some(expected) = decode_token(expected) else {
        return false;
    };

    bool::from(supplied.ct_eq(&expected))
}

fn decode_token(token: &str) -> Option<[u8; 32]> {
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    let mut decoded = [0_u8; 32];
    let (pairs, remainder) = token.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    for (index, pair) in pairs.iter().enumerate() {
        decoded[index] = (hex_value(pair[0])? << 4) | hex_value(pair[1])?;
    }
    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::token_matches;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_correct_well_formed_token() {
        assert!(token_matches(TOKEN, TOKEN));
    }

    #[test]
    fn rejects_incorrect_well_formed_token() {
        let incorrect = format!("{}0", &TOKEN[..63]);
        assert!(!token_matches(&incorrect, TOKEN));
    }

    #[test]
    fn rejects_malformed_length_and_non_hex_tokens() {
        assert!(!token_matches("", TOKEN));
        assert!(!token_matches(&TOKEN[..63], TOKEN));
        let malformed = format!("{}g", &TOKEN[..63]);
        assert!(!token_matches(&malformed, TOKEN));
    }
}
