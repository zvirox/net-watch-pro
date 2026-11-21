//! S3 `GET /diagnostics` and S4 `GET /events` (API.md §2.1).
//!
//! One payload for the Diagnostics modal: health checks, a short journal
//! excerpt from `daemon_events`, and DB metadata. Checks the local API process
//! cannot honestly perform (service-unit state, kernel compatibility) are
//! reported as `skip` with a `detail` — never guessed.

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::error::ApiError;
use netwatch_common::EbpfState;

/// One health check row (API.md S3).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Check {
    /// Stable check name (`service_unit`, `kernel`, `ebpf_attach`, `database`).
    pub name: &'static str,
    /// `pass` | `fail` | `skip`.
    pub result: &'static str,
    /// Human-readable context; required for `skip`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Optional self-help reference for a failed check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_url: Option<&'static str>,
}

/// One journal excerpt row (API.md S3).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct JournalRow {
    /// Epoch second of the event.
    pub occurred_at: i64,
    /// `info` / `warn` / `error`.
    pub level: String,
    /// Stable event code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// DB metadata for the modal (API.md S3).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DiagnosticsData {
    /// `page_count * page_size` from SQLite.
    pub db_size_bytes: i64,
    /// Configured retention in days.
    pub retention_days: i64,
}

/// Full S3 response.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DiagnosticsResponse {
    /// Health checks rendered as ✓ / ✗ / — in the modal.
    pub checks: Vec<Check>,
    /// Newest 40 `daemon_events` rows.
    pub journal_excerpt: Vec<JournalRow>,
    /// Database metadata.
    pub data: DiagnosticsData,
    /// Connection rejection statistics (Task 2).
    pub rejected_connections: crate::peer::RejectedConnections,
}

/// One S4 row.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EventRow {
    /// Stable event id (keyset cursor).
    pub id: i64,
    /// Epoch second of the event.
    pub occurred_at: i64,
    /// `info` / `warn` / `error`.
    pub level: String,
    /// Stable event code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional structured context as JSON text.
    pub detail_json: Option<String>,
}

/// S4 paginated envelope.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EventsEnvelope {
    /// Page rows, newest first.
    pub data: Vec<EventRow>,
    /// Opaque cursor for the next page.
    pub next_cursor: Option<String>,
    /// Whether more rows exist.
    pub has_more: bool,
}

fn skip(name: &'static str, detail: &str) -> Check {
    Check {
        name,
        result: "skip",
        detail: Some(detail.to_string()),
        guidance_url: None,
    }
}

