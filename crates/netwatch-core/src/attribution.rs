//! Per-app process attribution.
//!
//! F-05/D5a: cgroup-derived identity first, `exe_path` as fallback. The
//! collector can cache an identity at exec/connect time so short-lived
//! processes stay attributable after `/proc/<pid>` disappears.
//!
//! DATABASE.md §2.2: `app_key` is the unique identity key; `exe_path` is
//! non-unique provenance, so a new binary at the same path gets a new row.

use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// App identity key type.
pub type AppKey = u64;

/// Attribution confidence level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttributionConfidence {
    /// Exact match via cgroup identity at exec time.
    Exact,
    /// Heuristic match via `/proc` attributes.
    Heuristic,
    /// Could not determine attribution.
    Unknown,
}

/// How process identity was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IdentityKind {
    /// Captured at exec time via cgroup path (systemd `app-*.scope`).
    CgroupApp,
    /// Flatpak application.
    Flatpak,
    /// Snap package.
    Snap,
    /// `/proc/<pid>/exe` fallback.
    Exe,
}

/// Per-app identity information.
#[derive(Debug, Clone)]
pub struct AppIdentity {
    /// Stable identity key (cgroup path, flatpak, snap, or exe).
    pub app_key: AppKey,
    /// How this identity was captured.
    pub identity_kind: IdentityKind,
    /// Executable path (display/provenance only, never unique).
    pub exe_path: String,
    /// Hash of the binary when resolvable.
    pub exe_hash: Option<String>,
    /// Optional icon path; `None` uses the UI fallback.
    pub icon_path: Option<String>,
    /// Confidence in this attribution.
    pub confidence: AttributionConfidence,
    /// Epoch seconds of the first observation.
    pub first_seen_at: u64,
    /// Epoch seconds of the most recent observation.
    pub last_seen_at: u64,
}

/// Attribution engine: maps sockets to apps via cgroup identity.
#[derive(Default)]
pub struct Attribution {
    /// Socket cookie → `app_key` mapping.
    socket_to_app: HashMap<u64, AppKey>,
    /// PID → identity captured at exec/connect time (short-lived processes).
    pid_identity: HashMap<u32, AppKey>,
    /// Identity key → `app_key`, so non-unique `exe_path` values stay distinct.
    by_identity_key: HashMap<String, AppKey>,
    /// `app_key` → [`AppIdentity`] registry.
    apps: HashMap<AppKey, AppIdentity>,
    /// Next `app_key` to assign.
    next_app_key: AppKey,
}

impl Attribution {
    /// Create a new attribution engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_app_key: 1,
            ..Self::default()
        }
    }

    /// Attribute a socket connection to an app.
    ///
    /// Called when eBPF `cgroup_skb` or kprobe hooks observe a new socket.
    /// Cached exec-time identities win over a now-vanished `/proc` entry.
    pub fn attribute(&mut self, cookie: u64, pid: u32, exe_path: Option<&str>) -> AppKey {
        if let Some(&app_key) = self.socket_to_app.get(&cookie) {
            return app_key;
        }
        let app_key = if let Some(&app_key) = self.pid_identity.get(&pid) {
            self.touch(app_key);
            app_key
        } else if let Ok(content) = fs::read_to_string(format!("/proc/{pid}/cgroup")) {
            self.register_cgroup(&content, exe_path)
                .unwrap_or_else(|| self.register_exe(exe_path))
        } else {
            self.register_exe(exe_path)
        };
        self.socket_to_app.insert(cookie, app_key);
        app_key
    }

    /// Remember an identity observed at exec/connect time.
    ///
    /// The collector calls this before a short-lived process exits; later
    /// socket attribution uses this cache instead of reading `/proc`.
    #[must_use]
    pub fn cache_identity(
        &mut self,
        pid: u32,
        cgroup_content: &str,
        exe_path: Option<&str>,
    ) -> Option<AppKey> {
        let app_key = match self.register_cgroup(cgroup_content, exe_path) {
            Some(app_key) => app_key,
            None => self.register_exe(exe_path),
        };
        self.pid_identity.insert(pid, app_key);
        Some(app_key)
    }

    /// Look up the identity behind an `app_key` (for tests and diagnostics).
    #[must_use]
    pub fn identity(&self, app_key: AppKey) -> Option<&AppIdentity> {
        self.apps.get(&app_key)
    }

    fn register_cgroup(&mut self, content: &str, exe_path: Option<&str>) -> Option<AppKey> {
        let (kind, identity_key, display_name) = parse_cgroup_identity(content)?;
        let provenance = exe_path.unwrap_or(&display_name).to_string();
        Some(self.register(
            kind,
            identity_key,
            provenance,
            None,
            AttributionConfidence::Exact,
        ))
    }

    fn register_exe(&mut self, exe_path: Option<&str>) -> AppKey {
        let Some(exe_path) = exe_path else {
            return self.register(
                IdentityKind::Exe,
                "unknown".to_string(),
                "unknown".to_string(),
                None,
                AttributionConfidence::Unknown,
            );
        };
        let hash = fs::read(exe_path).ok().map(|bytes| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish().to_string()
        });
        let identity_key = match &hash {
            Some(hash) => format!("exe:{exe_path}#{hash}"),
            None => format!("exe:{exe_path}"),
        };
        self.register(
            IdentityKind::Exe,
            identity_key,
            exe_path.to_string(),
            hash,
            AttributionConfidence::Heuristic,
        )
    }

    fn register(
        &mut self,
        identity_kind: IdentityKind,
        identity_key: String,
        exe_path: String,
        exe_hash: Option<String>,
        confidence: AttributionConfidence,
    ) -> AppKey {
        if let Some(&app_key) = self.by_identity_key.get(&identity_key) {
            self.touch(app_key);
            return app_key;
        }
        let app_key = self.next_app_key;
        self.next_app_key += 1;
        let now = now_secs();
        self.apps.insert(
            app_key,
            AppIdentity {
                app_key,
                identity_kind,
                exe_path,
                exe_hash,
                icon_path: None,
                confidence,
                first_seen_at: now,
                last_seen_at: now,
            },
        );
        self.by_identity_key.insert(identity_key, app_key);
        app_key
    }

    fn touch(&mut self, app_key: AppKey) {
        if let Some(identity) = self.apps.get_mut(&app_key) {
            identity.last_seen_at = now_secs();
        }
    }
}

