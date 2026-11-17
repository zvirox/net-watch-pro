//! Interface-counter delta logic (ARCHITECTURE.md §5.1 step 4).
//!
//! This crate has no I/O dependencies (ARCHITECTURE.md §4): the caller reads
//! cumulative `/sys/class/net/*/statistics` counters and feeds them here. The
//! first observation is a baseline only — writing it as traffic would invent
//! usage the kernel never measured.

use std::collections::HashMap;

use crate::retention::bucket_15m;

/// Cumulative byte counters from one interface's sysfs statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceCounters {
    /// Bytes received since boot (`statistics/rx_bytes`).
    pub rx_bytes: u64,
    /// Bytes transmitted since boot (`statistics/tx_bytes`).
    pub tx_bytes: u64,
}

/// A measured delta for one interface and 15-minute UTC bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSample {
    /// sysfs interface name (FK to `interfaces.name`).
    pub name: String,
    /// UTC-aligned 15-minute bucket (`DATABASE.md` §2.5b).
    pub bucket_start: i64,
    /// Bytes received in this sample.
    pub rx_bytes: u64,
    /// Bytes transmitted in this sample.
    pub tx_bytes: u64,
}

/// Tracks the last cumulative counters seen per interface.
#[derive(Debug, Default)]
pub struct InterfaceSampler {
    previous: HashMap<String, InterfaceCounters>,
}

impl InterfaceSampler {
    /// Create an empty baseline map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one counter observation. Returns a sample only when bytes moved
    /// since the previous observation; counter resets and wraps clamp to zero
    /// rather than producing negative usage.
    pub fn observe(
        &mut self,
        name: String,
        current: InterfaceCounters,
        now: i64,
    ) -> Option<InterfaceSample> {
        let previous = self.previous.insert(name.clone(), current)?;
        let rx_bytes = current.rx_bytes.saturating_sub(previous.rx_bytes);
        let tx_bytes = current.tx_bytes.saturating_sub(previous.tx_bytes);
        if rx_bytes == 0 && tx_bytes == 0 {
            return None;
        }
        Some(InterfaceSample {
            name,
            bucket_start: bucket_15m(now),
            rx_bytes,
            tx_bytes,
        })
    }
}

/// Database `interfaces.kind` value inferred from a sysfs name
/// (`DATABASE.md` §2.3). The daemon cannot see link technology from the
/// counters themselves, so well-known prefixes are used; everything else is
/// recorded as ethernet.
#[must_use]
pub fn interface_kind(name: &str) -> &'static str {
    if name == "lo" {
        "loopback"
    } else if name.starts_with("wlan") || name.starts_with("wlp") {
        "wifi"
    } else if name.starts_with("wg") || name.starts_with("tun") || name.starts_with("tap") {
        "tun"
    } else {
        "ethernet"
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    #[test]
    fn first_observation_is_baseline_only() {
        let mut sampler = InterfaceSampler::new();
        let sample = sampler.observe(
            "eth0".to_string(),
            InterfaceCounters {
                rx_bytes: 100,
                tx_bytes: 50,
            },
            900,
        );
        assert!(sample.is_none(), "baseline must not be counted as traffic");
    }

    #[test]
    fn subsequent_observation_reports_delta_in_bucket() {
        let mut sampler = InterfaceSampler::new();
        sampler.observe(
            "eth0".to_string(),
            InterfaceCounters {
                rx_bytes: 100,
                tx_bytes: 50,
            },
            0,
        );
        let sample = sampler
            .observe(
                "eth0".to_string(),
                InterfaceCounters {
                    rx_bytes: 350,
                    tx_bytes: 70,
                },
                901,
            )
            .unwrap();
        assert_eq!(
            sample,
            InterfaceSample {
                name: "eth0".to_string(),
                bucket_start: 900,
                rx_bytes: 250,
                tx_bytes: 20,
            }
        );
    }

    #[test]
    fn reset_and_idle_counters_never_underflow() {
        let mut sampler = InterfaceSampler::new();
        sampler.observe(
            "wlan0".to_string(),
            InterfaceCounters {
                rx_bytes: 900,
                tx_bytes: 800,
            },
            0,
        );
        assert!(
            sampler
                .observe(
                    "wlan0".to_string(),
                    InterfaceCounters {
                        rx_bytes: 100,
                        tx_bytes: 50,
                    },
                    900,
                )
                .is_none()
        );
        assert_eq!(interface_kind("lo"), "loopback");
        assert_eq!(interface_kind("wlp2s0"), "wifi");
        assert_eq!(interface_kind("wg0"), "tun");
        assert_eq!(interface_kind("virbr0"), "ethernet");
    }
}
