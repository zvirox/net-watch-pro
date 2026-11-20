//! `GET /usage/timeline` — U2 dashboard series (API.md §2.5).
//!
//! Rows inside the raw window come from `connections`; older rows come from
//! `usage_rollup_15m`. The read-time union uses a hard cutoff so a byte is
//! counted exactly once across the seven-day seam (F-05). An explicit
//! `interface_id` switches to the per-interface counter table, which is the
//! only source carrying interface identity in older buckets.

use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::calendar::{Bucket, bucket_start, resolve_timezone};
use crate::error::ApiError;
use crate::range::RangePreset;

pub(crate) const DAY_S: i64 = 24 * 60 * 60;
const DEFAULT_DAY_BUCKETS: i64 = 90;
const DEFAULT_HOUR_BUCKETS: i64 = 168;
const DEFAULT_MINUTE15_BUCKETS: i64 = 96;
const LIMIT_MAX: i64 = 500;

/// One chart point (A3 shape, minus `app_id`).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TimelineRow {
    /// UTC epoch start of the local hour/day bucket.
    pub bucket_start: i64,
    /// Uploaded bytes in the bucket.
    pub up: i64,
    /// Downloaded bytes in the bucket.
    pub down: i64,
    /// Connection episodes contributing to the bucket.
    pub connections: i64,
}

/// Full U2 response payload.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Timeline {
    /// Requested bucket size.
    pub granularity: &'static str,
    /// Ascending series, empty when no samples exist.
    pub data: Vec<TimelineRow>,
}

struct Query {
    bucket: Bucket,
    since: i64,
    until: i64,
    limit: i64,
    interface_id: Option<i64>,
    app_id: Option<i64>,
}

/// Build the U2 payload for one request.
///
/// # Errors
///
/// `QUERY_INVALID` for malformed/contradictory parameters, `DB_ERROR` for
/// SQLite failures.
pub async fn timeline(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
) -> Result<Timeline, ApiError> {
    timeline_for_app(pool, now, raw_query, None).await
}

/// Build the A3 payload for one app (same shape as U2, app-scoped).
///
/// # Errors
///
/// `NOT_FOUND` for unknown apps; `QUERY_INVALID` for bad parameters;
/// `DB_ERROR` for SQLite failures.
pub async fn app_timeline(
    pool: &SqlitePool,
    now: i64,
    app_id: i64,
    raw_query: Option<&str>,
) -> Result<Timeline, ApiError> {
    if !crate::apps::app_exists(pool, app_id).await? {
        return Err(ApiError::not_found());
    }
    timeline_for_app(pool, now, raw_query, Some(app_id)).await
}

async fn timeline_for_app(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
    app_id: Option<i64>,
) -> Result<Timeline, ApiError> {
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let billing_day = crate::usage::setting(pool, "billing_cycle_day", "1")
        .await?
        .parse::<i64>()
        .unwrap_or(1);
    let tz = resolve_timezone(&timezone);
    let preset = crate::range::resolve(pool, raw_query).await?;
    let mut query = parse_query(raw_query, now, preset, tz, billing_day)?;
    query.app_id = app_id;
    let cutoff = raw_cutoff(pool, now).await?;

    let mut buckets = BTreeMap::new();
    if let Some(interface_id) = query.interface_id {
        load_interface_samples(pool, &mut buckets, &query, interface_id, tz).await?;
    } else {
        load_raw(pool, &mut buckets, &query, cutoff, tz).await?;
        load_rollups(pool, &mut buckets, &query, cutoff, tz).await?;
    }

    let mut data: Vec<TimelineRow> = buckets.into_values().collect();
    if data.len() > usize::try_from(query.limit).unwrap_or(usize::MAX) {
        let keep = usize::try_from(query.limit).unwrap_or(usize::MAX);
        data = data.split_off(data.len() - keep);
    }
    Ok(Timeline {
        granularity: granularity_name(query.bucket),
        data,
    })
}

/// Shared raw-window boundary for timeline and endpoint aggregation (API.md C2).
pub(crate) async fn raw_cutoff(pool: &SqlitePool, now: i64) -> Result<i64, ApiError> {
    let window = crate::usage::setting(pool, "raw_window_days", "7")
        .await?
        .parse::<i64>()
        .unwrap_or(7);
    Ok(now.saturating_sub(window.clamp(1, i64::MAX).saturating_mul(DAY_S)))
}

fn granularity_name(bucket: Bucket) -> &'static str {
    match bucket {
        Bucket::Minute15 => "15m",
        Bucket::Hour => "hour",
        Bucket::Day => "day",
    }
}

fn add_bucket(
    buckets: &mut BTreeMap<i64, TimelineRow>,
    bucket: i64,
    up: i64,
    down: i64,
    connections: i64,
) {
    buckets
        .entry(bucket)
        .and_modify(|row| {
            row.up = row.up.saturating_add(up);
            row.down = row.down.saturating_add(down);
            row.connections = row.connections.saturating_add(connections);
        })
        .or_insert(TimelineRow {
            bucket_start: bucket,
            up,
            down,
            connections,
        });
}

