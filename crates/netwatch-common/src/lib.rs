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
    /// No collector is running; PC rate sampling still works (Phase A).
    #[serde(rename = "no_collector")]
    NoCollector,
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

/// One interface's live rate, in integer bits per second (API.md §3 `net.rates`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceRate {
    /// sysfs interface name.
    pub name: String,
    /// Download (rx) bits per second.
    pub down_bps: u64,
    /// Upload (tx) bits per second.
    pub up_bps: u64,
}

/// Live per-app byte/rate row for the `live.apps` stream (API.md §3, Phase C).
///
/// `cgroup_id` is the socket's cgroup id (`bpf_skb_cgroup_id`); it is the
/// grouping key, not the PID. Display names are resolved in userspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUsage {
    /// Socket cgroup id — the app identity grouping key.
    pub cgroup_id: u64,
    /// Current download bits per second (decimal).
    pub down_bps: u64,
    /// Current upload bits per second (decimal).
    pub up_bps: u64,
    /// Cumulative downloaded bytes for this cgroup.
    pub bytes_down: u64,
    /// Cumulative uploaded bytes for this cgroup.
    pub bytes_up: u64,
    /// Sample timestamp (epoch seconds).
    pub ts: u64,
}

/// Live PC rate snapshot pushed on the `net.rates` WS channel (API.md §3).
///
/// Rates are decimal bits per second; `ts` is epoch seconds of the sample.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateSnapshot {
    /// Sample timestamp (epoch seconds).
    pub ts: u64,
    /// Total download bits per second across non-excluded interfaces.
    pub down_bps: u64,
    /// Total upload bits per second across non-excluded interfaces.
    pub up_bps: u64,
    /// Per-interface rates (only interfaces seen in the latest sample).
    pub interfaces: Vec<InterfaceRate>,
}

/// One total-rate sample in the last-hour history (API.md U4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateSample {
    /// Sample timestamp (epoch seconds).
    pub ts: u64,
    /// Total download bits per second.
    pub down_bps: u64,
    /// Total upload bits per second.
    pub up_bps: u64,
}

impl From<&RateSnapshot> for RateSample {
    fn from(snapshot: &RateSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            down_bps: snapshot.down_bps,
            up_bps: snapshot.up_bps,
        }
    }
}

/// Capacity of the in-memory last-hour rate ring (60 × 1 min = 1 hour).
pub const RATE_HISTORY_CAPACITY: usize = 60;

/// Bounded, thread-safe ring of the most recent total-rate samples.
///
/// One point per minute (a newer sample in the current minute replaces the
/// previous one), so 60 points cover the last hour at a resolution smoother
/// than the four 15-minute rollups (owner L-OQ1, `docs/LIVE-THROUGHPUT-PLAN.md`
/// §9). Process memory only — lost on restart by design. The 1 Hz rate task
/// feeds it; the API serves a snapshot on U4. No new dependency: a plain
/// `std::sync::Mutex` guards a small `VecDeque`.
#[derive(Debug, Default)]
pub struct RateHistory {
    inner: std::sync::Mutex<std::collections::VecDeque<RateSample>>,
}

impl RateHistory {
    /// Append a sample at one-minute resolution, dropping the oldest when
    /// full. A newer sample in the same minute replaces the previous point so
    /// the current minute stays fresh without growing the buffer.
    pub fn push(&self, sample: RateSample) {
        // Poisoning here cannot corrupt state — recover the guard rather than
        // panic in a daemon path (ARCHITECTURE.md §7.1).
        let mut ring = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(last) = ring.back_mut()
            && last.ts / 60 == sample.ts / 60
        {
            *last = sample;
            return;
        }
        if ring.len() == RATE_HISTORY_CAPACITY {
            ring.pop_front();
        }
        ring.push_back(sample);
    }

    /// Copy the retained samples, oldest first.
    #[must_use]
    pub fn samples(&self) -> Vec<RateSample> {
        let ring = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ring.iter().copied().collect()
    }

    /// Number of retained samples (0–60).
    #[must_use]
    pub fn len(&self) -> usize {
        let ring = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ring.len()
    }

    /// True while no sample has been recorded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
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
    /// 1 Hz live PC network rates.
    #[serde(rename = "net.rates")]
    Rates,
}

impl Channel {
    /// Canonical wire name (API.md §3 table).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::LiveConnections => "live.connections",
            Channel::Status => "status",
            Channel::UsageTotals => "usage.totals",
            Channel::Rates => "net.rates",
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

/// Severity of a daemon state/event log entry (ARCHITECTURE.md §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    /// Informational state change.
    Info,
    /// Degraded state; monitoring continues.
    Warn,
    /// Failure that needs diagnosis.
    Error,
}

impl EventLevel {
    /// Canonical database value (DATABASE.md §2.8).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One immutable diagnostics event (DATABASE.md §2.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonEvent {
    /// Severity.
    pub level: EventLevel,
    /// Stable taxonomy code, e.g. `EBPF_ATTACH_FAILED`.
    pub code: String,
    /// Human-readable explanation (≤ 2000 chars).
    pub message: String,
    /// Optional structured context as JSON text.
    pub detail_json: Option<String>,
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
        assert_eq!(Channel::Rates.as_str(), "net.rates");
    }

    #[test]
    fn no_collector_status_serializes_to_contract_string() {
        let value = serde_json::to_string(&EbpfState::NoCollector).unwrap();
        assert_eq!(value, "\"no_collector\"");
    }

    #[test]
    fn rate_history_is_bounded_and_oldest_first() {
        let history = RateHistory::default();
        assert!(history.is_empty());
        let base = 1_700_000_000;
        for minute in 0..(RATE_HISTORY_CAPACITY as u64 + 5) {
            history.push(RateSample {
                ts: base + minute * 60,
                down_bps: minute,
                up_bps: 0,
            });
        }
        let samples = history.samples();
        assert_eq!(samples.len(), RATE_HISTORY_CAPACITY, "one point per minute");
        assert_eq!(samples[0].ts, base + 5 * 60, "oldest minutes dropped");
        assert_eq!(
            samples[RATE_HISTORY_CAPACITY - 1].ts,
            base + (RATE_HISTORY_CAPACITY as u64 + 4) * 60
        );
    }

    #[test]
    fn rate_history_keeps_one_point_per_minute() {
        let history = RateHistory::default();
        history.push(RateSample {
            ts: 1_700_000_000,
            down_bps: 10,
            up_bps: 0,
        });
        // Same minute, one second later: replace rather than append.
        history.push(RateSample {
            ts: 1_700_000_001,
            down_bps: 20,
            up_bps: 0,
        });
        assert_eq!(history.len(), 1);
        assert_eq!(history.samples()[0].down_bps, 20);
        // Next minute: a new point.
        history.push(RateSample {
            ts: 1_700_000_060,
            down_bps: 30,
            up_bps: 0,
        });
        assert_eq!(history.len(), 2);
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
            serde_json::to_value(totals).unwrap(),
            serde_json::json!({ "today": { "up": 52_428_800, "down": 934_000_000 } })
        );
    }
}
