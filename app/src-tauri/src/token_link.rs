//! Fixed-path invocation of the one-shot Polkit token helper.

use std::fmt;
use std::io::Read;
use std::process::{Command, Stdio};

use serde::Serialize;

const PKEXEC_PATH: &str = "/usr/bin/pkexec";
const HELPER_PATH: &str = "/usr/bin/netwatch-token-helper";
const TOKEN_LEN: usize = 64;

/// Safe error categories returned to the frontend without subprocess output.
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenError {
    /// User dismissed Polkit authentication.
    Cancelled,
    /// Authorization was unavailable or denied.
    AuthorizationFailed,
    /// pkexec or the installed helper is missing.
    HelperUnavailable,
    /// Helper output did not match the canonical token format.
    InvalidOutput,
    /// Process creation or output reading failed.
    ExecutionFailed,
}

impl fmt::Display for TokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Cancelled => "authorization was cancelled",
            Self::AuthorizationFailed => "authorization failed",
            Self::HelperUnavailable => "token helper is unavailable",
            Self::InvalidOutput => "token helper returned invalid data",
            Self::ExecutionFailed => "token helper could not be executed",
        };
        formatter.write_str(message)
    }
}

/// Invoke pkexec with fixed paths and validate its stdout token.
pub fn retrieve_token() -> Result<String, TokenError> {
    let mut child = Command::new(PKEXEC_PATH)
        .arg(HELPER_PATH)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                TokenError::HelperUnavailable
            } else {
                TokenError::ExecutionFailed
            }
        })?;

    let mut stdout = child.stdout.take().ok_or(TokenError::ExecutionFailed)?;
    let mut output = Vec::with_capacity(TOKEN_LEN + 2);
    stdout
        .by_ref()
        .take((TOKEN_LEN + 3) as u64)
        .read_to_end(&mut output)
        .map_err(|_| TokenError::ExecutionFailed)?;
    if output.len() > TOKEN_LEN + 2 {
        drop(stdout);
        let _ = child.kill();
        let _ = child.wait();
        return Err(TokenError::InvalidOutput);
    }
    drop(stdout);
    let status = child.wait().map_err(|_| TokenError::ExecutionFailed)?;
    if !status.success() {
        return Err(match status.code() {
            Some(126) => TokenError::Cancelled,
            Some(127) => TokenError::AuthorizationFailed,
            _ => TokenError::ExecutionFailed,
        });
    }

    let token = output.strip_suffix(b"\n").unwrap_or(&output);
    if token.len() != TOKEN_LEN || !token.iter().all(u8::is_ascii_hexdigit) {
        return Err(TokenError::InvalidOutput);
    }
    String::from_utf8(token.to_vec()).map_err(|_| TokenError::InvalidOutput)
}
