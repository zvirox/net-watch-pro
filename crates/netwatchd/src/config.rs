//! Environment configuration — the closed list in ARCHITECTURE.md §8.3
//! (mirrored in `docs/env-vars.md` and `.env.example`). Adding a variable
//! requires updating ARCHITECTURE.md first (AGENTS.md §5).

use std::path::PathBuf;

/// Resolved daemon configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// DB + state directory (systemd sets it in production).
    pub data_dir: PathBuf,
    /// Unix domain socket path for the API (NETWATCH_API_SOCKET).
    pub socket_path: PathBuf,
}

impl Config {
    /// Read config from the environment, applying §8.3 defaults.
    ///
    /// Fail-fast with a distinct message per class (ARCHITECTURE.md §7.1):
    /// startup is the one place a crash is acceptable.
    pub fn from_env() -> Result<Self, String> {
        let data_dir = match std::env::var("NETWATCH_DATA_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from("/var/lib/netwatch-pro"),
        };

        let socket_path = match std::env::var("NETWATCH_API_SOCKET") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from("/run/netwatch-pro/api.sock"),
        };

        Ok(Self {
            data_dir,
            socket_path,
        })
    }
}
