//! Pure rate math for the 1 Hz PC-speed sampler (API.md §3 `net.rates`).
//!
//! No I/O lives here (ARCHITECTURE.md §4): the daemon supplies monotonic
//! timestamps and sysfs byte counters. Rates are decimal bits per second;
//! smoothing uses one exponential moving average constant for the whole app.

use std::collections::HashMap;

use netwatch_common::{InterfaceRate, RateSnapshot};

/// EMA time constant in seconds — about 3 s of smoothing (live plan §4).
pub const EMA_TAU_S: f64 = 3.0;
/// Decimal bits per byte (1 Mbps = 1,000,000 bps).
const BITS_PER_BYTE: f64 = 8.0;

/// Convert a byte delta over a measured interval into decimal bits/second.
///
/// A non-positive interval yields zero rather than infinity — a delayed or
/// duplicated tick must never inflate the displayed rate.
#[allow(clippy::cast_precision_loss)] // byte deltas are far below f64's exact integer range
#[must_use]
pub fn bits_per_second(delta_bytes: u64, elapsed_s: f64) -> f64 {
    if elapsed_s <= 0.0 {
        return 0.0;
    }
    (delta_bytes as f64) * BITS_PER_BYTE / elapsed_s
}

/// Exponential moving average with a measured time step.
#[must_use]
pub fn ema(previous: f64, sample: f64, elapsed_s: f64, tau_s: f64) -> f64 {
    if elapsed_s <= 0.0 || tau_s <= 0.0 {
        return previous;
    }
    let alpha = 1.0 - (-elapsed_s / tau_s).exp();
    previous + alpha * (sample - previous)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to a valid u64 range
fn to_bps(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        value.round().min(1.0e18) as u64
    }
}

/// Stateful rate calculator over per-interface byte counters.
#[derive(Debug, Default)]
pub struct RateMeter {
    /// Last observed (rx, tx) byte counters by interface name.
    previous: HashMap<String, (u64, u64)>,
    /// Last sample timestamp (monotonic seconds).
    last_ts: Option<f64>,
    /// Smoothed total download/upload rates.
    ema_down: f64,
    ema_up: f64,
}

impl RateMeter {
    /// Record one sample and return the current smoothed rate snapshot.
    ///
    /// The first call establishes the baseline (all rates zero). Counter
    /// decreases — from interface reset or a wrap — are treated as zero delta.
    /// Interfaces missing from `readings` are dropped from the previous state.
    pub fn observe(&mut self, now_s: f64, readings: &[(String, u64, u64)]) -> RateSnapshot {
        let elapsed_s = self.last_ts.map_or(0.0, |previous| now_s - previous);
        self.last_ts = Some(now_s);

        let current: HashMap<&str, (u64, u64)> = readings
            .iter()
            .map(|(name, rx, tx)| (name.as_str(), (*rx, *tx)))
            .collect();
        self.previous
            .retain(|name, _| current.contains_key(name.as_str()));

        let mut down_total = 0.0;
        let mut up_total = 0.0;
        let mut interfaces = Vec::with_capacity(readings.len());
        for (name, rx, tx) in readings {
            let (prev_rx, prev_tx) = self
                .previous
                .get(name.as_str())
                .copied()
                .unwrap_or((*rx, *tx));
            let down = bits_per_second(rx.saturating_sub(prev_rx), elapsed_s);
            let up = bits_per_second(tx.saturating_sub(prev_tx), elapsed_s);
            down_total += down;
            up_total += up;
            interfaces.push(InterfaceRate {
                name: name.clone(),
                down_bps: to_bps(down),
                up_bps: to_bps(up),
            });
            self.previous.insert(name.clone(), (*rx, *tx));
        }

        if elapsed_s > 0.0 {
            self.ema_down = ema(self.ema_down, down_total, elapsed_s, EMA_TAU_S);
            self.ema_up = ema(self.ema_up, up_total, elapsed_s, EMA_TAU_S);
        }
        RateSnapshot {
            // The daemon fills the epoch timestamp; this function only knows
            // the monotonic elapsed time used for the rate calculation.
            ts: 0,
            down_bps: to_bps(self.ema_down),
            up_bps: to_bps(self.ema_up),
            interfaces,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests

    use super::*;

    fn reading(name: &str, rx: u64, tx: u64) -> (String, u64, u64) {
        (name.to_string(), rx, tx)
    }

    #[test]
    fn decimal_rate_and_first_sample_baseline() {
        assert!((bits_per_second(125_000, 1.0) - 1_000_000.0).abs() < f64::EPSILON);
        let mut meter = RateMeter::default();
        let first = meter.observe(100.0, &[reading("wlan0", 500, 200)]);
        assert_eq!(first.down_bps, 0);
        assert_eq!(first.up_bps, 0);
        assert_eq!(first.interfaces.len(), 1);
    }

    #[test]
    fn counter_decrease_and_reset_never_go_negative() {
        let mut meter = RateMeter::default();
        meter.observe(0.0, &[reading("wlan0", 1_000, 1_000)]);
        // Device reset: counters went backwards.
        let reset = meter.observe(1.0, &[reading("wlan0", 10, 10)]);
        assert_eq!(reset.interfaces[0].down_bps, 0);
        assert!(reset.down_bps < 1_000_000); // EMA decaying toward zero
    }

    #[test]
    fn late_tick_uses_measured_elapsed_time() {
        let mut meter = RateMeter::default();
        meter.observe(0.0, &[reading("wlan0", 0, 0)]);
        // 1 MB in 2 s = 4 Mbps, not 8 Mbps (nominal 1 s tick).
        let snapshot = meter.observe(2.0, &[reading("wlan0", 1_000_000, 0)]);
        assert!(snapshot.interfaces[0].down_bps > 3_000_000);
        assert!(snapshot.interfaces[0].down_bps < 5_000_000);
    }

    #[test]
    fn interface_added_and_removed() {
        let mut meter = RateMeter::default();
        meter.observe(0.0, &[reading("wlan0", 0, 0)]);
        let both = meter.observe(
            1.0,
            &[reading("wlan0", 1_000, 0), reading("eth0", 2_000, 0)],
        );
        assert_eq!(both.interfaces.len(), 2);
        let only_wlan = meter.observe(2.0, &[reading("wlan0", 2_000, 0)]);
        assert_eq!(only_wlan.interfaces.len(), 1);
        assert_eq!(only_wlan.interfaces[0].name, "wlan0");
    }

    #[test]
    fn ema_smooths_a_step_without_overshoot() {
        let first = ema(0.0, 100.0, 1.0, EMA_TAU_S);
        assert!(first > 0.0 && first < 100.0);
        assert!(ema(first, 100.0, 1.0, EMA_TAU_S) <= 100.0);
    }
}
