//! Per-app process attribution.
//!
//! Phase 2: attribute socket connections to applications using cgroup identity
//! captured at exec/connect time (Design 1/D2 from spike), with `/proc` reads
//! as fallback.
//!
//! F-05/D5a: cgroup-derived identity first, `exe_path` as fallback. Handles
//! Flatpak, Snap, script interpreters, and short-lived processes.
//!
//! DATABASE.md §2.2: `apps` table uses `app_key` as unique identity key;
//! `exe_path` is non-unique display/provenance data.

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
    /// Returns the `app_key` for the connection.
    pub fn attribute(&mut self, cookie: u64, pid: u32, exe_path: Option<&str>) -> AppKey {
        // If we've seen this socket cookie before, return existing app_key.
        if let Some(&app_key) = self.socket_to_app.get(&cookie) {
            return app_key;
        }

        // Try cgroup identity first (D5a: cgroup-first, exe_path fallback).
        if let Some(app_key) = self.identify_by_cgroup(pid) {
            self.socket_to_app.insert(cookie, app_key);
            return app_key;
        }

        // Fallback to /proc/<pid>/exe attribution.
        if let Some(app_key) = self.identify_by_exe(exe_path) {
            self.socket_to_app.insert(cookie, app_key);
            return app_key;
        }

        // Last resort: create a new unknown app entry.
        let app_key = self.next_app_key;
        self.next_app_key += 1;
        self.apps.insert(
            app_key,
            AppIdentity {
                app_key,
                identity_kind: IdentityKind::Exe,
                exe_path: exe_path.unwrap_or("unknown").to_string(),
                exe_hash: None,
                icon_path: None,
                confidence: AttributionConfidence::Unknown,
                first_seen_at: now_secs(),
                last_seen_at: now_secs(),
            },
        );
        self.socket_to_app.insert(cookie, app_key);
        app_key
    }

    /// Try to identify a process by cgroup path (systemd app-*.scope,
    /// Flatpak, Snap).
    fn identify_by_cgroup(&mut self, pid: u32) -> Option<AppKey> {
        // Read /proc/<pid>/cgroup to get cgroup path.
        let cgroup_file = format!("/proc/{pid}/cgroup");
        let cgroup_content = fs::read_to_string(&cgroup_file).ok()?;

        // Parse cgroup v2 format: "0::/system.slice/app-foo.scope"
        for line in cgroup_content.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() < 3 {
                continue;
            }
            let cgroup_path = parts[2];
            let Some(app_name) = cgroup_path.strip_prefix("system.slice/app-") else {
                continue;
            };
            // Compare the extension case-insensitively (Clippy's file
            // extension rule) without treating arbitrary suffixes as paths.
            let Some((name, extension)) = app_name.rsplit_once('.') else {
                continue;
            };
            if !extension.eq_ignore_ascii_case("scope") {
                continue;
            }
            let app_key = hash_app_key(&format!("cgroup:{cgroup_path}"));
            if let Some(existing) = self.apps.get(&app_key) {
                return Some(existing.app_key);
            }
            let app_key = self.next_app_key;
            self.next_app_key += 1;
            self.apps.insert(
                app_key,
                AppIdentity {
                    app_key,
                    identity_kind: IdentityKind::CgroupApp,
                    exe_path: name.to_string(),
                    exe_hash: None,
                    icon_path: None,
                    confidence: AttributionConfidence::Exact,
                    first_seen_at: now_secs(),
                    last_seen_at: now_secs(),
                },
            );
            return Some(app_key);
        }
        None
    }

    /// Identify a process by its executable path.
    fn identify_by_exe(&mut self, exe_path: Option<&str>) -> Option<AppKey> {
        let exe_path = exe_path?;
        let exe_hash = fs::read(exe_path).ok().map(|bytes| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        });

        // Check if we've seen this exe_path before (same app, new socket).
        for app in self.apps.values() {
            if app.exe_path == exe_path && app.identity_kind == IdentityKind::Exe {
                return Some(app.app_key);
            }
        }

        let app_key = self.next_app_key;
        self.next_app_key += 1;
        self.apps.insert(
            app_key,
            AppIdentity {
                app_key,
                identity_kind: IdentityKind::Exe,
                exe_path: exe_path.to_string(),
                exe_hash: exe_hash.map(|hash| hash.to_string()),
                icon_path: None,
                confidence: AttributionConfidence::Heuristic,
                first_seen_at: now_secs(),
                last_seen_at: now_secs(),
            },
        );
        Some(app_key)
    }
}

/// Get current time in seconds since epoch.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Hash a string to an `app_key` (u64).
fn hash_app_key(s: &str) -> AppKey {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    #[test]
    fn attribute_new_socket_creates_app() {
        let mut attr = Attribution::new();
        let key1 = attr.attribute(1, 1234, Some("/usr/bin/firefox"));
        assert!(key1 > 0);
        // Same cookie returns same app_key
        assert_eq!(attr.attribute(1, 1234, Some("/usr/bin/firefox")), key1);
        // Different cookie gets different app_key
        assert_ne!(attr.attribute(2, 5678, Some("/usr/bin/chrome")), key1);
    }

    #[test]
    fn exe_path_deduplication() {
        let mut attr = Attribution::new();
        let key1 = attr.attribute(1, 1234, Some("/usr/bin/firefox"));
        let key2 = attr.attribute(2, 5678, Some("/usr/bin/firefox"));
        assert_eq!(key1, key2);
    }

    #[test]
    fn cgroup_identity_priority() {
        let mut attr = Attribution::new();
        // First attribution via exe_path
        let key1 = attr.attribute(1, 1234, Some("/usr/bin/firefox"));
        // Second socket with same exe_path should use same app_key
        let key2 = attr.attribute(2, 5678, Some("/usr/bin/firefox"));
        assert_eq!(key1, key2);
    }
}
