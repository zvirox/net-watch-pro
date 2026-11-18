//! Interfaces I1/I2 (API.md §2.6): list all interfaces and update user-set
//! flags. `is_excluded` changes headline totals, so routes broadcast a status
//! refresh when it changes (API.md I2).

use serde_json::Value;
use sqlx::{QueryBuilder, Row, SqlitePool};

use crate::calendar::{day_period, resolve_timezone};
use crate::error::ApiError;
use crate::interfaces_usage::InterfaceBytes;

/// One I1 row: U3 shape plus the exclusion flag (API.md I1).
#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct InterfaceRecord {
    /// `interfaces.id`.
    pub id: i64,
    /// sysfs name.
    pub name: String,
    /// Stored link kind.
    pub kind: String,
    /// Metered label (D6a budgets use this).
    pub is_metered: bool,
    /// Whether counters are excluded from headline totals.
    pub is_excluded: bool,
    /// Current-day counter totals.
    pub bytes: InterfaceBytes,
}

/// List every interface, including excluded ones (API.md I1).
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn list(pool: &SqlitePool, now: i64) -> Result<Vec<InterfaceRecord>, ApiError> {
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let today = day_period(now, resolve_timezone(&timezone));
    fetch(pool, today.start, today.end, None).await
}

/// Update `is_metered` / `is_excluded` and return the full row (API.md I2).
///
/// Returns `(row, exclusion_changed)` so the route can broadcast a status
/// update only when totals semantics actually changed.
///
/// # Errors
///
/// `QUERY_INVALID` for unknown keys or non-boolean values, `NOT_FOUND` for an
/// unknown id, `DB_ERROR` for SQLite failures.
pub async fn update(
    pool: &SqlitePool,
    now: i64,
    id: i64,
    body: &Value,
) -> Result<(InterfaceRecord, bool), ApiError> {
    let object = body
        .as_object()
        .ok_or_else(|| ApiError::query_invalid("body must be a JSON object"))?;
    if object.is_empty() {
        return Err(ApiError::query_invalid("at least one flag is required"));
    }
    let mut meter = None;
    let mut exclude = None;
    for (key, value) in object {
        let flag = value
            .as_bool()
            .ok_or_else(|| ApiError::query_invalid(format!("'{key}' must be a boolean")))?;
        match key.as_str() {
            "is_metered" => meter = Some(flag),
            "is_excluded" => exclude = Some(flag),
            other => {
                return Err(ApiError::query_invalid(format!(
                    "unknown interface flag '{other}'"
                )));
            }
        }
    }

    let old_excluded: Option<bool> =
        sqlx::query_scalar("SELECT is_excluded FROM interfaces WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(crate::history::db_error)?
            .map(|value: i64| value != 0);
    if old_excluded.is_none() {
        return Err(ApiError::new(
            netwatch_common::ErrorCode::NotFound,
            axum::http::StatusCode::NOT_FOUND,
            "interface not found",
        ));
    }

    // Three valid shapes; static SQL avoids placeholder/separator mistakes.
    match (meter, exclude) {
        (Some(metered), Some(excluded)) => {
            sqlx::query(
                "UPDATE interfaces SET is_metered = ?1, is_excluded = ?2,
                        updated_at = strftime('%s','now') WHERE id = ?3",
            )
            .bind(i64::from(metered))
            .bind(i64::from(excluded))
            .bind(id)
            .execute(pool)
            .await
            .map_err(crate::history::db_error)?;
        }
        (Some(metered), None) => {
            sqlx::query(
                "UPDATE interfaces SET is_metered = ?1,
                        updated_at = strftime('%s','now') WHERE id = ?2",
            )
            .bind(i64::from(metered))
            .bind(id)
            .execute(pool)
            .await
            .map_err(crate::history::db_error)?;
        }
        (None, Some(excluded)) => {
            sqlx::query(
                "UPDATE interfaces SET is_excluded = ?1,
                        updated_at = strftime('%s','now') WHERE id = ?2",
            )
            .bind(i64::from(excluded))
            .bind(id)
            .execute(pool)
            .await
            .map_err(crate::history::db_error)?;
        }
        (None, None) => unreachable!("empty bodies are rejected above"),
    }

    let record = fetch_one(pool, now, id).await?;
    let exclusion_changed = exclude.is_some_and(|next| Some(next) != old_excluded);
    Ok((record, exclusion_changed))
}

async fn fetch_one(pool: &SqlitePool, now: i64, id: i64) -> Result<InterfaceRecord, ApiError> {
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let today = day_period(now, resolve_timezone(&timezone));
    let mut rows = fetch(pool, today.start, today.end, Some(id)).await?;
    rows.pop().ok_or_else(|| {
        ApiError::new(
            netwatch_common::ErrorCode::NotFound,
            axum::http::StatusCode::NOT_FOUND,
            "interface not found",
        )
    })
}

async fn fetch(
    pool: &SqlitePool,
    since: i64,
    until: i64,
    id: Option<i64>,
) -> Result<Vec<InterfaceRecord>, ApiError> {
    let mut builder = QueryBuilder::new(
        "SELECT i.id AS id, i.name AS name, i.kind AS kind,
                i.is_metered AS is_metered, i.is_excluded AS is_excluded,
                COALESCE(SUM(u.tx_bytes), 0) AS up,
                COALESCE(SUM(u.rx_bytes), 0) AS down
           FROM interfaces i
           LEFT JOIN usage_interface_15m u
             ON u.interface_id = i.id AND u.bucket_start >= ",
    );
    builder
        .push_bind(since)
        .push(" AND u.bucket_start < ")
        .push_bind(until);
    if let Some(id) = id {
        builder.push(" WHERE i.id = ").push_bind(id);
    }
    builder.push(" GROUP BY i.id, i.name, i.kind, i.is_metered, i.is_excluded");
    builder.push(" ORDER BY i.id");

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?;
    rows.iter()
        .map(|row| {
            Ok(InterfaceRecord {
                id: row.try_get("id").map_err(crate::history::db_error)?,
                name: row.try_get("name").map_err(crate::history::db_error)?,
                kind: row.try_get("kind").map_err(crate::history::db_error)?,
                is_metered: row
                    .try_get::<i64, _>("is_metered")
                    .map_err(crate::history::db_error)?
                    != 0,
                is_excluded: row
                    .try_get::<i64, _>("is_excluded")
                    .map_err(crate::history::db_error)?
                    != 0,
                bytes: InterfaceBytes {
                    up: row.try_get("up").map_err(crate::history::db_error)?,
                    down: row.try_get("down").map_err(crate::history::db_error)?,
                },
            })
        })
        .collect()
}
