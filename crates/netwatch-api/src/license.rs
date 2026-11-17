//! License endpoints L1–L3 (API.md §2.8).
//!
//! Verification is fully offline via `netwatch_core::license`; only the
//! fingerprint and entitlement timestamps are persisted (DATABASE.md §2.7).

use netwatch_core::license::{self, LicenseError};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::ApiError;

const PUBLIC_KEY_HEX: &str = include_str!("../../../packaging/licenses/nwp-public.hex");

/// Entitlements derived from the current tier (API.md L1).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Entitlements {
    /// Advanced C2 filters (Pro).
    pub advanced_filters: bool,
    /// CSV/JSON export (Pro).
    pub export: bool,
    /// Retention ceiling in days.
    pub retention_max_days: i64,
}

/// L1 response payload.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LicenseState {
    /// Current tier.
    pub tier: &'static str,
    /// Server-side entitlements; the UI never parses the key itself.
    pub entitlements: Entitlements,
}

/// Read the current entitlement state, falling back to free on a missing row.
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn current(pool: &SqlitePool) -> Result<LicenseState, ApiError> {
    let tier: Option<String> = sqlx::query_scalar("SELECT tier FROM license WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)?;
    Ok(state_from_tier(tier.as_deref()))
}

/// Whether the singleton license row currently grants Pro.
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn is_pro(pool: &SqlitePool) -> Result<bool, ApiError> {
    Ok(current(pool).await?.tier == "pro")
}

/// Activate a signed offline key with a caller-supplied public key.
///
/// This variant lets tests verify the full success path with an ephemeral
/// keypair; the HTTP route uses [`activate`] with the embedded public key.
///
/// # Errors
///
/// `QUERY_INVALID` for malformed keys, `LICENSE_INVALID` for bad/expired
/// keys, `CONFLICT` when already Pro, and `DB_ERROR` for SQLite failures.
pub async fn activate_with_public_key(
    pool: &SqlitePool,
    key: &str,
    now: i64,
    public_key: &[u8; 32],
) -> Result<LicenseState, ApiError> {
    if current(pool).await?.tier == "pro" {
        return Err(ApiError::conflict(
            "Pro is already active; no action needed",
        ));
    }
    let claims = license::verify_license(key, public_key, now).map_err(license_error)?;
    let result = sqlx::query(
        "UPDATE license SET tier = 'pro', key_fingerprint = ?1,
                activated_at = ?2, expires_at = ?3, signature_verified_at = ?2,
                updated_at = strftime('%s','now')
          WHERE id = 1 AND tier = 'free'",
    )
    .bind(&claims.fingerprint)
    .bind(now)
    .bind(claims.expires_at)
    .execute(pool)
    .await
    .map_err(crate::history::db_error)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "Pro is already active; no action needed",
        ));
    }
    Ok(state_from_tier(Some("pro")))
}

/// Activate a signed key using the repository's embedded public key.
///
/// # Errors
///
/// Same as [`activate_with_public_key`], plus `LICENSE_INVALID` if the public
/// key file is unreadable/malformed.
pub async fn activate(pool: &SqlitePool, key: &str, now: i64) -> Result<LicenseState, ApiError> {
    let public_key = license::parse_public_key_hex(PUBLIC_KEY_HEX.trim())
        .map_err(|_| ApiError::license_invalid("Embedded public key is invalid"))?;
    activate_with_public_key(pool, key, now, &public_key).await
}

/// Reset the singleton license row to free.
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn deactivate(pool: &SqlitePool) -> Result<LicenseState, ApiError> {
    sqlx::query(
        "UPDATE license SET tier = 'free', key_fingerprint = NULL,
                activated_at = NULL, expires_at = NULL, signature_verified_at = NULL,
                updated_at = strftime('%s','now')
          WHERE id = 1",
    )
    .execute(pool)
    .await
    .map_err(crate::history::db_error)?;
    Ok(state_from_tier(Some("free")))
}

fn state_from_tier(tier: Option<&str>) -> LicenseState {
    let pro = tier == Some("pro");
    LicenseState {
        tier: if pro { "pro" } else { "free" },
        entitlements: Entitlements {
            advanced_filters: pro,
            export: pro,
            retention_max_days: if pro { 365 } else { 90 },
        },
    }
}

fn license_error(error: LicenseError) -> ApiError {
    match error {
        LicenseError::Malformed => ApiError::query_invalid("Invalid license key format"),
        LicenseError::InvalidSignature => ApiError::license_invalid("License signature is invalid"),
        LicenseError::InvalidPayload => ApiError::license_invalid("License payload is unsupported"),
        LicenseError::Expired => ApiError::license_invalid("License key has expired"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use ed25519_dalek::{Signer, SigningKey};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::io::Read;

    const TEST_VERSION: u8 = 1;

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE license (
                id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
                tier TEXT NOT NULL DEFAULT 'free',
                key_fingerprint TEXT,
                activated_at INTEGER,
                expires_at INTEGER,
                signature_verified_at INTEGER,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO license (id) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn ephemeral_key() -> SigningKey {
        let mut seed = [0_u8; 32];
        std::fs::File::open("/dev/urandom")
            .unwrap()
            .read_exact(&mut seed)
            .unwrap();
        SigningKey::from_bytes(&seed)
    }

    fn signed_key(signing_key: &SigningKey, expiry: u64) -> String {
        let mut payload = vec![TEST_VERSION, 1];
        payload.extend_from_slice(&expiry.to_be_bytes());
        payload.extend_from_slice(&[9_u8; 16]);
        let signature = signing_key.sign(&payload).to_bytes();
        format!(
            "NWP1-{}-{}",
            STANDARD_NO_PAD.encode(payload),
            STANDARD_NO_PAD.encode(signature)
        )
    }

    #[tokio::test]
    async fn activate_stores_only_fingerprint_and_unlocks_pro() {
        let pool = test_db().await;
        let signing_key = ephemeral_key();
        let key = signed_key(&signing_key, 0);
        let state =
            activate_with_public_key(&pool, &key, 1_000, signing_key.verifying_key().as_bytes())
                .await
                .unwrap();
        assert_eq!(state, state_from_tier(Some("pro")));

        let (tier, fingerprint, activated_at): (String, String, i64) =
            sqlx::query_as("SELECT tier, key_fingerprint, activated_at FROM license WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tier, "pro");
        assert_eq!(fingerprint.len(), 64);
        assert!(!fingerprint.contains(key.as_str()));
        assert_eq!(activated_at, 1_000);

        let conflict =
            activate_with_public_key(&pool, &key, 1_001, signing_key.verifying_key().as_bytes())
                .await
                .unwrap_err();
        assert_eq!(conflict.code(), netwatch_common::ErrorCode::Conflict);

        let state = deactivate(&pool).await.unwrap();
        assert_eq!(state.tier, "free");
        let fingerprint: Option<String> =
            sqlx::query_scalar("SELECT key_fingerprint FROM license WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(fingerprint.is_none());
    }
}
