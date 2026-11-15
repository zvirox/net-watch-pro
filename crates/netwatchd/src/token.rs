//! Per-boot API token — `AUTH_SECURITY.md` §2.3 / `ARCHITECTURE.md` §8.3.
//!
//! The token is the only secret in the product: auto-generated at every
//! boot, never user-set, never logged, and atomically replaced at mode 0600.
//! Packaged builds use the helper's fixed production path.

use std::fmt::Write as _;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
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

/// Atomically replace the token at `path` with a root-owned mode-0600 file.
///
/// Uses a same-directory temporary file and rename so the helper can never
/// observe a truncated/partially-written token during daemon restart.
pub fn write_token_file(path: &Path, token: &str) -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "token must be exactly 64 hexadecimal characters",
        ));
    }

    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "token path has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let parent_meta = std::fs::symlink_metadata(parent)?;
    let production_path = path == Path::new("/var/lib/netwatch-pro/api-token");
    if parent_meta.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "token directory cannot be a symlink",
        ));
    }
    if !parent_meta.is_dir()
        || (production_path && parent_meta.uid() != 0)
        || parent_meta.mode() & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "token directory must be a root-owned directory not writable by group or others",
        ));
    }

    if let Ok(existing) = std::fs::symlink_metadata(path) {
        let file_type = existing.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "existing token path must be a regular file, not a symlink",
            ));
        }
        if production_path && (existing.uid() != 0 || existing.mode() & 0o7777 != 0o600) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "existing production token file must be root-owned with exact mode 0600",
            ));
        }
    }

    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "token path has no filename",
        )
    })?;
    let (temporary, mut file) = create_temporary_file(parent, name.to_string_lossy().as_ref())?;
    let result = write_atomic_file(&mut file, &temporary, path, token);
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn create_temporary_file(
    parent: &Path,
    name: &str,
) -> std::io::Result<(std::path::PathBuf, std::fs::File)> {
    for attempt in 0..32_u8 {
        let temporary = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), attempt));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to allocate a unique temporary token file",
    ))
}

fn write_atomic_file(
    file: &mut std::fs::File,
    temporary: &Path,
    path: &Path,
    token: &str,
) -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata()?;
    if (path == Path::new("/var/lib/netwatch-pro/api-token") && metadata.uid() != 0)
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "new token file must be root-owned with exact mode 0600",
        ));
    }
    file.write_all(token.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;

    std::fs::rename(temporary, path)?;
    std::fs::File::open(path.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn token_is_hex_and_long() {
        let t = generate().unwrap();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(generate().unwrap(), t, "tokens must not repeat");
    }

    #[test]
    fn token_file_is_atomically_replaced_with_mode_0600() {
        let dir = std::env::temp_dir().join(format!("nwp-token-test-{}", std::process::id()));
        let path = dir.join("sub/api-token");
        let first = "a".repeat(64);
        let second = "b".repeat(64);
        write_token_file(&path, &first).unwrap();
        write_token_file(&path, &second).unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o7777, 0o600);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("{second}\n")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_symlink_target_and_malformed_token() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join(format!("nwp-token-reject-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("api-token");
        let target = dir.join("target");
        std::fs::write(&target, "existing").unwrap();
        symlink(&target, &path).unwrap();
        assert!(write_token_file(&path, &"c".repeat(64)).is_err());
        assert!(write_token_file(&dir.join("malformed"), "bad").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
