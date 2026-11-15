//! Fixed-path token file validation shared with the helper unit tests.

#![deny(unsafe_code)]
#![deny(missing_docs)]

use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

/// Production token location. The helper never accepts this path from a caller.
pub const TOKEN_PATH: &str = "/var/lib/netwatch-pro/api-token";
const TOKEN_LEN: usize = 64;

/// Open the fixed token file without following a final-component symlink, then validate its metadata and contents.
///
/// # Errors
///
/// Returns an error if the path, file metadata, or token contents violate the production contract.
pub fn read_token() -> Result<String, String> {
    read_token_at(Path::new(TOKEN_PATH))
}

fn read_token_at(path: &Path) -> Result<String, String> {
    validate_parent_directories(path)?;
    let file = open_nofollow(path).map_err(|error| format!("cannot open token file: {error}"))?;
    validate_metadata(&file).map_err(|error| format!("unsafe token file: {error}"))?;

    let mut bytes = Vec::with_capacity(TOKEN_LEN + 2);
    file.take((TOKEN_LEN + 3) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read token file: {error}"))?;
    let token = trim_one_line_ending(&bytes);

    if token.len() != TOKEN_LEN || !token.iter().all(u8::is_ascii_hexdigit) {
        return Err("token must be exactly 64 hexadecimal characters".to_string());
    }

    String::from_utf8(token.to_vec()).map_err(|_| "token is not valid ASCII".to_string())
}

fn validate_parent_directories(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "token path has no parent".to_string())?;
    let metadata = std::fs::symlink_metadata(parent).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != 0
        || metadata.mode() & 0o022 != 0
    {
        return Err("parent directory must be root-owned and not group/other writable".to_string());
    }
    Ok(())
}

fn open_nofollow(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

fn validate_metadata(file: &File) -> Result<(), String> {
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("not a regular file".to_string());
    }
    if metadata.uid() != 0 {
        return Err("owner must be UID 0".to_string());
    }
    if metadata.mode() & 0o7777 != 0o600 {
        return Err("permissions must be exactly 0600".to_string());
    }
    Ok(())
}

fn trim_one_line_ending(bytes: &[u8]) -> &[u8] {
    match bytes.strip_suffix(b"\n") {
        Some(without_lf) => without_lf.strip_suffix(b"\r").unwrap_or(without_lf),
        None => bytes,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{TOKEN_LEN, trim_one_line_ending};

    #[test]
    fn accepts_exact_token_with_optional_line_ending() {
        let token = vec![b'a'; TOKEN_LEN];
        assert_eq!(trim_one_line_ending(&token), token.as_slice());
        let mut newline = token.clone();
        newline.push(b'\n');
        assert_eq!(trim_one_line_ending(&newline), token.as_slice());
    }

    #[test]
    fn rejects_extra_lines_and_oversized_content() {
        let mut two_lines = vec![b'a'; TOKEN_LEN];
        two_lines.extend_from_slice(b"\nextra");
        assert_ne!(trim_one_line_ending(&two_lines).len(), TOKEN_LEN);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_non_regular_files_and_wrong_modes() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("nwp-helper-{}-{unique}", std::process::id()));
        std::fs::create_dir(&directory).expect("create temp dir");
        let target = directory.join("target");
        let token = directory.join("token");
        std::fs::write(&target, format!("{}\n", "a".repeat(TOKEN_LEN))).expect("write target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("chmod target");
        symlink(&target, &token).expect("create symlink");
        assert!(super::read_token_at(&token).is_err());

        std::fs::remove_file(&token).expect("remove symlink");
        std::fs::create_dir(&token).expect("create directory at token path");
        assert!(super::read_token_at(&token).is_err());
        std::fs::remove_dir(&token).expect("remove directory");
        std::fs::write(&token, format!("{}\n", "a".repeat(TOKEN_LEN))).expect("write token");
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o640))
            .expect("chmod token");
        assert!(super::read_token_at(&token).is_err());

        std::fs::remove_dir_all(directory).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn accepts_secure_root_owned_token_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("nwp-helper-ok-{}-{unique}", std::process::id()));
        std::fs::create_dir(&directory).expect("create temp dir");
        let path = directory.join("token");
        let expected = "b".repeat(TOKEN_LEN);
        std::fs::write(&path, format!("{expected}\n")).expect("write token");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("chmod token");
        if std::fs::metadata(&directory)
            .expect("directory metadata")
            .uid()
            == 0
        {
            assert_eq!(
                super::read_token_at(&path).expect("read valid token"),
                expected
            );
        } else {
            assert!(super::read_token_at(&path).is_err());
        }
        std::fs::remove_dir_all(directory).expect("cleanup");
    }
}
