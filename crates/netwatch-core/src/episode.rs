//! Connection-episode store — the daemon's in-memory open-episode state.
//!
//! ARCHITECTURE.md §5.1 step 3 + F-09: open-episode state is kept in memory;
//! rows persist on episode close and at a checkpoint interval (~60 s crash
//! loss window). This module is the pure correctness core: events in,
//! snapshots/deltas/checkpoint rows out, no I/O and no clock reads (the
//! caller stamps every event with epoch seconds).
//!
//! Episodes are keyed by a caller-supplied collector handle (`key` — the
//! socket cookie once the eBPF collector lands). Episode ids are assigned
//! here so the live view (WS snapshot + deltas) and SQLite rows share one
//! id space; the caller seeds `start_id` above `MAX(connections.id)`.

use std::collections::HashMap;

use netwatch_common::{LiveApp, LiveConnectionRow, LiveDelta, LiveUpdateRow, Protocol};

/// One collector observation of a new connection (attributed upstream by
/// the attribution engine — P2.3; tests feed synthetic identities).
#[derive(Debug, Clone)]
pub struct EpisodeOpen {
    /// Stable collector handle (socket cookie).
    pub key: u64,
    /// Attributed app (FK id + display name).
    pub app_id: i64,
    pub app_name: String,
    pub icon: Option<String>,
    /// OS pid snapshot at attribution.
    pub pid: u32,
    pub process_name: Option<String>,
    pub protocol: Protocol,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub remote_host: Option<String>,
    /// Interface the episode traverses, when known (FK `interfaces.id`).
    pub interface_id: Option<i32>,
    /// Epoch seconds when the collector saw the episode open.
    pub at: u64,
}

/// One collector event for an existing or new episode.
#[derive(Debug, Clone)]
pub enum EpisodeEvent {
    /// New connection observed.
    Open(EpisodeOpen),
    /// Byte/packet counters advanced for a known episode.
    Bytes {
        key: u64,
        bytes_up: u64,
        bytes_down: u64,
        packets_up: u64,
        packets_down: u64,
        at: u64,
    },
    /// Episode ended (socket closed).
    Close { key: u64, at: u64 },
}

/// A row as persisted to SQLite (connections table). One id space with the
/// live view: the store assigns `id`, the writer inserts it explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRow {
    pub id: u64,
    pub app_id: i64,
    pub interface_id: Option<i32>,
    pub pid: u32,
    pub process_name: Option<String>,
    pub protocol: Protocol,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub remote_host: Option<String>,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
    pub first_seen_at: u64,
    pub last_seen_at: u64,
    pub is_open: bool,
}

/// What one 1 Hz sample produces: the wire delta plus the close-flushes
/// (closed episodes persist immediately per F-09).
#[derive(Debug, Default)]
pub struct Sample {
    pub delta: LiveDelta,
    pub closed_rows: Vec<EpisodeRow>,
}

/// Internal episode state.
struct Episode {
    key: u64,
    id: u64,
    app_id: i64,
    app_name: String,
    icon: Option<String>,
    pid: u32,
    process_name: Option<String>,
    protocol: Protocol,
    local_ip: String,
    local_port: u16,
    remote_ip: String,
    remote_port: u16,
    remote_host: Option<String>,
    interface_id: Option<i32>,
    bytes_up: u64,
    bytes_down: u64,
    packets_up: u64,
    packets_down: u64,
    first_seen_at: u64,
    last_seen_at: u64,
    /// Bytes at the previous sample (rate numerator).
    sampled_up: u64,
    sampled_down: u64,
    sampled_at: u64,
    /// Rates from the previous sample (kept for the row snapshot).
    rate_up: u64,
    rate_down: u64,
    /// Set when a `Close` arrived; cleared at the next sample, which emits
    /// the row into `closed_rows` and removes it from the live view.
    closed: bool,
    /// False until the episode's first sample ran (first sighting goes to
    /// delta `added`, later samples to `updated`).
    ever_sampled: bool,
}

/// In-memory episode state for all open (and just-closed) connections.
pub struct EpisodeStore {
    /// Collector handle → episode.
    by_key: HashMap<u64, Episode>,
    /// Episodes opened since the previous sample (for delta `added`).
    next_id: u64,
}

