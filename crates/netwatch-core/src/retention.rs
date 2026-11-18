//! Retention math shared by the compaction job (DATABASE.md §2.5/§2.5a).
//!
//! ARCHITECTURE.md §4 keeps this crate free of I/O: the caller supplies epoch
//! seconds and the SQLite layer performs the writes. Buckets are UTC-aligned
//! here; timezone concerns only affect *display* boundaries (D4a).

/// Length of a base usage bucket, in seconds (DATABASE.md §2.5).
pub const BUCKET_15M_S: i64 = 15 * 60;

/// Length of an endpoint-day bucket, in seconds (DATABASE.md §2.5a).
pub const DAY_S: i64 = 24 * 60 * 60;

/// Default endpoint cardinality cap (DATABASE.md §2.5a, D2a).
pub const DEFAULT_ENDPOINT_CAP: usize = 50;

/// Align a timestamp to its 15-minute UTC bucket.
#[must_use]
pub fn bucket_15m(epoch_seconds: i64) -> i64 {
    epoch_seconds.div_euclid(BUCKET_15M_S) * BUCKET_15M_S
}

/// Align a timestamp to its UTC day bucket.
#[must_use]
pub fn bucket_day(epoch_seconds: i64) -> i64 {
    epoch_seconds.div_euclid(DAY_S) * DAY_S
}

/// Free-tier retention ceiling in days (PRD §6.1 #5).
pub const FREE_MAX_DAYS: i64 = 90;
/// Pro retention ceiling in days (API.md L1 `retention_max_days`).
pub const PRO_MAX_DAYS: i64 = 365;
/// Minimum retention, even if a bad setting value is stored.
pub const MIN_RETENTION_DAYS: i64 = 7;

/// Clamp a configured retention value to the caller's tier (DATABASE.md §6).
///
/// The API validates `retention_days` on write, but a license can be
/// deactivated afterwards while a larger value remains stored — so the
/// pruner re-applies the ceiling rather than trusting the setting alone.
#[must_use]
pub fn effective_retention_days(configured: i64, is_pro: bool) -> i64 {
    let ceiling = if is_pro { PRO_MAX_DAYS } else { FREE_MAX_DAYS };
    configured.clamp(MIN_RETENTION_DAYS, ceiling)
}

/// First epoch second outside the retention window. Rows older than this are
/// eligible for pruning; rollups are never pruned (DATABASE.md §2.5).
#[must_use]
pub fn retention_cutoff(now: i64, days: i64) -> i64 {
    now.saturating_sub(
        days.clamp(MIN_RETENTION_DAYS, PRO_MAX_DAYS)
            .saturating_mul(DAY_S),
    )
}

/// First epoch second outside the raw window. Rows with `last_seen_at` below
/// this cutoff are eligible for compaction (API.md C2).
///
/// A non-positive window is treated as the one-day minimum: zero would
/// compact every row, including data observed in the current second.
#[must_use]
pub fn raw_cutoff(now: i64, raw_window_days: i64) -> i64 {
    now.saturating_sub(raw_window_days.clamp(1, i64::MAX).saturating_mul(DAY_S))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    #[test]
    fn buckets_align_to_utc_boundaries() {
        assert_eq!(bucket_15m(0), 0);
        assert_eq!(bucket_15m(899), 0);
        assert_eq!(bucket_15m(900), 900);
        assert_eq!(bucket_15m(1_753_155_601), 1_753_155_000);
        assert_eq!(bucket_day(86_399), 0);
        assert_eq!(bucket_day(86_400), 86_400);
    }

    #[test]
    fn cutoff_is_now_minus_raw_window() {
        let now = 2_000_000_000;
        assert_eq!(raw_cutoff(now, 7), now - 7 * DAY_S);
        assert_eq!(raw_cutoff(now, 0), now - DAY_S);
    }

    #[test]
    fn tier_ceiling_is_reapplied_at_prune_time() {
        assert_eq!(effective_retention_days(120, false), FREE_MAX_DAYS);
        assert_eq!(effective_retention_days(120, true), 120);
        assert_eq!(effective_retention_days(400, true), PRO_MAX_DAYS);
        assert_eq!(effective_retention_days(0, false), MIN_RETENTION_DAYS);
        assert_eq!(
            retention_cutoff(2_000_000_000, FREE_MAX_DAYS),
            2_000_000_000 - 90 * DAY_S
        );
    }
}
