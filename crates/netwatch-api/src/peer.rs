//! Peer-credential (`SO_PEERCRED`) enforcement for the API socket (D1a).
//!
//! Before this module the socket was protected only by its `0660` group
//! ownership: the allowed-UID file written during onboarding was never read,
//! so every member of the socket's group had full API access. This listener
//! checks the kernel-reported UID of each connecting process and drops the
//! connection before any HTTP is spoken when the UID is not allowed.
//!
//! Allowed UIDs:
//! - root (0) and the daemon's own UID (always),
//! - every UID listed in the allowlist file (default
//!   `/etc/netwatch-pro/allowed-uids`, override `NETWATCH_ALLOWED_UIDS_FILE`),
//! - `SUDO_UID` when a developer starts the daemon with sudo (the same
//!   convention `SUDO_GID` already follows for the socket group).
//!
//! The file is re-read on every accepted connection, so onboarding changes
//! apply without a restart. A missing or unreadable file means "no extra
//! UIDs": fail closed, never open.

use std::path::PathBuf;
use std::time::Duration;

use tokio::io;
use tokio::net::{UnixListener, UnixStream, unix::SocketAddr};

/// Default allowlist location (ARCHITECTURE.md §5.3).
pub const DEFAULT_ALLOWED_UIDS_FILE: &str = "/etc/netwatch-pro/allowed-uids";
/// Environment override for the allowlist path (tests, unusual layouts).
pub const ALLOWED_UIDS_ENV: &str = "NETWATCH_ALLOWED_UIDS_FILE";

/// Who may talk to the API.
#[derive(Debug, Clone)]
pub struct PeerPolicy {
    file: PathBuf,
    own_uid: u32,
    sudo_uid: Option<u32>,
}

impl PeerPolicy {
    /// Build the policy from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        let file = std::env::var_os(ALLOWED_UIDS_ENV)
            .map_or_else(|| PathBuf::from(DEFAULT_ALLOWED_UIDS_FILE), PathBuf::from);
        Self {
            file,
            own_uid: own_uid(),
            sudo_uid: std::env::var("SUDO_UID")
                .ok()
                .and_then(|value| value.trim().parse().ok()),
        }
    }

    /// Explicit constructor for tests.
    #[must_use]
    pub fn new(file: PathBuf, own_uid: u32, sudo_uid: Option<u32>) -> Self {
        Self {
            file,
            own_uid,
            sudo_uid,
        }
    }

    /// Whether a peer with this UID may use the API.
    #[must_use]
    pub fn allows(&self, uid: u32) -> bool {
        if uid == 0 || uid == self.own_uid || self.sudo_uid == Some(uid) {
            return true;
        }
        match std::fs::read_to_string(&self.file) {
            Ok(text) => parse_uid_list(&text).contains(&uid),
            Err(_) => false,
        }
    }
}

/// The daemon's own effective UID, read without `unsafe`/libc.
fn own_uid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    // `/proc/self` is owned by the process's effective UID.
    std::fs::metadata("/proc/self").map_or(u32::MAX, |meta| meta.uid())
}

/// Parse one UID per line; `#` starts a comment; malformed lines are ignored
/// (a typo can never widen access, only narrow it).
#[must_use]
pub fn parse_uid_list(text: &str) -> Vec<u32> {
    text.lines()
        .filter_map(|line| {
            let line = line.split('#').next().unwrap_or_default().trim();
            line.parse::<u32>().ok()
        })
        .collect()
}

/// A `UnixListener` that only yields connections from allowed peers.
#[derive(Debug)]
pub struct PeerCredListener {
    inner: UnixListener,
    policy: PeerPolicy,
}

impl PeerCredListener {
    /// Wrap a bound listener.
    #[must_use]
    pub fn new(inner: UnixListener, policy: PeerPolicy) -> Self {
        Self { inner, policy }
    }
}

impl axum::serve::Listener for PeerCredListener {
    type Io = UnixStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.inner.accept().await {
                Ok(pair) => pair,
                Err(error) => {
                    // Mirrors axum's own accept loop: log, back off, retry.
                    tracing::warn!(%error, "API socket accept failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            match stream.peer_cred() {
                Ok(cred) if self.policy.allows(cred.uid()) => return (stream, addr),
                Ok(cred) => {
                    tracing::warn!(uid = cred.uid(), "API connection rejected: UID not allowed");
                }
                Err(error) => {
                    tracing::warn!(%error, "API connection rejected: no peer credentials");
                }
            }
            // `stream` drops here: the peer sees an immediate close.
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, body: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("nwp-peer-{name}-{}", std::process::id()));
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn uid_list_ignores_comments_blanks_and_garbage() {
        let parsed =
            parse_uid_list("1000\n# admin\n\n  1001  # laptop user\nabc\n-5\n99999999999\n");
        assert_eq!(parsed, vec![1000, 1001]);
    }

    #[test]
    fn root_self_and_sudo_uid_are_always_allowed() {
        let policy = PeerPolicy::new(PathBuf::from("/nonexistent/allowed-uids"), 977, Some(1000));
        assert!(policy.allows(0));
        assert!(policy.allows(977));
        assert!(policy.allows(1000));
        assert!(!policy.allows(1001));
    }

    #[test]
    fn missing_file_fails_closed() {
        let policy = PeerPolicy::new(PathBuf::from("/nonexistent/allowed-uids"), 977, None);
        assert!(!policy.allows(1000));
    }

    #[test]
    fn listed_uids_are_allowed_and_file_is_reread() {
        let path = temp_file("reread", "1000\n");
        let policy = PeerPolicy::new(path.clone(), 977, None);
        assert!(policy.allows(1000));
        assert!(!policy.allows(1001));
        std::fs::write(&path, "1000\n1001\n").unwrap();
        assert!(
            policy.allows(1001),
            "onboarding edits apply without restart"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[tokio::test]
    async fn listener_drops_disallowed_peers_and_accepts_allowed_ones() {
        use axum::serve::Listener;
        use tokio::io::AsyncWriteExt;

        let dir = std::env::temp_dir().join(format!("nwp-peer-sock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("api.sock");
        let _ = std::fs::remove_file(&sock);
        let me = own_uid();

        // Policy that does NOT include this process's UID: the connection must
        // be dropped, so accept() never returns within the timeout.
        let policy = PeerPolicy::new(PathBuf::from("/nonexistent"), me.wrapping_add(1), None);
        let mut listener = PeerCredListener::new(UnixListener::bind(&sock).unwrap(), policy);
        let client = tokio::spawn({
            let sock = sock.clone();
            async move {
                let mut stream = UnixStream::connect(&sock).await.unwrap();
                let _ = stream.write_all(b"x").await;
                stream
            }
        });
        let rejected = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
        // Root is always allowed, so when the suite runs as root this peer is
        // (correctly) accepted; only assert the rejection for non-root runs.
        if me != 0 {
            assert!(rejected.is_err(), "disallowed UID was accepted");
        }
        drop(client);

        // Policy that allows this process: accept() returns promptly.
        let _ = std::fs::remove_file(&sock);
        let policy = PeerPolicy::new(PathBuf::from("/nonexistent"), me, None);
        let mut listener = PeerCredListener::new(UnixListener::bind(&sock).unwrap(), policy);
        let client = tokio::spawn({
            let sock = sock.clone();
            async move { UnixStream::connect(&sock).await.unwrap() }
        });
        let accepted = tokio::time::timeout(Duration::from_secs(2), listener.accept()).await;
        assert!(accepted.is_ok(), "allowed UID was not accepted");
        drop(client);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
