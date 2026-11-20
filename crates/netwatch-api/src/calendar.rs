//! Local day and billing-cycle periods for usage aggregates (D4a).
//!
//! Base rollups stay UTC-aligned (`DATABASE.md` §2.5); only the boundaries
//! used to sum them are local. This module has no I/O: the caller reads the
//! configured timezone/billing day and passes them in.

use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;

/// Chart bucket size for usage timelines (API.md A3/U2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Fifteen minutes — the automatic bucket for `1h` and `24h` (Phase D).
    Minute15,
    /// One local hour.
    Hour,
    /// One local calendar day.
    Day,
}

/// Half-open UTC epoch-second range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    /// Inclusive UTC start second.
    pub start: i64,
    /// Exclusive UTC end second.
    pub end: i64,
}

/// Resolve a configured timezone name. `"local"` (the seed default) uses the
/// host's `/etc/localtime` symlink or `/etc/timezone`, falling back to UTC.
/// Unknown names also fall back to UTC rather than failing a usage query.
#[must_use]
pub fn resolve_timezone(configured: &str) -> Tz {
    let name = if configured.is_empty() || configured == "local" {
        host_timezone().unwrap_or_else(|| "UTC".to_string())
    } else {
        configured.to_string()
    };
    name.parse::<Tz>().unwrap_or(Tz::UTC)
}

fn host_timezone() -> Option<String> {
    if let Ok(link) = std::fs::read_link("/etc/localtime") {
        let path = link.to_string_lossy();
        if let Some((_, name)) = path.rsplit_once("zoneinfo/") {
            return Some(name.to_string());
        }
    }
    std::fs::read_to_string("/etc/timezone")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn local_now(now: i64, tz: Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(now, 0)
        .unwrap_or_default()
        .with_timezone(&tz)
}

fn local_datetime(naive: NaiveDateTime, tz: Tz) -> DateTime<Tz> {
    for hours in 0..3 {
        let candidate = naive + Duration::hours(hours);
        match tz.from_local_datetime(&candidate) {
            LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => return dt,
            LocalResult::None => {}
        }
    }
    Utc.from_utc_datetime(&naive).with_timezone(&tz)
}

/// Midnight in `tz` for a local date. Some zones skip midnight on DST change
/// days; the first valid local time within the next three hours is used.
fn local_midnight(date: NaiveDate, tz: Tz) -> DateTime<Tz> {
    local_datetime(date.and_hms_opt(0, 0, 0).unwrap_or_default(), tz)
}

/// UTC epoch start of the local hour/day bucket containing `epoch_seconds`.
#[must_use]
pub fn bucket_start(epoch_seconds: i64, bucket: Bucket, tz: Tz) -> i64 {
    let local = local_now(epoch_seconds, tz);
    let naive = match bucket {
        Bucket::Minute15 => local
            .date_naive()
            .and_hms_opt(local.hour(), (local.minute() / 15) * 15, 0)
            .unwrap_or_default(),
        Bucket::Hour => local
            .date_naive()
            .and_hms_opt(local.hour(), 0, 0)
            .unwrap_or_default(),
        Bucket::Day => local.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default(),
    };
    local_datetime(naive, tz).timestamp()
}

fn period_from(start_date: NaiveDate, end_date: NaiveDate, tz: Tz) -> Period {
    Period {
        start: local_midnight(start_date, tz).timestamp(),
        end: local_midnight(end_date, tz).timestamp(),
    }
}

/// The local calendar day containing `now`.
#[must_use]
pub fn day_period(now: i64, tz: Tz) -> Period {
    let date = local_now(now, tz).date_naive();
    let end = date.succ_opt().unwrap_or(date);
    period_from(date, end, tz)
}

/// The last `days` local calendar days ending at the next local midnight
/// (Phase D `7d` / `30d`). Uses local dates so a `Asia/Kolkata` day boundary
/// is 00:00 +05:30, not 05:30 UTC.
#[must_use]
pub fn local_days_period(now: i64, tz: Tz, days: i64) -> Period {
    let today = local_now(now, tz).date_naive();
    let start = today - Duration::days(days.saturating_sub(1).max(0));
    let end = today.succ_opt().unwrap_or(today);
    period_from(start, end, tz)
}

fn previous_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// Clamp a requested day-of-month (e.g. 31) to short months.
fn date_with_day(year: i32, month: u32, requested: u32) -> NaiveDate {
    (1..=requested)
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_default())
}

