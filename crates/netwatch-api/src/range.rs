//! Phase D range presets (API.md §1.4).
//!
//! A single preset → period/bucket resolver shared by U1/U2/U3/A1/A3 so one
//! choice drives headline totals, timelines and app rankings together. All
//! boundaries are computed from the configured timezone and billing day
//! (`calendar.rs`, D4a); this module has no I/O of its own.

use chrono::DateTime;
use chrono_tz::Tz;
use sqlx::SqlitePool;

use crate::calendar::{self, Bucket, Period};
use crate::error::ApiError;

/// The five presets exposed by the UI (DESIGN.md §7.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangePreset {
    /// Rolling last 60 minutes.
    Hour1,
    /// Current local calendar day (local midnight → next midnight).
    Day1,
    /// Seven local calendar days, ending tomorrow's local midnight.
    Days7,
    /// Thirty local calendar days, ending tomorrow's local midnight.
    Days30,
    /// Current billing cycle from `billing_cycle_day`.
    Cycle,
}

impl RangePreset {
    /// Parse a `?range=` value.
    ///
    /// # Errors
    ///
    /// `QUERY_INVALID` for an unknown preset value.
    pub fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "1h" => Ok(Self::Hour1),
            "24h" => Ok(Self::Day1),
            "7d" => Ok(Self::Days7),
            "30d" => Ok(Self::Days30),
            "cycle" => Ok(Self::Cycle),
            other => Err(ApiError::query_invalid(format!("invalid range: {other}"))),
        }
    }

    /// Stable wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hour1 => "1h",
            Self::Day1 => "24h",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::Cycle => "cycle",
        }
    }

    /// Automatic chart bucket for this preset (D3).
    #[must_use]
    pub const fn bucket(self) -> Bucket {
        match self {
            Self::Hour1 | Self::Day1 => Bucket::Minute15,
            Self::Days7 => Bucket::Hour,
            Self::Days30 | Self::Cycle => Bucket::Day,
        }
    }

    /// Half-open period `[start, end)` for this preset.
    #[must_use]
    pub fn period(self, now: i64, tz: Tz, billing_day: i64) -> Period {
        match self {
            Self::Hour1 => Period {
                start: now.saturating_sub(60 * 60),
                end: now,
            },
            Self::Day1 => calendar::day_period(now, tz),
            Self::Days7 => calendar::local_days_period(now, tz, 7),
            Self::Days30 => calendar::local_days_period(now, tz, 30),
            Self::Cycle => calendar::billing_period(now, tz, billing_day),
        }
    }
}

/// Read `range` from the query string, if present. Other keys are ignored so
/// each endpoint's own parser can still reject unknown parameters.
///
/// # Errors
///
/// `QUERY_INVALID` when the `range` value is unknown or percent-encoding is
/// malformed.
pub fn from_query(raw: Option<&str>) -> Result<Option<RangePreset>, ApiError> {
    for (key, value) in crate::history::parse_query_params(raw.unwrap_or(""))? {
        if key == "range" {
            return RangePreset::parse(&value).map(Some);
        }
    }
    Ok(None)
}

/// Resolve the effective preset: explicit query value, else the remembered
/// `range_default` setting, else `24h` (API.md §1.4).
///
/// # Errors
///
/// `QUERY_INVALID` for a bad query value; `DB_ERROR` when the setting cannot
/// be read. An unknown stored setting value falls back to `24h`.
pub async fn resolve(pool: &SqlitePool, raw: Option<&str>) -> Result<RangePreset, ApiError> {
    if let Some(preset) = from_query(raw)? {
        return Ok(preset);
    }
    let configured = crate::usage::setting(pool, "range_default", "24h").await?;
    RangePreset::parse(&configured).or(Ok(RangePreset::Day1))
}

/// Whole days from `now` until the next cycle starts (D5 cycle card).
#[must_use]
pub fn days_left(now: i64, tz: Tz, cycle: Period) -> i64 {
    let today = DateTime::from_timestamp(now, 0)
        .unwrap_or_default()
        .with_timezone(&tz)
        .date_naive();
    let end = DateTime::from_timestamp(cycle.end, 0)
        .unwrap_or_default()
        .with_timezone(&tz)
        .date_naive();
    (end - today).num_days().max(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use chrono::{TimeZone, Utc};

    fn epoch(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn day_preset_resets_at_local_midnight_in_kolkata() {
        let kolkata = "Asia/Kolkata".parse::<Tz>().unwrap();
        // 18:30 UTC is 00:00 local on the 16th — the local day starts there,
        // not at 05:30 UTC when a naive UTC-midnight would flip it.
        let period = RangePreset::Day1.period(epoch(2026, 1, 15, 18, 30), kolkata, 1);
        assert_eq!(period.start, epoch(2026, 1, 15, 18, 30));
        assert_eq!(period.end, epoch(2026, 1, 16, 18, 30));
        let before = RangePreset::Day1.period(epoch(2026, 1, 15, 18, 29), kolkata, 1);
        assert_eq!(before.end, period.start);
    }

    #[test]
    fn cycle_preset_uses_configured_day_and_clamps_short_months() {
        let tz = Tz::UTC;
        let period = RangePreset::Cycle.period(epoch(2026, 2, 10, 12, 0), tz, 15);
        assert_eq!(period.start, epoch(2026, 1, 15, 0, 0));
        assert_eq!(period.end, epoch(2026, 2, 15, 0, 0));

        let short = RangePreset::Cycle.period(epoch(2026, 4, 10, 12, 0), tz, 31);
        assert_eq!(short.start, epoch(2026, 3, 31, 0, 0));
        assert_eq!(short.end, epoch(2026, 4, 30, 0, 0));
    }

    #[test]
    fn buckets_and_periods_follow_the_preset() {
        assert_eq!(RangePreset::Hour1.bucket(), Bucket::Minute15);
        assert_eq!(RangePreset::Day1.bucket(), Bucket::Minute15);
        assert_eq!(RangePreset::Days7.bucket(), Bucket::Hour);
        assert_eq!(RangePreset::Days30.bucket(), Bucket::Day);
        assert_eq!(RangePreset::Cycle.bucket(), Bucket::Day);

        let now = epoch(2026, 1, 15, 12, 0);
        let hour = RangePreset::Hour1.period(now, Tz::UTC, 1);
        assert_eq!(hour.start, now - 3600);
        assert_eq!(hour.end, now);
        let days7 = RangePreset::Days7.period(now, Tz::UTC, 1);
        assert_eq!(days7.start, epoch(2026, 1, 9, 0, 0));
        assert_eq!(days7.end, epoch(2026, 1, 16, 0, 0));
    }

    #[test]
    fn days_left_counts_to_next_cycle_start() {
        let tz = Tz::UTC;
        let now = epoch(2026, 1, 20, 12, 0);
        let cycle = RangePreset::Cycle.period(now, tz, 15);
        assert_eq!(days_left(now, tz, cycle), 26);
    }

    #[test]
    fn invalid_range_is_query_invalid() {
        assert!(RangePreset::parse("13h").is_err());
        assert_eq!(RangePreset::parse("7d").unwrap(), RangePreset::Days7);
    }
}
