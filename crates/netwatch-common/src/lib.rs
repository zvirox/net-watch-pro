//! Shared types, IDs, error codes, and API DTOs for Net Watch Pro.
//!
//! ARCHITECTURE.md §4: shared code lives here — never copied between crates.
//! The error codes and status model below are part of the wire contract
//! (API.md §1.3/§2.1); changing them requires an API.md version bump.

// Tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping in test
// assertions is standard and expected.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use serde::{Deserialize, Serialize};

/// Stable error codes over the API (API.md §1.3 / ARCHITECTURE.md §7.2).
///
/// Serialized as the string form of the variant; never invent new codes
/// without updating API.md first (AGENTS.md §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Daemon not reachable on the loopback port.
    DaemonUnreachable,
    /// Missing/invalid bearer token (HTTP 401, WS close 4401).
    AuthInvalid,
    /// eBPF program failed to attach (503 degraded).
    EbpfAttachFailed,
    /// Kernel lacks required eBPF/BTF support (503 degraded).
    EbpfUnsupportedKernel,
    /// Not permitted (systemd/policy), distinct from auth.
    PermissionDenied,
    /// SQLite failure (500).
    DbError,
    /// Malformed query or unknown channel (400).
    QueryInvalid,
    /// Freemium gate (403).
    ExportProRequired,
    /// Offline license key rejected (403).
    LicenseInvalid,
    /// Rate limit exceeded (429).
    RateLimited,
    /// Resource not found (404).
    NotFound,
    /// Concurrent modification (409).
    Conflict,
    /// Method not allowed (405).
    MethodNotAllowed,
}

/// Daemon lifecycle state — 1:1 with the DESIGN.md §7.5 chip states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonState {
    /// Collecting and recording.
    Monitoring,
    /// Partial capability (e.g. collector off, some interfaces excluded).
    Degraded,
    /// Not collecting.
    Stopped,
    /// Boot sequence in progress.
    Starting,
}

/// eBPF collector state (API.md S2 `ebpf` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EbpfState {
    /// Programs attached.
    Attached,
    /// Attach failed — degraded data path.
    Failed,
    /// Kernel unsupported (missing BTF, old kernel).
    Unsupported,
    /// Not yet attempted (daemon starting).
    Unattached,
}

/// Snapshot served by `GET /status` and pushed on the `status` WS channel
/// (API.md S2 / §3). `ws_clients`/`uptime_s`/`drops` are filled by the API
/// layer at serve time; the source only publishes the observed facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSnapshot {
    /// Daemon lifecycle state (1:1 with the DESIGN.md §7.5 chip).
    pub daemon: DaemonState,
    /// eBPF collector state.
    pub ebpf: EbpfState,
    /// Whether traffic is being recorded into SQLite.
    pub recording: bool,
}

/// WS channel names (API.md §3, dot-namespaced kebab-case).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// Live connection snapshot + deltas (Phase 2 data; skeleton in Phase 1).
    #[serde(rename = "live.connections")]
    LiveConnections,
    /// Daemon status updates on change.
    #[serde(rename = "status")]
    Status,
    /// Headline usage totals.
    #[serde(rename = "usage.totals")]
    UsageTotals,
}

impl Channel {
    /// Canonical wire name (API.md §3 table).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::LiveConnections => "live.connections",
            Channel::Status => "status",
            Channel::UsageTotals => "usage.totals",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_to_contract_strings() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::AuthInvalid).unwrap(),
            "\"AUTH_INVALID\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::EbpfUnsupportedKernel).unwrap(),
            "\"EBPF_UNSUPPORTED_KERNEL\""
        );
    }

    #[test]
    fn status_snapshot_serializes_lowercase() {
        let s = StatusSnapshot {
            daemon: DaemonState::Monitoring,
            ebpf: EbpfState::Attached,
            recording: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            json,
            r#"{"daemon":"monitoring","ebpf":"attached","recording":true}"#
        );
    }

    #[test]
    fn channels_use_contract_names() {
        assert_eq!(Channel::LiveConnections.as_str(), "live.connections");
        assert_eq!(Channel::UsageTotals.as_str(), "usage.totals");
    }
}
