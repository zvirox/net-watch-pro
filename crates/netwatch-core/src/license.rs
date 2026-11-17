//! Offline Pro license verification (P6.2, `AUTH_SECURITY.md` §2.4).
//!
//! Key format: `NWP1-<payload>-<signature>` with standard base64 (no padding).
//! Payload bytes: version `1`, tier `1` (Pro), 8-byte big-endian expiry
//! (`0` = perpetual), then a 16-byte nonce. The Ed25519 signature covers the
//! payload bytes. No network calls and no private key exist in this crate.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

const PREFIX: &str = "NWP1-";
const PAYLOAD_LEN: usize = 26;
const VERSION: u8 = 1;
const TIER_PRO: u8 = 1;

/// Why a license key was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseError {
    /// Prefix, segments, base64, or payload length are wrong.
    Malformed,
    /// Signature does not verify with the embedded public key.
    InvalidSignature,
    /// Unsupported payload version or tier.
    InvalidPayload,
    /// Signature is valid but the key expired at or before `now`.
    Expired,
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "malformed license key",
            Self::InvalidSignature => "license signature is invalid",
            Self::InvalidPayload => "unsupported license payload",
            Self::Expired => "license key has expired",
        })
    }
}

impl std::error::Error for LicenseError {}

/// Tier granted by a verified license.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseTier {
    /// Free tier — no signed key.
    Free,
    /// Pro tier — unlocked by a signed key.
    Pro,
}

/// Claims extracted from a verified license key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseClaims {
    /// Entitlement tier.
    pub tier: LicenseTier,
    /// Expiry epoch seconds; `None` means perpetual.
    pub expires_at: Option<i64>,
    /// SHA-256 fingerprint of the raw key (never store the raw key).
    pub fingerprint: String,
}

/// Verify a license key offline against a 32-byte Ed25519 public key.
///
/// `now` is supplied by the caller so expiry tests are deterministic and the
/// verifier itself never reads the clock or the network.
///
/// # Errors
///
/// Returns [`LicenseError::Malformed`], [`LicenseError::InvalidSignature`],
/// [`LicenseError::InvalidPayload`], or [`LicenseError::Expired`].
pub fn verify_license(
    key: &str,
    public_key: &[u8; 32],
    now: i64,
) -> Result<LicenseClaims, LicenseError> {
    let body = key.strip_prefix(PREFIX).ok_or(LicenseError::Malformed)?;
    let mut parts = body.split('-');
    let payload_part = parts.next().ok_or(LicenseError::Malformed)?;
    let signature_part = parts.next().ok_or(LicenseError::Malformed)?;
    if parts.next().is_some() {
        return Err(LicenseError::Malformed);
    }
    let payload = STANDARD_NO_PAD
        .decode(payload_part)
        .map_err(|_| LicenseError::Malformed)?;
    let signature_bytes = STANDARD_NO_PAD
        .decode(signature_part)
        .map_err(|_| LicenseError::Malformed)?;
    if payload.len() != PAYLOAD_LEN || signature_bytes.len() != ed25519_dalek::SIGNATURE_LENGTH {
        return Err(LicenseError::Malformed);
    }

    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| LicenseError::Malformed)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| LicenseError::InvalidSignature)?;
    verifying_key
        .verify_strict(&payload, &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;

    if payload[0] != VERSION || payload[1] != TIER_PRO {
        return Err(LicenseError::InvalidPayload);
    }
    let expiry = u64::from_be_bytes(
        payload[2..10]
            .try_into()
            .map_err(|_| LicenseError::Malformed)?,
    );
    let expires_at = if expiry == 0 {
        None
    } else {
        Some(i64::try_from(expiry).map_err(|_| LicenseError::InvalidPayload)?)
    };
    if expires_at.is_some_and(|expires_at| now >= expires_at) {
        return Err(LicenseError::Expired);
    }

    Ok(LicenseClaims {
        tier: LicenseTier::Pro,
        expires_at,
        fingerprint: hex_encode(&Sha256::digest(key.as_bytes())),
    })
}