impl EpisodeStore {
    /// Create a store whose episode ids start above the highest persisted
    /// connection id (`MAX(connections.id) + 1` at boot) so live ids and DB
    /// rows never collide across restarts.
    #[must_use]
    pub fn new(start_id: u64) -> Self {
        Self {
            by_key: HashMap::new(),
            next_id: start_id,
        }
    }

    /// Apply one collector event. Unknown `key`s on Bytes/Close are ignored
    /// (the collector may flush a final delta after the store already
    /// removed the episode; counters lost that way are within the F-09
    /// crash-loss window semantics).
    pub fn apply(&mut self, event: EpisodeEvent) {
        match event {
            EpisodeEvent::Open(open) => {
                // Re-open of a live key is a no-op (idempotent collector
                // replay must not reset counters).
                if self.by_key.contains_key(&open.key) {
                    return;
                }
                let id = self.next_id;
                self.next_id += 1;
                self.by_key.insert(
                    open.key,
                    Episode {
                        key: open.key,
                        id,
                        app_id: open.app_id,
                        app_name: open.app_name,
                        icon: open.icon,
                        pid: open.pid,
                        process_name: open.process_name,
                        protocol: open.protocol,
                        local_ip: open.local_ip,
                        local_port: open.local_port,
                        remote_ip: open.remote_ip,
                        remote_port: open.remote_port,
                        remote_host: open.remote_host,
                        interface_id: open.interface_id,
                        bytes_up: 0,
                        bytes_down: 0,
                        packets_up: 0,
                        packets_down: 0,
                        first_seen_at: open.at,
                        last_seen_at: open.at,
                        sampled_up: 0,
                        sampled_down: 0,
                        sampled_at: open.at,
                        rate_up: 0,
                        rate_down: 0,
                        closed: false,
                        ever_sampled: false,
                    },
                );
            }
            EpisodeEvent::Bytes {
                key,
                bytes_up,
                bytes_down,
                packets_up,
                packets_down,
                at,
            } => {
                let Some(ep) = self.by_key.get_mut(&key) else {
                    return;
                };
                // Counters are absolute totals from the collector, clamped
                // monotonically (a buggy source must not roll bytes back).
                ep.bytes_up = ep.bytes_up.max(bytes_up);
                ep.bytes_down = ep.bytes_down.max(bytes_down);
                ep.packets_up = ep.packets_up.max(packets_up);
                ep.packets_down = ep.packets_down.max(packets_down);
                ep.last_seen_at = ep.last_seen_at.max(at);
            }
            EpisodeEvent::Close { key, at } => {
                let Some(ep) = self.by_key.get_mut(&key) else {
                    return;
                };
                ep.last_seen_at = ep.last_seen_at.max(at);
                ep.closed = true;
            }
        }
    }

    /// Advance one 1 Hz sample: compute rates, diff the live view, emit the
    /// wire delta and the closed-episode rows (persisted immediately).
    pub fn sample(&mut self, now: u64) -> Sample {
        let mut sample = Sample::default();
        for ep in self.by_key.values_mut() {
            if ep.closed {
                // First sample after close: emit removal + final row.
                sample.delta.removed.push(ep.id);
                sample.closed_rows.push(Self::row_of(ep));
                continue;
            }
            // Rates: bytes accumulated since the previous sample, over the
            // seconds between samples (bytes/s, API.md C1 `*_bps`).
            if !ep.ever_sampled {
                // First sighting: delta `added` carries the full row. The
                // sample baseline is still advanced so the next sample
                // measures only bytes accumulated after this one.
                ep.ever_sampled = true;
                ep.sampled_up = ep.bytes_up;
                ep.sampled_down = ep.bytes_down;
                ep.sampled_at = now;
                sample.delta.added.push(Self::live_row_of(ep));
                continue;
            }
            let dt = now.saturating_sub(ep.sampled_at);
            if dt > 0 {
                ep.rate_up = ep.bytes_up.saturating_sub(ep.sampled_up) / dt;
                ep.rate_down = ep.bytes_down.saturating_sub(ep.sampled_down) / dt;
                ep.sampled_up = ep.bytes_up;
                ep.sampled_down = ep.bytes_down;
                ep.sampled_at = now;
            }
            sample.delta.updated.push(LiveUpdateRow {
                id: ep.id,
                rate_up_bps: ep.rate_up,
                rate_down_bps: ep.rate_down,
                bytes_up: ep.bytes_up,
                bytes_down: ep.bytes_down,
            });
        }
        // Drop closed episodes after emitting their removal.
        self.by_key.retain(|_, ep| !ep.closed);
        sample
    }