/// Extract a stable identity from cgroup v2 content.
///
/// Returns `(kind, identity key, display name)`. The paths mirror the three
/// cgroup-backed identity kinds in DATABASE.md §2.2.
fn parse_cgroup_identity(content: &str) -> Option<(IdentityKind, String, String)> {
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }
        let path = parts[2];
        let leaf = path.rsplit('/').next().unwrap_or(path);
        let leaf_without_scope = leaf.strip_suffix(".scope").unwrap_or(leaf);
        if leaf.contains("flatpak")
            || (leaf_without_scope.starts_with("app-") && leaf_without_scope.contains('.'))
        {
            let id = leaf_without_scope
                .strip_prefix("app-")
                .unwrap_or(leaf_without_scope);
            return Some((
                IdentityKind::Flatpak,
                format!("flatpak:{id}"),
                id.to_string(),
            ));
        }
        if leaf.contains("snap.") || path.contains("/snap/") {
            let name = leaf.trim_start_matches("snap.").trim_end_matches(".scope");
            return Some((IdentityKind::Snap, format!("snap:{name}"), name.to_string()));
        }
        let Some(name) = leaf.strip_suffix(".scope") else {
            continue;
        };
        let name = name
            .strip_prefix("app-")
            .unwrap_or(name)
            .strip_prefix("systemd-")
            .unwrap_or(name);
        return Some((
            IdentityKind::CgroupApp,
            format!("cgroup:{path}"),
            name.to_string(),
        ));
    }
    None
}

/// Get current time in seconds since epoch.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    const SYSTEMD_SCOPE: &str = "0::/system.slice/app-firefox.scope";
    const FLATPAK_SCOPE: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.mozilla.firefox-1234.scope";
    const SNAP_SCOPE: &str = "0::/system.slice/snap.firefox.firefox.service";

    struct ExeFixture {
        path: std::path::PathBuf,
    }

    impl ExeFixture {
        fn new(bytes: &[u8]) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("nwp-exe-{}-{nonce}", std::process::id()));
            fs::write(&path, bytes).unwrap();
            Self { path }
        }

        fn rewrite(&self, bytes: &[u8]) {
            fs::write(&self.path, bytes).unwrap();
        }
    }

    impl Drop for ExeFixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn attribute_new_socket_creates_app() {
        let mut attr = Attribution::new();
        let key1 = attr.attribute(1, 1234, Some("/usr/bin/firefox"));
        assert!(key1 > 0);
        assert_eq!(attr.attribute(1, 1234, Some("/usr/bin/firefox")), key1);
        assert_ne!(attr.attribute(2, 5678, Some("/usr/bin/chrome")), key1);
    }

    #[test]
    fn cgroup_first_beats_exe_fallback() {
        let mut attr = Attribution::new();
        let _ = attr.cache_identity(2213, SYSTEMD_SCOPE, Some("/usr/bin/firefox"));
        // Short-lived process: /proc is gone and the exe path is misleading.
        let key = attr.attribute(1, 2213, Some("/usr/bin/chrome"));
        let identity = attr.identity(key).unwrap();
        assert_eq!(identity.identity_kind, IdentityKind::CgroupApp);
        assert_eq!(identity.confidence, AttributionConfidence::Exact);
        assert_eq!(identity.exe_path, "/usr/bin/firefox");
    }

    #[test]
    fn flatpak_and_snap_get_distinct_identity_kinds() {
        let mut attr = Attribution::new();
        let flatpak = attr.cache_identity(1, FLATPAK_SCOPE, None).unwrap();
        let snap = attr.cache_identity(2, SNAP_SCOPE, None).unwrap();
        assert_eq!(
            attr.identity(flatpak).unwrap().identity_kind,
            IdentityKind::Flatpak
        );
        assert_eq!(
            attr.identity(snap).unwrap().identity_kind,
            IdentityKind::Snap
        );
        assert_ne!(flatpak, snap);
    }

    #[test]
    fn same_exe_path_with_new_binary_hash_is_a_new_app() {
        let fixture = ExeFixture::new(b"binary-v1");
        let mut attr = Attribution::new();
        let first = attr.attribute(1, 9001, Some(fixture.path.to_str().unwrap()));
        fixture.rewrite(b"binary-v2-different");
        let second = attr.attribute(2, 9002, Some(fixture.path.to_str().unwrap()));
        assert_ne!(first, second, "exe_path is provenance, not identity");
        assert_eq!(
            attr.identity(first).unwrap().confidence,
            AttributionConfidence::Heuristic
        );
    }

    #[test]
    fn unknown_process_is_honest() {
        let mut attr = Attribution::new();
        let key = attr.attribute(3, u32::MAX, None);
        let identity = attr.identity(key).unwrap();
        assert_eq!(identity.confidence, AttributionConfidence::Unknown);
        assert_eq!(identity.exe_path, "unknown");
    }
}