/// Decode the repository's embedded public key (64 hex characters).
///
/// # Errors
///
/// Returns [`LicenseError::Malformed`] for wrong length or non-hex input.
pub fn parse_public_key_hex(hex: &str) -> Result<[u8; 32], LicenseError> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return Err(LicenseError::Malformed);
    }
    let mut key = [0_u8; 32];
    for (index, chunk) in bytes.as_chunks::<2>().0.iter().enumerate() {
        let Some(high) = hex_nibble(chunk[0]) else {
            return Err(LicenseError::Malformed);
        };
        let Some(low) = hex_nibble(chunk[1]) else {
            return Err(LicenseError::Malformed);
        };
        key[index] = (high << 4) | low;
    }
    Ok(key)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::io::Read;

    fn ephemeral_key() -> SigningKey {
        let mut seed = [0_u8; 32];
        std::fs::File::open("/dev/urandom")
            .unwrap()
            .read_exact(&mut seed)
            .unwrap();
        SigningKey::from_bytes(&seed)
    }

    fn signed_key(signing_key: &SigningKey, expiry: u64) -> String {
        let mut payload = Vec::with_capacity(PAYLOAD_LEN);
        payload.push(VERSION);
        payload.push(TIER_PRO);
        payload.extend_from_slice(&expiry.to_be_bytes());
        payload.extend_from_slice(&[7_u8; 16]);
        let signature = signing_key.sign(&payload).to_bytes();
        format!(
            "{PREFIX}{}-{}",
            STANDARD_NO_PAD.encode(payload),
            STANDARD_NO_PAD.encode(signature)
        )
    }

    #[test]
    fn valid_key_verifies_offline() {
        let signing_key = ephemeral_key();
        let key = signed_key(&signing_key, 0);
        let claims = verify_license(&key, signing_key.verifying_key().as_bytes(), 1_000).unwrap();
        assert_eq!(claims.tier, LicenseTier::Pro);
        assert_eq!(claims.expires_at, None);
        assert_eq!(claims.fingerprint.len(), 64);
        assert_eq!(
            verify_license(&key, signing_key.verifying_key().as_bytes(), 1_001).unwrap(),
            claims
        );
    }

    #[test]
    fn tampered_and_wrong_key_fail() {
        let signing_key = ephemeral_key();
        let key = signed_key(&signing_key, 0);
        let other = ephemeral_key();
        assert_eq!(
            verify_license(&key, other.verifying_key().as_bytes(), 1_000),
            Err(LicenseError::InvalidSignature)
        );
        let mut segments: Vec<String> = key.split('-').map(str::to_string).collect();
        let index = segments[2]
            .char_indices()
            .find_map(|(index, ch)| (ch != 'A').then_some(index))
            .unwrap();
        segments[2].replace_range(index..=index, "A");
        let tampered = segments.join("-");
        assert_eq!(
            verify_license(&tampered, signing_key.verifying_key().as_bytes(), 1_000),
            Err(LicenseError::InvalidSignature)
        );
    }

    #[test]
    fn expired_and_malformed_keys_are_distinct() {
        let signing_key = ephemeral_key();
        let expired = signed_key(&signing_key, 1_000);
        assert_eq!(
            verify_license(&expired, signing_key.verifying_key().as_bytes(), 1_000),
            Err(LicenseError::Expired)
        );
        assert_eq!(
            verify_license("BAD-KEY", signing_key.verifying_key().as_bytes(), 1_000),
            Err(LicenseError::Malformed)
        );
    }

    #[test]
    fn embedded_public_key_is_parseable() {
        let embedded = include_str!("../../../packaging/licenses/nwp-public.hex");
        assert!(parse_public_key_hex(embedded.trim()).is_ok());
        assert_eq!(parse_public_key_hex("00"), Err(LicenseError::Malformed));
    }
}
