//! Environment configuration — the closed list in ARCHITECTURE.md §8.3
//! (mirrored in `docs/env-vars.md` and `.env.example`). Adding a variable
//! requires updating ARCHITECTURE.md first (AGENTS.md §5).

use std::path::PathBuf;

/// Resolved daemon configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// DB + state directory (systemd sets it in production).
    pub data_dir: PathBuf,
    /// Unix domain socket path for the API (`NETWATCH_API_SOCKET`).
    pub socket_path: PathBuf,
    /// Group allowed to connect to the socket (`NETWATCH_API_GID`, then
    /// `SUDO_GID`). `None` leaves the socket group as root.
    pub api_gid: Option<u32>,
}

impl Config {
    /// Read config from the environment, applying §8.3 defaults.
    ///
    /// Fail-fast with a distinct message per class (ARCHITECTURE.md §7.1):
    /// startup is the one place a crash is acceptable.
    // Keep the Result shape for the documented config-error exit class (§7.1)
    // even though the current closed key list has no invalid combinations.
    #[allow(clippy::unnecessary_wraps)]
    pub fn from_env() -> Result<Self, String> {
        let data_dir = match std::env::var("NETWATCH_DATA_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from("/var/lib/netwatch-pro"),
        };

        let socket_path = match std::env::var("NETWATCH_API_SOCKET") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from("/run/netwatch-pro/api.sock"),
        };

        // `SUDO_GID` is supplied by sudo when a developer starts the daemon
        // from their own session; packages set `NETWATCH_API_GID` explicitly.
        let api_gid = std::env::var("NETWATCH_API_GID")
            .ok()
            .and_then(|value| value.parse().ok())
            .or_else(|| {
                std::env::var("SUDO_GID")
                    .ok()
                    .and_then(|value| value.parse().ok())
            });

        Ok(Self {
            data_dir,
            socket_path,
            api_gid,
        })
    }
}
