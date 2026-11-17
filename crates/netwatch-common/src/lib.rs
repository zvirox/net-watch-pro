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

/// Uplink/downlink byte pair (API.md §3 `usage.totals` frame).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectionTotals {
    /// Transmitted bytes (uplink).
    pub up: u64,
    /// Received bytes (downlink).
    pub down: u64,
}

/// Headline totals pushed on the `usage.totals` WS channel (API.md §3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTotals {
    /// Current local-day interface counter totals.
    pub today: DirectionTotals,
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

/// IP protocol of a connection episode (API.md C1 `protocol` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// Transmission Control Protocol.
    Tcp,
    /// User Datagram Protocol.
    Udp,
    /// Internet Control Message Protocol.
    Icmp,
    /// Anything else (SCTP, DCCP, …).
    Other,
}

impl Protocol {
    /// Canonical wire value (API.md C1).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Icmp => "icmp",
            Protocol::Other => "other",
        }
    }
}

/// App identity block in a live row (API.md C1 `app` object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveApp {
    /// Stable app id (FK to `apps.id`).
    pub id: i64,
    /// Display name.
    pub name: String,
    /// Icon reference; `null` → UI fallback container (DESIGN.md §6.2).
    pub icon: Option<String>,
}

/// One live connection row (API.md C1 shape; WS `live.connections`
/// snapshot rows use the same shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveConnectionRow {
    /// Stable episode id — the key for delta replacement (API.md §3).
    pub id: u64,
    /// Attributed app.
    pub app: LiveApp,
    /// OS pid snapshot at attribution.
    pub pid: u32,
    /// Process name snapshot (display fallback).
    pub process_name: Option<String>,
    /// IP protocol.
    pub protocol: Protocol,
    /// Local address.
    pub local_ip: String,
    /// Local port.
    pub local_port: u16,
    /// Remote address.
    pub remote_ip: String,
    /// Remote port.
    pub remote_port: u16,
    /// PTR/display name if resolved; never replaces `remote_ip`.
    pub remote_host: Option<String>,
    /// Current uplink rate (bytes/s, sampled 1 Hz).
    pub rate_up_bps: u64,
    /// Current downlink rate (bytes/s, sampled 1 Hz).
    pub rate_down_bps: u64,
    /// Total uplink bytes this episode.
    pub bytes_up: u64,
    /// Total downlink bytes this episode.
    pub bytes_down: u64,
}

/// Partial row for a delta `updated` entry (API.md §3: only volatile
/// counters are pushed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveUpdateRow {
    /// Episode id this update replaces.
    pub id: u64,
    /// Current uplink rate.
    pub rate_up_bps: u64,
    /// Current downlink rate.
    pub rate_down_bps: u64,
    /// Total uplink bytes so far.
    pub bytes_up: u64,
    /// Total downlink bytes so far.
    pub bytes_down: u64,
}

/// One `live.connections` delta frame payload (API.md §3). Replacement is
/// keyed by row `id`, not positional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveDelta {
    /// Rows first seen since the previous delta.
    pub added: Vec<LiveConnectionRow>,
    /// Counter updates for already-known rows.
    pub updated: Vec<LiveUpdateRow>,
    /// Episode ids removed since the previous delta (episode ended).
    pub removed: Vec<u64>,
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

    #[test]
    fn usage_totals_match_stream_frame_shape() {
        let totals = UsageTotals {
            today: DirectionTotals {
                up: 52_428_800,
                down: 934_000_000,
            },
        };
        assert_eq!(
            serde_json::to_value(&totals).unwrap(),
            serde_json::json!({ "today": { "up": 52_428_800, "down": 934_000_000 } })
        );
    }
}