async fn load_raw(
    pool: &SqlitePool,
    buckets: &mut BTreeMap<i64, TimelineRow>,
    query: &Query,
    cutoff: i64,
    tz: chrono_tz::Tz,
) -> Result<(), ApiError> {
    let start = query.since.max(cutoff);
    if start >= query.until {
        return Ok(());
    }
    let rows = if let Some(app_id) = query.app_id {
        sqlx::query(
            "SELECT bytes_up, bytes_down, last_seen_at
               FROM connections
              WHERE last_seen_at >= ?1 AND last_seen_at < ?2
                AND app_id = ?3",
        )
        .bind(start)
        .bind(query.until)
        .bind(app_id)
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?
    } else {
        sqlx::query(
            "SELECT bytes_up, bytes_down, last_seen_at
               FROM connections
              WHERE last_seen_at >= ?1 AND last_seen_at < ?2",
        )
        .bind(start)
        .bind(query.until)
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?
    };
    for row in rows {
        let at: i64 = row
            .try_get("last_seen_at")
            .map_err(crate::history::db_error)?;
        add_bucket(
            buckets,
            bucket_start(at, query.bucket, tz),
            row.try_get("bytes_up").map_err(crate::history::db_error)?,
            row.try_get("bytes_down")
                .map_err(crate::history::db_error)?,
            1,
        );
    }
    Ok(())
}

async fn load_rollups(
    pool: &SqlitePool,
    buckets: &mut BTreeMap<i64, TimelineRow>,
    query: &Query,
    cutoff: i64,
    tz: chrono_tz::Tz,
) -> Result<(), ApiError> {
    let end = query.until.min(cutoff);
    if query.since >= end {
        return Ok(());
    }
    let rows = if let Some(app_id) = query.app_id {
        sqlx::query(
            "SELECT bucket_start, bytes_up, bytes_down, connection_count
               FROM usage_rollup_15m
              WHERE bucket_start >= ?1 AND bucket_start < ?2
                AND app_id = ?3",
        )
        .bind(query.since)
        .bind(end)
        .bind(app_id)
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?
    } else {
        sqlx::query(
            "SELECT bucket_start, bytes_up, bytes_down, connection_count
               FROM usage_rollup_15m
              WHERE bucket_start >= ?1 AND bucket_start < ?2",
        )
        .bind(query.since)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?
    };
    for row in rows {
        let source: i64 = row
            .try_get("bucket_start")
            .map_err(crate::history::db_error)?;
        add_bucket(
            buckets,
            bucket_start(source, query.bucket, tz),
            row.try_get("bytes_up").map_err(crate::history::db_error)?,
            row.try_get("bytes_down")
                .map_err(crate::history::db_error)?,
            row.try_get("connection_count")
                .map_err(crate::history::db_error)?,
        );
    }
    Ok(())
}

async fn load_interface_samples(
    pool: &SqlitePool,
    buckets: &mut BTreeMap<i64, TimelineRow>,
    query: &Query,
    interface_id: i64,
    tz: chrono_tz::Tz,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        "SELECT bucket_start, rx_bytes, tx_bytes
           FROM usage_interface_15m
          WHERE interface_id = ?1 AND bucket_start >= ?2 AND bucket_start < ?3",
    )
    .bind(interface_id)
    .bind(query.since)
    .bind(query.until)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    for row in rows {
        let source: i64 = row
            .try_get("bucket_start")
            .map_err(crate::history::db_error)?;
        let rx: i64 = row.try_get("rx_bytes").map_err(crate::history::db_error)?;
        let tx: i64 = row.try_get("tx_bytes").map_err(crate::history::db_error)?;
        // Counter tables have no episode count; zero is honest, not a guess.
        add_bucket(buckets, bucket_start(source, query.bucket, tz), tx, rx, 0);
    }
    Ok(())
}

fn parse_query(
    raw: Option<&str>,
    now: i64,
    preset: RangePreset,
    tz: chrono_tz::Tz,
    billing_day: i64,
) -> Result<Query, ApiError> {
    let mut bucket = preset.bucket();
    let mut since = None;
    let mut until = None;
    let mut limit = None;
    let mut interface_id = None;
    for (key, value) in crate::history::parse_query_params(raw.unwrap_or(""))? {
        match key.as_str() {
            "granularity" => {
                bucket = match value.as_str() {
                    "15m" => Bucket::Minute15,
                    "hour" => Bucket::Hour,
                    "day" => Bucket::Day,
                    other => {
                        return Err(ApiError::query_invalid(format!(
                            "invalid granularity: {other}"
                        )));
                    }
                };
            }
            "range" => {
                // Already validated by crate::range::resolve; the preset's
                // bucket/window are applied unless explicitly overridden.
            }
            "since" => since = Some(parse_int("since", &value)?),
            "until" => until = Some(parse_int("until", &value)?),
            "limit" => limit = Some(parse_int("limit", &value)?),
            "interface_id" => interface_id = Some(parse_int("interface_id", &value)?),
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    let default_limit = match bucket {
        Bucket::Minute15 => DEFAULT_MINUTE15_BUCKETS,
        Bucket::Hour => DEFAULT_HOUR_BUCKETS,
        Bucket::Day => DEFAULT_DAY_BUCKETS,
    };
    let period = preset.period(now, tz, billing_day);
    let until = until.unwrap_or(period.end.min(now));
    let since = since.unwrap_or(period.start);
    let limit = limit.unwrap_or(default_limit);
    if since >= until {
        return Err(ApiError::query_invalid("since must be earlier than until"));
    }
    if !(1..=LIMIT_MAX).contains(&limit) {
        return Err(ApiError::query_invalid(format!(
            "limit must be 1..={LIMIT_MAX}"
        )));
    }
    Ok(Query {
        bucket,
        since,
        until,
        limit,
        interface_id,
        app_id: None,
    })
}

fn parse_int(key: &str, value: &str) -> Result<i64, ApiError> {
    value
        .parse::<i64>()
        .map_err(|_| ApiError::query_invalid(format!("invalid query parameter '{key}': {value}")))
}