/// Build the S3 payload from live status plus the diagnostics DB.
///
/// # Errors
///
/// `DB_ERROR` if the journal or DB metadata cannot be read.
pub async fn diagnostics(
    pool: &SqlitePool,
    ebpf: EbpfState,
    stats: &crate::peer::PeerStats,
) -> Result<DiagnosticsResponse, ApiError> {
    let ebpf_check = match ebpf {
        EbpfState::Attached => Check {
            name: "ebpf_attach",
            result: "pass",
            detail: None,
            guidance_url: None,
        },
        EbpfState::Failed | EbpfState::Unsupported => Check {
            name: "ebpf_attach",
            result: "fail",
            detail: None,
            guidance_url: Some("docs:kernel-btf"),
        },
        EbpfState::Unattached | EbpfState::NoCollector => skip(
            "ebpf_attach",
            "no collector attached — PC rate sampling still works (degraded mode)",
        ),
    };

    // BEGIN IMMEDIATE acquires the write lock without changing any row: a real
    // writability probe, not a guess from file permissions.
    let database = match sqlx::query("BEGIN IMMEDIATE").execute(pool).await {
        Ok(_) => {
            let _ = sqlx::query("ROLLBACK").execute(pool).await;
            Check {
                name: "database",
                result: "pass",
                detail: None,
                guidance_url: None,
            }
        }
        Err(err) => Check {
            name: "database",
            result: "fail",
            detail: Some(format!("write probe failed: {err}")),
            guidance_url: None,
        },
    };

    let page_count: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(pool)
        .await
        .map_err(crate::history::db_error)?;
    let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
        .fetch_one(pool)
        .await
        .map_err(crate::history::db_error)?;
    let retention_days = crate::usage::setting(pool, "retention_days", "90")
        .await?
        .parse::<i64>()
        .unwrap_or(90);

    let rows = sqlx::query(
        "SELECT occurred_at, level, code, message
           FROM daemon_events ORDER BY occurred_at DESC, id DESC LIMIT 40",
    )
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    let journal_excerpt = rows
        .iter()
        .map(|row| {
            Ok(JournalRow {
                occurred_at: row
                    .try_get("occurred_at")
                    .map_err(crate::history::db_error)?,
                level: row.try_get("level").map_err(crate::history::db_error)?,
                code: row.try_get("code").map_err(crate::history::db_error)?,
                message: row.try_get("message").map_err(crate::history::db_error)?,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(DiagnosticsResponse {
        checks: vec![
            skip(
                "service_unit",
                "service-unit state is not observable from the local API process",
            ),
            skip(
                "kernel",
                "kernel/BTF compatibility is not observable from the local API process",
            ),
            ebpf_check,
            database,
        ],
        journal_excerpt,
        data: DiagnosticsData {
            db_size_bytes: page_count.saturating_mul(page_size),
            retention_days,
        },
        rejected_connections: stats.snapshot(),
    })
}

/// Build the S4 page from `daemon_events` (newest first, keyset by id).
///
/// # Errors
///
/// `QUERY_INVALID` for bad parameters; `DB_ERROR` if the query fails.
pub async fn events(
    pool: &SqlitePool,
    raw_query: Option<&str>,
) -> Result<EventsEnvelope, ApiError> {
    let mut limit = 40_i64;
    let mut cursor: Option<i64> = None;
    let mut level: Option<String> = None;
    let mut code: Option<String> = None;
    for (key, value) in crate::history::parse_query_params(raw_query.unwrap_or(""))? {
        match key.as_str() {
            "limit" => {
                limit = value
                    .parse::<i64>()
                    .map_err(|_| ApiError::query_invalid(format!("invalid limit: {value}")))?;
            }
            "cursor" => {
                cursor =
                    Some(value.parse::<i64>().map_err(|_| {
                        ApiError::query_invalid(format!("invalid cursor: {value}"))
                    })?);
            }
            "level" => {
                if !["info", "warn", "error"].contains(&value.as_str()) {
                    return Err(ApiError::query_invalid(format!("invalid level: {value}")));
                }
                level = Some(value);
            }
            "code" => code = Some(value),
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    if !(1..=500).contains(&limit) {
        return Err(ApiError::query_invalid("limit must be 1..=500"));
    }

    // Fixed placeholder positions: absent filters bind NULL and match all.
    let rows = sqlx::query(
        "SELECT id, occurred_at, level, code, message, detail_json
           FROM daemon_events
          WHERE (?1 IS NULL OR id < ?1)
            AND (?2 IS NULL OR level = ?2)
            AND (?3 IS NULL OR code = ?3)
          ORDER BY id DESC LIMIT ?4",
    )
    .bind(cursor)
    .bind(level)
    .bind(code)
    .bind(limit + 1)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;

    let has_more = rows.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    let page: Vec<&sqlx::sqlite::SqliteRow> = rows.iter().take(take).collect();
    let data = page
        .iter()
        .map(|row| {
            Ok(EventRow {
                id: row.try_get("id").map_err(crate::history::db_error)?,
                occurred_at: row
                    .try_get("occurred_at")
                    .map_err(crate::history::db_error)?,
                level: row.try_get("level").map_err(crate::history::db_error)?,
                code: row.try_get("code").map_err(crate::history::db_error)?,
                message: row.try_get("message").map_err(crate::history::db_error)?,
                detail_json: row
                    .try_get("detail_json")
                    .map_err(crate::history::db_error)?,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let next_cursor = if has_more {
        data.last().map(|row| row.id.to_string())
    } else {
        None
    };
    Ok(EventsEnvelope {
        data,
        next_cursor,
        has_more,
    })
}