    /// Full live view, sorted by total activity (rate + bytes) descending —
    /// the C1 default `sort=rate&order=desc` and the UI's top-row-first
    /// ordering (DESIGN.md §8.4).
    #[must_use]
    pub fn snapshot(&self) -> Vec<LiveConnectionRow> {
        let mut rows: Vec<LiveConnectionRow> = self
            .by_key
            .values()
            .map(|ep| Self::live_row_of(ep))
            .collect();
        rows.sort_by(|a, b| {
            let act_a = a.rate_up_bps + a.rate_down_bps + a.bytes_up + a.bytes_down;
            let act_b = b.rate_up_bps + b.rate_down_bps + b.bytes_up + b.bytes_down;
            act_b.cmp(&act_a).then_with(|| a.id.cmp(&b.id))
        });
        rows
    }

    /// Checkpoint rows for all open episodes (persisted on the ~60 s
    /// cadence; absolute totals — the writer upserts, never accumulates).
    #[must_use]
    pub fn checkpoint_rows(&self) -> Vec<EpisodeRow> {
        self.by_key
            .values()
            .filter(|ep| !ep.closed)
            .map(|ep| Self::row_of(ep))
            .collect()
    }

    /// Number of open episodes (diagnostics/cheap sizing).
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.by_key.values().filter(|ep| !ep.closed).count()
    }

    fn live_row_of(ep: &Episode) -> LiveConnectionRow {
        LiveConnectionRow {
            id: ep.id,
            app: LiveApp {
                id: ep.app_id,
                name: ep.app_name.clone(),
                icon: ep.icon.clone(),
            },
            pid: ep.pid,
            process_name: ep.process_name.clone(),
            protocol: ep.protocol,
            local_ip: ep.local_ip.clone(),
            local_port: ep.local_port,
            remote_ip: ep.remote_ip.clone(),
            remote_port: ep.remote_port,
            remote_host: ep.remote_host.clone(),
            rate_up_bps: ep.rate_up,
            rate_down_bps: ep.rate_down,
            bytes_up: ep.bytes_up,
            bytes_down: ep.bytes_down,
        }
    }

    fn row_of(ep: &Episode) -> EpisodeRow {
        EpisodeRow {
            id: ep.id,
            app_id: ep.app_id,
            interface_id: ep.interface_id,
            pid: ep.pid,
            process_name: ep.process_name.clone(),
            protocol: ep.protocol,
            local_ip: ep.local_ip.clone(),
            local_port: ep.local_port,
            remote_ip: ep.remote_ip.clone(),
            remote_port: ep.remote_port,
            remote_host: ep.remote_host.clone(),
            bytes_up: ep.bytes_up,
            bytes_down: ep.bytes_down,
            packets_up: ep.packets_up,
            packets_down: ep.packets_down,
            first_seen_at: ep.first_seen_at,
            last_seen_at: ep.last_seen_at,
            is_open: !ep.closed,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    const KEY: u64 = 42;

    fn open_event(key: u64) -> EpisodeEvent {
        EpisodeEvent::Open(EpisodeOpen {
            key,
            app_id: 1,
            app_name: "Firefox".to_string(),
            icon: None,
            pid: 2213,
            process_name: Some("firefox".to_string()),
            protocol: Protocol::Tcp,
            local_ip: "192.168.1.23".to_string(),
            local_port: 53210,
            remote_ip: "151.101.1.1".to_string(),
            remote_port: 443,
            remote_host: None,
            interface_id: Some(2),
            at: 1000,
        })
    }

    #[test]
    fn open_assigns_sequential_ids_above_start() {
        let mut store = EpisodeStore::new(88231);
        store.apply(open_event(KEY));
        store.apply(open_event(KEY + 1));
        let snap = store.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].id, 88231);
        assert_eq!(snap[1].id, 88232);
    }

    #[test]
    fn first_sample_adds_full_row() {
        let mut store = EpisodeStore::new(1);
        store.apply(open_event(KEY));
        let s = store.sample(1001);
        assert_eq!(s.delta.added.len(), 1);
        assert_eq!(s.delta.updated.len(), 0);
        assert_eq!(s.delta.added[0].app.name, "Firefox");
        assert_eq!(s.delta.added[0].bytes_up, 0);
        // Checkpoint includes the open episode even with zero bytes.
        assert_eq!(store.checkpoint_rows().len(), 1);
    }

    #[test]
    fn bytes_advance_counters_monotonically() {
        let mut store = EpisodeStore::new(1);
        store.apply(open_event(KEY));
        store.apply(EpisodeEvent::Bytes {
            key: KEY,
            bytes_up: 240_000,
            bytes_down: 1_980_000,
            packets_up: 300,
            packets_down: 1900,
            at: 1002,
        });
        // A lower total from a buggy source must not roll back.
        store.apply(EpisodeEvent::Bytes {
            key: KEY,
            bytes_up: 10,
            bytes_down: 10,
            packets_up: 1,
            packets_down: 1,
            at: 1003,
        });
        let row = &store.checkpoint_rows()[0];
        assert_eq!(row.bytes_up, 240_000);
        assert_eq!(row.bytes_down, 1_980_000);
    }

    #[test]
    fn rate_computed_from_byte_delta_over_seconds() {
        let mut store = EpisodeStore::new(1);
        store.apply(open_event(KEY));
        let _ = store.sample(1001); // added
        store.apply(EpisodeEvent::Bytes {
            key: KEY,
            bytes_up: 40_960,
            bytes_down: 262_144,
            packets_up: 40,
            packets_down: 260,
            at: 1003,
        });
        let s = store.sample(1003); // 2 s window → rate = bytes/2
        let upd = &s.delta.updated[0];
        assert_eq!(upd.rate_up_bps, 20_480);
        assert_eq!(upd.rate_down_bps, 131_072);
        assert_eq!(upd.bytes_up, 40_960);
    }

    #[test]
    fn close_emits_removal_and_final_row_then_gone() {
        let mut store = EpisodeStore::new(1);
        store.apply(open_event(KEY));
        let _ = store.sample(1001);
        store.apply(EpisodeEvent::Bytes {
            key: KEY,
            bytes_up: 1000,
            bytes_down: 2000,
            packets_up: 10,
            packets_down: 20,
            at: 1002,
        });
        store.apply(EpisodeEvent::Close { key: KEY, at: 1003 });

        let s = store.sample(1003);
        assert_eq!(s.delta.removed, vec![1]);
        let final_row = &s.closed_rows[0];
        assert_eq!(final_row.id, 1);
        assert_eq!(final_row.bytes_up, 1000);
        assert!(!final_row.is_open);
        // Gone from the live view afterwards.
        assert_eq!(store.snapshot().len(), 0);
        assert_eq!(store.open_count(), 0);
    }

    #[test]
    fn reopen_after_close_starts_fresh() {
        let mut store = EpisodeStore::new(5);
        store.apply(open_event(KEY));
        let _ = store.sample(1001);
        store.apply(EpisodeEvent::Close { key: KEY, at: 1002 });
        let _ = store.sample(1002);
        store.apply(open_event(KEY)); // new socket with a recycled cookie
        let s = store.sample(1003);
        assert_eq!(s.delta.added.len(), 1);
        assert_eq!(s.delta.added[0].id, 6); // fresh id, not the old one
        assert_eq!(s.delta.added[0].bytes_up, 0);
    }

    #[test]
    fn unknown_key_events_ignored() {
        let mut store = EpisodeStore::new(1);
        store.apply(EpisodeEvent::Bytes {
            key: 999,
            bytes_up: 1,
            bytes_down: 1,
            packets_up: 0,
            packets_down: 0,
            at: 1,
        });
        store.apply(EpisodeEvent::Close { key: 999, at: 1 });
        assert_eq!(store.open_count(), 0);
        let s = store.sample(2);
        assert!(s.delta.added.is_empty() && s.delta.removed.is_empty());
    }

    #[test]
    fn snapshot_sorted_by_activity_desc() {
        let mut store = EpisodeStore::new(1);
        store.apply(open_event(KEY)); // id 1
        store.apply(open_event(KEY + 1)); // id 2
        let _ = store.sample(1001);
        store.apply(EpisodeEvent::Bytes {
            key: KEY + 1,
            bytes_up: 500_000,
            bytes_down: 0,
            packets_up: 0,
            packets_down: 0,
            at: 1002,
        });
        let _ = store.sample(1002);
        let snap = store.snapshot();
        assert_eq!(snap[0].id, 2, "busier episode first");
        assert_eq!(snap[1].id, 1);
    }
}
