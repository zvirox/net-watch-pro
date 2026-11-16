//! Per-socket byte attribution accounting.
//!
//! Phase 2: accumulate byte deltas per socket cookie from eBPF cgroup_skb
//! programs. Open-episode state is kept in memory and persisted on episode
//! close or at checkpoint interval (~60s) to limit crash-loss window.
//!
//! ARCHITECTURE.md §5.1 step 3: open-episode state in memory, persisted on
//! episode close and at checkpoint interval.
//! DESIGN.md §8.4: Live Connections table tracks per-connection bytes.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-socket byte counters accumulated from eBPF cgroup_skb programs.
#[derive(Debug, Clone, Default)]
pub struct SocketCounters {
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
    pub first_seen_at: u64,
    pub last_seen_at: u64,
    pub is_open: bool,
}

/// Socket cookie -> per-socket byte counters mapping.
/// Populated from eBPF cgroup_skb ingress/egress programs.
pub struct SocketCounterMap {
    /// socket cookie -> per-socket counters
    counters: HashMap<u64, SocketCounters>,
}

impl SocketCounterMap {
    /// Create a new empty counter map.
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Record bytes for a socket cookie.
    pub fn add_bytes(
        &mut self,
        cookie: u64,
        bytes_up: u64,
        bytes_down: u64,
        packets_up: u64,
        packets_down: u64,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let counter = self
            .counters
            .entry(cookie)
            .or_insert_with(|| SocketCounters {
                bytes_up: 0,
                bytes_down: 0,
                packets_up: 0,
                packets_down: 0,
                first_seen_at: now,
                last_seen_at: now,
                is_open: true,
            });

        counter.bytes_up = counter.bytes_up.saturating_add(bytes_up);
        counter.bytes_down = counter.bytes_down.saturating_add(bytes_down);
        counter.packets_up = counter.packets_up.saturating_add(packets_up);
        counter.packets_down = counter.packets_down.saturating_add(packets_down);
        counter.last_seen_at = counter.last_seen_at.max(now);
        counter.is_open = true;
    }

    /// Mark a socket as closed (no longer active).
    pub fn close_socket(&mut self, cookie: u64) {
        if let Some(counter) = self.counters.get_mut(&cookie) {
            counter.is_open = false;
        }
    }

    /// Get a reference to the counter for a socket cookie.
    pub fn get(&self, cookie: u64) -> Option<&SocketCounters> {
        self.counters.get(&cookie)
    }

    /// Get all open connections.
    pub fn open_connections(&self) -> Vec<(u64, &SocketCounters)> {
        self.counters
            .iter()
            .filter(|(_, c)| c.is_open)
            .map(|(k, v)| (*k, v))
            .collect()
    }

    /// Get all counters (open and closed).
    pub fn all(&self) -> Vec<(u64, &SocketCounters)> {
        self.counters.iter().map(|(k, v)| (*k, v)).collect()
    }

    /// Number of tracked sockets.
    pub fn len(&self) -> usize {
        self.counters.len()
    }

    /// Number of tracked sockets (alias for `len`, used in tests).
    pub fn pending_count(&self) -> usize {
        self.counters.len()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.counters.is_empty()
    }

    /// Remove a closed socket from the map (called after persisting to DB).
    pub fn remove_closed(&mut self) {
        self.counters.retain(|_, c| c.is_open);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_counter_accumulates_bytes() {
        let mut map = SocketCounterMap::new();
        map.add_bytes(1, 1000, 2000, 10, 20);
        map.add_bytes(1, 500, 300, 5, 10);

        let c = map.get(1).unwrap();
        assert_eq!(c.bytes_up, 1500);
        assert_eq!(c.bytes_down, 2300);
        assert_eq!(c.packets_up, 15);
        assert_eq!(c.packets_down, 30);
        assert!(c.is_open);
    }

    #[test]
    fn socket_close_marks_closed() {
        let mut map = SocketCounterMap::new();
        map.add_bytes(1, 1000, 2000, 10, 20);
        map.close_socket(1);

        let c = map.get(1).unwrap();
        assert!(!c.is_open);
    }

    #[test]
    fn open_connections_filters_closed() {
        let mut map = SocketCounterMap::new();
        map.add_bytes(1, 1000, 2000, 10, 20);
        map.add_bytes(2, 500, 300, 5, 10);
        map.close_socket(1);

        let open: Vec<(u64, &SocketCounters)> = map.open_connections();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].0, 2);
    }

    #[test]
    fn remove_closed_removes_from_map() {
        let mut map = SocketCounterMap::new();
        map.add_bytes(1, 1000, 2000, 10, 20);
        map.close_socket(1);
        map.add_bytes(2, 500, 300, 5, 10);

        map.remove_closed();
        assert_eq!(map.pending_count(), 1);
        assert_eq!(map.get(2).is_some(), true);
        assert_eq!(map.get(1).is_some(), false);
    }

    #[test]
    fn empty_map_has_zero_len() {
        let map = SocketCounterMap::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }
}