/// The billing cycle containing `now`. `billing_day` is 1–31; the current
/// calendar month is used when today is on or after that day, otherwise the
/// cycle started in the previous month (`DATABASE.md` §2.6).
#[must_use]
pub fn billing_period(now: i64, tz: Tz, billing_day: i64) -> Period {
    let day = u32::try_from(billing_day.clamp(1, 31)).unwrap_or(1);
    let local = local_now(now, tz);
    let this_month = date_with_day(local.year(), local.month(), day);
    let (start_year, start_month) = if local.day() >= this_month.day() {
        (local.year(), local.month())
    } else {
        previous_month(local.year(), local.month())
    };
    let start_date = date_with_day(start_year, start_month, day);
    let (end_year, end_month) = next_month(start_year, start_month);
    let end_date = date_with_day(end_year, end_month, day);
    period_from(start_date, end_date, tz)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    fn epoch(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn utc_day_is_midnight_to_midnight() {
        let period = day_period(epoch(2026, 1, 15, 12, 0), Tz::UTC);
        assert_eq!(period.start, epoch(2026, 1, 15, 0, 0));
        assert_eq!(period.end, epoch(2026, 1, 16, 0, 0));
    }

    #[test]
    fn half_and_quarter_hour_zones_shift_day_boundaries() {
        let kolkata = "Asia/Kolkata".parse::<Tz>().unwrap();
        let period = day_period(epoch(2026, 1, 15, 18, 30), kolkata);
        assert_eq!(period.start, epoch(2026, 1, 15, 18, 30));
        assert_eq!(period.end, epoch(2026, 1, 16, 18, 30));

        let kathmandu = "Asia/Kathmandu".parse::<Tz>().unwrap();
        let period = day_period(epoch(2026, 1, 15, 18, 15), kathmandu);
        assert_eq!(period.start, epoch(2026, 1, 15, 18, 15));
        assert_eq!(period.end, epoch(2026, 1, 16, 18, 15));
    }

    #[test]
    fn billing_cycle_uses_previous_month_before_cycle_day() {
        let tz = Tz::UTC;
        let period = billing_period(epoch(2026, 1, 20, 12, 0), tz, 15);
        assert_eq!(period.start, epoch(2026, 1, 15, 0, 0));
        assert_eq!(period.end, epoch(2026, 2, 15, 0, 0));

        let period = billing_period(epoch(2026, 2, 10, 12, 0), tz, 15);
        assert_eq!(period.start, epoch(2026, 1, 15, 0, 0));
        assert_eq!(period.end, epoch(2026, 2, 15, 0, 0));
    }

    #[test]
    fn billing_day_31_clamps_to_short_months() {
        let tz = Tz::UTC;
        let period = billing_period(epoch(2026, 4, 10, 12, 0), tz, 31);
        assert_eq!(period.start, epoch(2026, 3, 31, 0, 0));
        assert_eq!(period.end, epoch(2026, 4, 30, 0, 0));
    }

    #[test]
    fn timeline_buckets_align_to_local_hour_and_day() {
        let kolkata = "Asia/Kolkata".parse::<Tz>().unwrap();
        // 18:30 UTC is 00:00 local the next day in Kolkata (+05:30).
        let instant = epoch(2026, 1, 15, 18, 30);
        assert_eq!(
            bucket_start(instant, Bucket::Day, kolkata),
            epoch(2026, 1, 15, 18, 30)
        );
        assert_eq!(
            bucket_start(instant, Bucket::Hour, kolkata),
            epoch(2026, 1, 15, 18, 30)
        );
        // 19:15 UTC is 00:45 local, still the same local hour/day start.
        assert_eq!(
            bucket_start(epoch(2026, 1, 15, 19, 15), Bucket::Day, kolkata),
            epoch(2026, 1, 15, 18, 30)
        );
        assert_eq!(
            bucket_start(epoch(2026, 1, 15, 19, 15), Bucket::Hour, kolkata),
            epoch(2026, 1, 15, 18, 30)
        );
        // 20:15 UTC is 01:45 local → local hour starts at 01:00 (19:30 UTC).
        assert_eq!(
            bucket_start(epoch(2026, 1, 15, 20, 15), Bucket::Hour, kolkata),
            epoch(2026, 1, 15, 19, 30)
        );
    }
}
