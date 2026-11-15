//! Environment configuration — the closed list in ARCHITECTURE.md §8.3
//! (mirrored in `docs/env-vars.md` and `.env.example`). Adding a variable
//! requires updating ARCHITECTURE.md first (AGENTS.md §5).

use std::net::SocketAddr;
use std::path::PathBuf;

/// Resolved daemon configuration (Phase 1 subset of §8.3; the retention and
/// eBPF switches are consumed in Phase 2).
#[derive(Debug, Clone)]
pub struct Config {
    /// DB + state directory (systemd sets it in production).
    pub data_dir: PathBuf,
    /// Loopback-only API bind address.
    pub api_addr: SocketAddr,
    /// Where the per-boot token is written (mode 0600).
    pub token_file: PathBuf,
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

        let addr =
            std::env::var("NETWATCH_API_ADDR").unwrap_or_else(|_| "127.0.0.1:7897".to_string());

        let api_addr: SocketAddr = addr
            .parse()
            .map_err(|_| format!("NETWATCH_API_ADDR: invalid socket address '{addr}'"))?;
        if !api_addr.ip().is_loopback() {
            // §5.3: loopback-only bind is a contract, not a preference.
            return Err(format!(
                "NETWATCH_API_ADDR must be loopback-only, got '{addr}'"
            ));
        }

        // Packaged release builds always use the helper's fixed production
        // path. Debug builds may override it for isolated local tests.
        let token_file = if cfg!(debug_assertions) {
            std::env::var("NETWATCH_API_TOKEN_FILE").map_or_else(
                |_| PathBuf::from("/var/lib/netwatch-pro/api-token"),
                PathBuf::from,
            )
        } else {
            PathBuf::from("/var/lib/netwatch-pro/api-token")
        };

        Ok(Self {
            data_dir,
            api_addr,
            token_file,
        })
    }
}
