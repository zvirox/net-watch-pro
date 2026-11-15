//! Per-boot API token — `AUTH_SECURITY.md` §2.3 / `ARCHITECTURE.md` §8.3.
//!
//! The token is the only secret in the product: auto-generated at every
//! boot, never user-set, never logged, written mode 0600 so only root (the
//! daemon) can read it; the Tauri shell reads it once at link time.

use std::fmt::Write as _;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// Generate a fresh per-boot token: 32 random bytes, hex-encoded (64 chars).
///
/// # Errors
///
/// Propagates a system-CSPRNG failure — a startup-class fault (§7.1); the
/// caller exits with a distinct code rather than falling back to a
/// predictable token.
pub fn generate() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for b in &bytes {
        let _ = write!(token, "{b:02x}");
    }
    Ok(token)
}

/// Write the token to `path` with mode 0600, creating parent dirs 0700.
///
/// Called every boot (per-boot rotation); an existing file is truncated.
pub fn write_token_file(path: &Path, token: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        // Best-effort tightening of the state dir; packaging sets the
        // canonical permissions (AUTH_SECURITY.md §9), this guards dev runs.
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }

    // OpenOptionsExt: create/truncate with 0600 from the start — no window
    // where the token file is world-readable.
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(token.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_hex_and_long() {
        let t = generate().unwrap();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(generate().unwrap(), t, "tokens must not repeat");
    }

    #[test]
    fn token_file_is_mode_0600() {
        let dir = std::env::temp_dir().join(format!("nwp-token-test-{}", std::process::id()));
        let path = dir.join("sub/api-token");
        write_token_file(&path, "abc123").unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.trim(), "abc123");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
