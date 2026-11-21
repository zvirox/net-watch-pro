//! `GET /connections` — history episodes (API.md C2) with keyset pagination
//! and the raw-window seam: rows inside the raw window (`raw_window_days`
//! setting, default 7) come from `connections`; older rows come from
//! `usage_endpoint_day` aggregates, flagged `"kind":"aggregate"` (D2a).
//!
//! The two sources are scanned under one total order — sort column ±, then
//! all raw rows before all aggregate rows at equal sort values, then each
//! source's tiebreakers — and the opaque cursor records the last emitted
//! row's order key so the walk resumes exactly there (API.md §1.4: stable
//! order, opaque cursor, pass back verbatim).
//!
//! Free-tier rule (API.md C2): any of `since/until/remote_ip/remote_host/
//! remote_port`, or ≥2 filters at once, requires Pro → `403
//! EXPORT_PRO_REQUIRED` naming the offending filters (gate against the
//! `license` table, never UI-only — DATABASE.md §6).

use std::collections::HashMap;

use netwatch_common::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::ApiError;

/// Raw-window default (`raw_window_days` setting fallback; PRD §6.1 #5).
const RAW_WINDOW_DAYS_DEFAULT: i64 = 7;
/// Pagination bounds (API.md §1.4).
const LIMIT_DEFAULT: i64 = 50;
const LIMIT_MAX: i64 = 500;
/// Day length for aggregate-day gating (`bucket_start` is day-aligned UTC).
const DAY_S: i64 = 86_400;

// ---------------------------------------------------------------------------
// Query parsing
// ---------------------------------------------------------------------------

/// Sort whitelist (API.md C2). The column names are identical on both
/// sources, so one enum drives both scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    /// Episode start time.
    FirstSeenAt,
    /// Episode end / last activity time.
    LastSeenAt,
    /// Total uplink bytes.
    BytesUp,
    /// Total downlink bytes.
    BytesDown,
    /// Remote address (text sort).
    RemoteIp,
}

impl SortField {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "first_seen_at" => Some(Self::FirstSeenAt),
            "last_seen_at" => Some(Self::LastSeenAt),
            "bytes_up" => Some(Self::BytesUp),
            "bytes_down" => Some(Self::BytesDown),
            "remote_ip" => Some(Self::RemoteIp),
            _ => None,
        }
    }

    fn column(self) -> &'static str {
        match self {
            Self::FirstSeenAt => "first_seen_at",
            Self::LastSeenAt => "last_seen_at",
            Self::BytesUp => "bytes_up",
            Self::BytesDown => "bytes_down",
            Self::RemoteIp => "remote_ip",
        }
    }

    fn is_text(self) -> bool {
        matches!(self, Self::RemoteIp)
    }
}

/// Sort direction (default `desc` — newest/most-usage first, API.md §1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Ascending (oldest/smallest first).
    Asc,
    /// Descending (newest/largest first).
    Desc,
}

impl SortOrder {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "asc" => Some(Self::Asc),
            "desc" => Some(Self::Desc),
            _ => None,
        }
    }

    fn sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }

    /// Comparison operator against the cursor value in this direction.
    fn beyond_op(self) -> &'static str {
        match self {
            Self::Asc => ">",
            Self::Desc => "<",
        }
    }

    fn tie_op(self) -> &'static str {
        match self {
            Self::Asc => ">",
            Self::Desc => "<",
        }
    }
}

/// Parsed C2 query: filters, sort, limit, cursor.
#[derive(Debug, Clone)]
pub struct ConnectionsQuery {
    /// Filter: app id (exact).
    pub app_id: Option<i64>,
    /// Filter: app name (substring, free tier).
    pub app_name: Option<String>,
    /// Filter: open episodes only (raw window only).
    pub open: Option<bool>,
    /// Filter: IP protocol.
    pub protocol: Option<String>,
    /// Filter: interface id (raw window only).
    pub interface_id: Option<i64>,
    /// Filter: earliest activity second (Pro).
    pub since: Option<i64>,
    /// Filter: latest activity second (Pro).
    pub until: Option<i64>,
    /// Filter: remote address exact (Pro).
    pub remote_ip: Option<String>,
    /// Filter: remote hostname substring (Pro).
    pub remote_host: Option<String>,
    /// Filter: remote port exact (Pro).
    pub remote_port: Option<i64>,
    /// Sort column (whitelist).
    pub sort: SortField,
    /// Sort direction.
    pub order: SortOrder,
    /// Page size (1..=500, default 50).
    pub limit: i64,
    /// Opaque keyset cursor (API.md §1.4).
    pub cursor: Option<Cursor>,
}

/// Parse the raw query string into a validated C2 query.
///
/// # Errors
///
/// `QUERY_INVALID` (400) naming the offending key when a parameter is
/// unknown, malformed, or out of bounds (API.md §1.4).
pub fn parse_query(raw: &str) -> Result<ConnectionsQuery, ApiError> {
    let params = parse_query_params(raw)?;
    let mut q = ConnectionsQuery {
        app_id: None,
        app_name: None,
        open: None,
        protocol: None,
        interface_id: None,
        since: None,
        until: None,
        remote_ip: None,
        remote_host: None,
        remote_port: None,
        sort: SortField::LastSeenAt,
        order: SortOrder::Desc,
        limit: LIMIT_DEFAULT,
        cursor: None,
    };
    let mut raw_cursor: Option<String> = None;

    for (key, value) in &params {
        match key.as_str() {
            "limit" => {
                q.limit = value.parse::<i64>().map_err(|_| bad_limit(value))?;
            }
            "cursor" => {
                // Decoded after the loop — the sort field types the value.
                raw_cursor = Some(value.clone());
            }
            "sort" => {
                q.sort = SortField::parse(value)
                    .ok_or_else(|| bad_param("sort", &format!("unknown sort field: {value}")))?;
            }
            "order" => {
                q.order = SortOrder::parse(value)
                    .ok_or_else(|| bad_param("order", &format!("unknown order: {value}")))?;
            }
            "app_id" => q.app_id = Some(parse_int(key, value)?),
            "app_name" => q.app_name = Some(value.clone()),
            "open" => q.open = Some(parse_bool(key, value)?),
            "protocol" => {
                let protocol = value.clone();
                if !matches!(protocol.as_str(), "tcp" | "udp" | "icmp" | "other") {
                    return Err(bad_param(
                        "protocol",
                        &format!("unknown protocol: {protocol}"),
                    ));
                }
                q.protocol = Some(protocol);
            }
            "interface_id" => q.interface_id = Some(parse_int(key, value)?),
            "since" => q.since = Some(parse_int(key, value)?),
            "until" => q.until = Some(parse_int(key, value)?),
            "remote_ip" => q.remote_ip = Some(value.clone()),
            "remote_host" => q.remote_host = Some(value.clone()),
            "remote_port" => q.remote_port = Some(parse_int(key, value)?),
            other => return Err(bad_param(other, "unknown filter")),
        }
    }

    if !(1..=LIMIT_MAX).contains(&q.limit) {
        return Err(bad_param(
            "limit",
            &format!("limit must be 1..={LIMIT_MAX}"),
        ));
    }
    if let Some(raw) = raw_cursor {
        q.cursor = Some(decode_cursor(&raw, q.sort)?);
    }
    Ok(q)
}

/// Free-tier rule (API.md C2): advanced-set filters or ≥2 filters need Pro.
/// Returns the offending filter names when Pro would be required.
#[must_use]
pub fn pro_required_filters(q: &ConnectionsQuery) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    let mut count = 0usize;
    let mut check = |name: &str, present: bool, advanced_set: bool| {
        if present {
            count += 1;
            if advanced_set {
                named.push(name.to_string());
            }
        }
    };
    check("app_id", q.app_id.is_some(), false);
    check("app_name", q.app_name.is_some(), false);
    check("open", q.open.is_some(), false);
    check("protocol", q.protocol.is_some(), false);
    check("interface_id", q.interface_id.is_some(), false);
    check("since", q.since.is_some(), true);
    check("until", q.until.is_some(), true);
    check("remote_ip", q.remote_ip.is_some(), true);
    check("remote_host", q.remote_host.is_some(), true);
    check("remote_port", q.remote_port.is_some(), true);
    if count >= 2 {
        // ≥2 filters of any kind is the Pro *combined* filter set (§1.4).
        return vec!["combined filters (≥2)".to_string()];
    }
    named
}

fn parse_int(key: &str, value: &str) -> Result<i64, ApiError> {
    value
        .parse::<i64>()
        .map_err(|_| bad_param(key, &format!("not an integer: {value}")))
}

fn parse_bool(key: &str, value: &str) -> Result<bool, ApiError> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(bad_param(key, &format!("not a boolean: {other}"))),
    }
}

fn bad_param(key: &str, reason: &str) -> ApiError {
    ApiError::new(
        ErrorCode::QueryInvalid,
        axum::http::StatusCode::BAD_REQUEST,
        format!("invalid query parameter '{key}': {reason}"),
    )
}

fn bad_limit(value: &str) -> ApiError {
    bad_param("limit", &format!("not an integer: {value}"))
}

/// Minimal `a=b&c=d` split with percent-decoding — keeps the §1.3 error
/// envelope guaranteed (axum's Query extractor rejects with a plain 400).
pub(crate) fn parse_query_params(raw: &str) -> Result<HashMap<String, String>, ApiError> {
    let mut map = HashMap::new();
    for pair in raw.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key =
            percent_decode(key).ok_or_else(|| bad_param("key", "malformed percent-encoding"))?;
        let value =
            percent_decode(value).ok_or_else(|| bad_param(&key, "malformed percent-encoding"))?;
        map.insert(key, value);
    }
    Ok(map)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = bytes.get(i + 1..i + 3)?;
                // Both nibbles are < 16, so the byte is always in range.
                let hi = u8::try_from((hex[0] as char).to_digit(16)?).ok()?;
                let lo = u8::try_from((hex[1] as char).to_digit(16)?).ok()?;
                out.push(hi * 16 + lo);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

// ---------------------------------------------------------------------------
// Cursor codec (opaque per §1.4: hex(JSON), pass back verbatim)
// ---------------------------------------------------------------------------

/// Cursor order-key marker: which source produced the last row, its sort
/// value, and its source tiebreak (raw id / aggregate composite identity).
#[derive(Debug, Clone, PartialEq)]
pub struct Cursor {
    /// Which scan produced the last emitted row.
    pub source: CursorSource,
    /// Sort value at the last emitted row.
    pub value: CursorValue,
    /// Raw tiebreak: `connections.id`.
    pub raw_id: Option<i64>,
    /// Aggregate tiebreak: `(app_id, remote_ip, remote_port, protocol)`.
    pub agg_tie: Option<(i64, String, i64, String)>,
}

/// Which scan produced the cursor row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorSource {
    /// Row from `connections` (inside the raw window).
    Raw,
    /// Row from `usage_endpoint_day` (beyond the raw window).
    Aggregate,
}

/// Sort value at the cursor (integer for time/bytes columns, string for IP).
#[derive(Debug, Clone, PartialEq)]
pub enum CursorValue {
    /// Numeric sort column (time, bytes).
    Int(i64),
    /// Text sort column (remote address).
    Text(String),
}

#[derive(Debug, Deserialize, Serialize)]
struct CursorWire {
    src: u8,
    v: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rid: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    a: Option<(i64, String, i64, String)>,
}

impl Cursor {
    fn to_wire(&self) -> CursorWire {
        CursorWire {
            src: match self.source {
                CursorSource::Raw => 0,
                CursorSource::Aggregate => 1,
            },
            v: match &self.value {
                CursorValue::Int(v) => Value::from(*v),
                CursorValue::Text(v) => Value::from(v.clone()),
            },
            rid: self.raw_id,
            a: self.agg_tie.clone(),
        }
    }

    fn from_wire(wire: CursorWire, sort: SortField) -> Result<Self, ApiError> {
        let source = match wire.src {
            0 => CursorSource::Raw,
            1 => CursorSource::Aggregate,
            _ => return Err(bad_param("cursor", "unknown cursor source")),
        };
        let value = match (sort.is_text(), &wire.v) {
            (true, Value::String(s)) => CursorValue::Text(s.clone()),
            (false, Value::Number(n)) => {
                let v = n
                    .as_i64()
                    .ok_or_else(|| bad_param("cursor", "non-integer sort value"))?;
                CursorValue::Int(v)
            }
            _ => {
                return Err(bad_param(
                    "cursor",
                    "sort value type does not match the sort field",
                ));
            }
        };
        if source == CursorSource::Raw && wire.rid.is_none() {
            return Err(bad_param("cursor", "missing raw tiebreak"));
        }
        if source == CursorSource::Aggregate && wire.a.is_none() {
            return Err(bad_param("cursor", "missing aggregate tiebreak"));
        }
        Ok(Self {
            source,
            value,
            raw_id: wire.rid,
            agg_tie: wire.a,
        })
    }

    fn encode(&self) -> String {
        let json = serde_json::to_vec(&self.to_wire()).unwrap_or_default();
        hex_encode(&json)
    }
}

fn decode_cursor(value: &str, sort: SortField) -> Result<Cursor, ApiError> {
    let bytes = hex_decode(value).ok_or_else(|| bad_param("cursor", "not hex"))?;
    let wire: CursorWire = serde_json::from_slice(&bytes)
        .map_err(|_| bad_param("cursor", "malformed cursor payload"))?;
    Cursor::from_wire(wire, sort)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap_or('0'));
    }
    out
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            let hi = u8::try_from(s[i..].chars().next()?.to_digit(16)?).ok()?;
            let lo = u8::try_from(s[i + 1..].chars().next()?.to_digit(16)?).ok()?;
            Some((hi << 4) | lo)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Row + envelope shapes
// ---------------------------------------------------------------------------

/// One history row for the C2 envelope.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryRow {
    /// C1 row shape minus rates, plus packets — raw episodes carry `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// `"aggregate"` beyond the raw window (API.md C2); raw rows omit the flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    /// Owner app identity.
    pub app: HistoryApp,
    /// Process id (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<i64>,
    /// Process name (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    /// IP protocol (`tcp`/`udp`/`icmp`/`other`).
    pub protocol: String,
    /// Local address (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_ip: Option<String>,
    /// Local port (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_port: Option<i64>,
    /// Remote address.
    pub remote_ip: String,
    /// Remote port.
    pub remote_port: i64,
    /// Resolved hostname (V2 DNS display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_host: Option<String>,
    /// Uplink bytes.
    pub bytes_up: i64,
    /// Downlink bytes.
    pub bytes_down: i64,
    /// Uplink packets (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_up: Option<i64>,
    /// Downlink packets (raw rows only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_down: Option<i64>,
    /// Aggregate-only: episodes folded into this endpoint-day row.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_count: Option<i64>,
    /// Episode/day start.
    pub first_seen_at: i64,
    /// Episode/day end.
    pub last_seen_at: i64,
}

/// App identity block (C1 shape; `icon` is the icon handle, never bytes).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryApp {
    /// App id (joins `apps.id`).
    pub id: i64,
    /// Display name.
    pub name: String,
    /// Icon handle (never bytes; DESIGN.md §6.2 fallback when absent).
    pub icon: Option<String>,
}

/// C2 success envelope (§1.4): `next_cursor` omitted when `has_more` is false.
#[derive(Debug, serde::Serialize)]
pub struct HistoryEnvelope {
    /// This page of rows.
    pub data: Vec<HistoryRow>,
    /// Opaque cursor for the next page; absent when `has_more` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Whether more rows exist after this page.
    pub has_more: bool,
}

/// Order key carried alongside each fetched row for the merge and cursor.
#[derive(Debug, Clone)]
struct Fetched {
    row: HistoryRow,
    key: OrderKey,
}

#[derive(Debug, Clone)]
struct OrderKey {
    tag: u8, // 0 = raw, 1 = aggregate
    int_val: Option<i64>,
    text_val: Option<String>,
    raw_id: Option<i64>,
    agg_tie: Option<(i64, String, i64, String)>,
}

impl OrderKey {
    fn sort_less(&self, other: &Self, sort: SortField, order: SortOrder) -> bool {
        let cmp = if sort.is_text() {
            self.text_val
                .as_deref()
                .unwrap_or_default()
                .cmp(other.text_val.as_deref().unwrap_or_default())
        } else {
            self.int_val
                .unwrap_or_default()
                .cmp(&other.int_val.unwrap_or_default())
        };
        let cmp = match order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        };
        if cmp != std::cmp::Ordering::Equal {
            return cmp == std::cmp::Ordering::Less;
        }
        // Equal sort values: raw before aggregate; then source tiebreaks.
        if self.tag != other.tag {
            return self.tag < other.tag;
        }
        if self.tag == 0 {
            let cmp = self.raw_id.cmp(&other.raw_id);
            let cmp = match order {
                SortOrder::Asc => cmp,
                SortOrder::Desc => cmp.reverse(),
            };
            return cmp == std::cmp::Ordering::Less;
        }
        let tie = |k: &Self| {
            k.agg_tie
                .clone()
                .unwrap_or_else(|| (i64::MAX, String::new(), i64::MAX, String::new()))
        };
        tie(self) < tie(other)
    }

    fn to_cursor(&self, sort: SortField) -> Cursor {
        Cursor {
            source: if self.tag == 0 {
                CursorSource::Raw
            } else {
                CursorSource::Aggregate
            },
            value: if sort.is_text() {
                CursorValue::Text(self.text_val.clone().unwrap_or_default())
            } else {
                CursorValue::Int(self.int_val.unwrap_or_default())
            },
            raw_id: self.raw_id,
            agg_tie: self.agg_tie.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// SQL assembly
// ---------------------------------------------------------------------------

/// Bind value accumulator (order matters: SQL text and binds append together).
#[derive(Default)]
struct Sql {
    text: String,
    /// One ordered bind list — `?` placeholders and binds must stay in
    /// lockstep, so ints and texts cannot live in separate vectors.
    binds: Vec<Bind>,
}

/// Positional bind value (order matters).
enum Bind {
    Int(i64),
    Text(String),
}

impl Sql {
    fn push_int(&mut self, v: i64) {
        self.text.push('?');
        self.binds.push(Bind::Int(v));
    }
    fn push_text(&mut self, v: &str) {
        self.text.push('?');
        self.binds.push(Bind::Text(v.to_string()));
    }
    fn push_raw(&mut self, s: &str) {
        self.text.push_str(s); // validated enum fragments only — never user input
    }

    /// Bind the accumulated values in emission order.
    fn bind_all<'q>(
        &'q self,
        mut query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    ) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        for bind in &self.binds {
            query = match bind {
                Bind::Int(v) => query.bind(*v),
                Bind::Text(v) => query.bind(v.as_str()),
            };
        }
        query
    }
}

/// LIKE pattern with wildcard escaping (`app_name`/`remote_host` substring,
/// API.md C2).
#[must_use]
pub fn like_pattern(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('%');
    for c in value.chars() {
        match c {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('%');
    out
}

/// Apply the shared filters to a source whose alias is `c` (raw) or `e`
/// (aggregate), joined to `apps a`. Only filters the source can serve.
fn push_filters(sql: &mut Sql, q: &ConnectionsQuery, raw: bool) {
    let col = if raw { "c" } else { "e" };
    if let Some(app_id) = q.app_id {
        sql.push_raw(&format!(" AND {col}.app_id = "));
        sql.push_int(app_id);
    }
    if let Some(name) = &q.app_name {
        sql.push_raw(" AND a.name LIKE ");
        sql.push_text(&like_pattern(name));
        sql.push_raw(" ESCAPE '\\'");
    }
    if raw {
        if let Some(open) = q.open {
            sql.push_raw(" AND c.is_open = ");
            sql.push_int(i64::from(open));
        }
        if let Some(interface_id) = q.interface_id {
            sql.push_raw(" AND c.interface_id = ");
            sql.push_int(interface_id);
        }
        if let Some(host) = &q.remote_host {
            sql.push_raw(" AND c.remote_host LIKE ");
            sql.push_text(&like_pattern(host));
            sql.push_raw(" ESCAPE '\\'");
        }
    }
    if let Some(protocol) = &q.protocol {
        sql.push_raw(&format!(" AND {}.protocol = ", { col }));
        sql.push_text(protocol);
    }
    if let Some(ip) = &q.remote_ip {
        sql.push_raw(&format!(" AND {}.remote_ip = ", { col }));
        sql.push_text(ip);
    }
    if let Some(port) = q.remote_port {
        sql.push_raw(&format!(" AND {}.remote_port = ", { col }));
        sql.push_int(port);
    }
    // since/until bound the episode's activity span: overlap semantics —
    // an episode/day counts when it was active at any point in [since, until].
    if let Some(since) = q.since {
        sql.push_raw(&format!(" AND {}.last_seen_at >= ", { col }));
        sql.push_int(since);
    }
    if let Some(until) = q.until {
        sql.push_raw(&format!(" AND {}.first_seen_at <= ", { col }));
        sql.push_int(until);
    }
}

/// Keyset continuation predicate for one source, given the cursor row's
/// order key (see module docs for the total order the predicates encode).
fn push_keyset(sql: &mut Sql, cursor: &Cursor, sort: SortField, order: SortOrder) {
    let col = format!("c.{}", sort.column());
    let beyond = format!(" {col} {} ", order.beyond_op());
    let eq = format!(" {col} = ");
    let push_value = |sql: &mut Sql, value: &CursorValue| match value {
        CursorValue::Int(v) => sql.push_int(*v),
        CursorValue::Text(v) => sql.push_text(v),
    };

    match cursor.source {
        CursorSource::Raw => {
            // Raw rows at equal sort values sort before aggregates: when the
            // cursor sits on a raw row, aggregates at that value still come.
            sql.push_raw("(");
            sql.push_raw(&beyond);
            push_value(sql, &cursor.value);
            sql.push_raw(" OR (");
            sql.push_raw(&eq);
            push_value(sql, &cursor.value);
            // `c.id` — unqualified `id` is ambiguous against apps.id.
            sql.push_raw(&format!(" AND c.id {} ", order.tie_op()));
            sql.push_int(cursor.raw_id.unwrap_or_default());
            sql.push_raw("))");
        }
        CursorSource::Aggregate => {
            // Cursor on an aggregate: raw rows at equal values sort BEFORE
            // it (already emitted) — only strictly-beyond raws qualify.
            sql.push_raw(&beyond);
            push_value(sql, &cursor.value);
        }
    }
}

/// Aggregate-side keyset: like `push_keyset` but emits the aggregate-source
/// predicate (includes aggregates at the cursor's own sort value when the
/// cursor sits on a raw row, tie-broken by the composite identity).
fn push_keyset_agg(sql: &mut Sql, cursor: &Cursor, sort: SortField, order: SortOrder) {
    let col = format!("e.{}", sort.column());
    let beyond = format!(" {col} {} ", order.beyond_op());
    let eq = format!(" {col} = ");
    let push_value = |sql: &mut Sql, value: &CursorValue| match value {
        CursorValue::Int(v) => sql.push_int(*v),
        CursorValue::Text(v) => sql.push_text(v),
    };

    sql.push_raw("(");
    sql.push_raw(&beyond);
    push_value(sql, &cursor.value);
    if cursor.source == CursorSource::Raw {
        // All aggregates at the cursor's sort value come after its raw row.
        sql.push_raw(" OR (");
        sql.push_raw(&eq);
        push_value(sql, &cursor.value);
        sql.push_raw("))");
        return;
    }
    // Cursor on an aggregate: continue past its composite tiebreak (asc).
    // Flat disjunction — each term carries its own equality prefix, so the
    // paren balance is self-evident.
    let Some((app_id, ip, port, proto)) = cursor.agg_tie.clone() else {
        sql.push_raw(" OR 0)"); // unreachable: from_wire enforces the tie
        return;
    };
    sql.push_raw(" OR (");
    sql.push_raw(&format!("{col} = "));
    push_value(sql, &cursor.value);
    sql.push_raw(" AND ((e.app_id > ");
    sql.push_int(app_id);
    sql.push_raw(") OR (e.app_id = ");
    sql.push_int(app_id);
    sql.push_raw(" AND e.remote_ip > ");
    sql.push_text(&ip);
    sql.push_raw(") OR (e.app_id = ");
    sql.push_int(app_id);
    sql.push_raw(" AND e.remote_ip = ");
    sql.push_text(&ip);
    sql.push_raw(" AND e.remote_port > ");
    sql.push_int(port);
    sql.push_raw(") OR (e.app_id = ");
    sql.push_int(app_id);
    sql.push_raw(" AND e.remote_ip = ");
    sql.push_text(&ip);
    sql.push_raw(" AND e.remote_port = ");
    sql.push_int(port);
    sql.push_raw(" AND e.protocol > ");
    sql.push_text(&proto);
    // Close: disjunct #4, the disjunction group, the `(col = V AND ...)`
    // wrapper, and the outermost group opened at function start.
    sql.push_raw("))))");
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Serve C2: license gate → two source scans → merge → envelope.
///
/// # Errors
///
/// `QUERY_INVALID` for bad parameters, `EXPORT_PRO_REQUIRED` when the
/// free tier uses advanced filters, `DB_ERROR` for SQLite failures.
pub async fn connections_page(
    pool: &SqlitePool,
    raw_query: &str,
    now: i64,
) -> Result<HistoryEnvelope, ApiError> {
    let q = parse_query(raw_query)?;

    // Free-tier gate against the license table (DATABASE.md §6: gating is
    // never UI-only).
    let required = pro_required_filters(&q);
    if !required.is_empty() && !crate::license::is_pro(pool).await? {
        return Err(ApiError::new(
            ErrorCode::ExportProRequired,
            axum::http::StatusCode::FORBIDDEN,
            format!("Pro required for: {}", required.join(", ")),
        ));
    }

    let raw_window_days: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'raw_window_days'")
            .fetch_optional(pool)
            .await
            .map_err(db_error)?;
    let raw_window_days = raw_window_days
        .and_then(|v| v.parse().ok())
        .unwrap_or(RAW_WINDOW_DAYS_DEFAULT);
    let cutoff = now - raw_window_days * DAY_S;

    // Aggregate rows only for days fully beyond the raw window (the compactor
    // writes them exactly then; no double-count across the seam).
    let agg_open = q.open != Some(true) && q.interface_id.is_none() && q.remote_host.is_none();

    let mut raws = fetch_raw(pool, &q, cutoff).await?;
    let mut aggs = if agg_open {
        fetch_aggregates(pool, &q, cutoff).await?
    } else {
        Vec::new()
    };
    raws.sort_by(|a, b| {
        if a.key.sort_less(&b.key, q.sort, q.order) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    aggs.sort_by(|a, b| {
        if a.key.sort_less(&b.key, q.sort, q.order) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    // Merge the two sorted scans under the total order, take `limit`.
    let mut merged: Vec<Fetched> = Vec::with_capacity(raws.len() + aggs.len());
    let (mut ri, mut ai) = (0usize, 0usize);
    while ri < raws.len() || ai < aggs.len() {
        let take_raw = match (raws.get(ri), aggs.get(ai)) {
            (Some(r), Some(a)) => r.key.sort_less(&a.key, q.sort, q.order),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_raw {
            merged.push(raws[ri].clone());
            ri += 1;
        } else {
            merged.push(aggs[ai].clone());
            ai += 1;
        }
    }

    // limit is validated to 1..=500, and merged.len() fits comfortably.
    let limit = usize::try_from(q.limit).unwrap_or(500); // fallback can't trigger: validated 1..=500
    let has_more = merged.len() > limit;
    merged.truncate(limit);
    let next_cursor = if has_more {
        merged.last().map(|f| f.key.to_cursor(q.sort).encode())
    } else {
        None
    };

    Ok(HistoryEnvelope {
        data: merged.iter().map(|f| f.row.clone()).collect(),
        next_cursor,
        has_more,
    })
}

// Takes the error by value so it composes with `.map_err(db_error)`.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn db_error(e: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::DbError,
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("database error: {e}"),
    )
}

/// Raw scan: `connections` rows inside the raw window.
async fn fetch_raw(
    pool: &SqlitePool,
    q: &ConnectionsQuery,
    cutoff: i64,
) -> Result<Vec<Fetched>, ApiError> {
    let mut sql = Sql::default();
    sql.push_raw(
        "SELECT c.id AS id, c.app_id AS app_id, a.name AS app_name, \
         a.icon_path AS app_icon, c.pid AS pid, c.process_name AS process_name, \
         c.protocol AS protocol, c.local_ip AS local_ip, c.local_port AS local_port, \
         c.remote_ip AS remote_ip, c.remote_port AS remote_port, \
         c.remote_host AS remote_host, c.bytes_up AS bytes_up, \
         c.bytes_down AS bytes_down, c.packets_up AS packets_up, \
         c.packets_down AS packets_down, c.first_seen_at AS first_seen_at, \
         c.last_seen_at AS last_seen_at \
         FROM connections c JOIN apps a ON a.id = c.app_id \
         WHERE c.last_seen_at >= ",
    );
    sql.push_int(cutoff);
    push_filters(&mut sql, q, true);
    if let Some(cursor) = &q.cursor {
        sql.push_raw(" AND ");
        push_keyset(&mut sql, cursor, q.sort, q.order);
    }
    let dir = q.order.sql();
    sql.push_raw(&format!(
        " ORDER BY c.{0} {1}, c.id {1} LIMIT ",
        q.sort.column(),
        dir
    ));
    sql.push_int(q.limit + 1);

    let query = sql.bind_all(sqlx::query(&sql.text));
    let rows = query.fetch_all(pool).await.map_err(db_error)?;
    Ok(rows
        .into_iter()
        .map(|row| raw_fetched(&row, q.sort))
        .collect())
}

fn raw_fetched(row: &SqliteRow, sort: SortField) -> Fetched {
    // Order-key sort value comes from the same whitelisted column the scan
    // sorted by.
    let (int_val, text_val) = if sort.is_text() {
        (
            None,
            row.try_get::<Option<String>, _>(sort.column())
                .ok()
                .flatten(),
        )
    } else {
        (
            row.try_get::<Option<i64>, _>(sort.column()).ok().flatten(),
            None,
        )
    };
    Fetched {
        key: OrderKey {
            tag: 0,
            raw_id: row.try_get("id").ok(),
            int_val,
            text_val,
            agg_tie: None,
        },
        row: HistoryRow {
            id: row.try_get("id").ok(),
            kind: None,
            app: HistoryApp {
                id: row.try_get("app_id").unwrap_or_default(),
                name: row.try_get("app_name").unwrap_or_default(),
                icon: row.try_get("app_icon").ok(),
            },
            pid: row.try_get("pid").ok(),
            process_name: row.try_get("process_name").ok(),
            protocol: row.try_get("protocol").unwrap_or_default(),
            local_ip: row.try_get("local_ip").ok(),
            local_port: row.try_get("local_port").ok(),
            remote_ip: row.try_get("remote_ip").unwrap_or_default(),
            remote_port: row.try_get("remote_port").unwrap_or_default(),
            remote_host: row.try_get("remote_host").ok(),
            bytes_up: row.try_get("bytes_up").unwrap_or_default(),
            bytes_down: row.try_get("bytes_down").unwrap_or_default(),
            packets_up: row.try_get("packets_up").ok(),
            packets_down: row.try_get("packets_down").ok(),
            connection_count: None,
            first_seen_at: row.try_get("first_seen_at").unwrap_or_default(),
            last_seen_at: row.try_get("last_seen_at").unwrap_or_default(),
        },
    }
}

/// Aggregate scan: `usage_endpoint_day` rows for days fully beyond the
/// raw window.
async fn fetch_aggregates(
    pool: &SqlitePool,
    q: &ConnectionsQuery,
    cutoff: i64,
) -> Result<Vec<Fetched>, ApiError> {
    let mut sql = Sql::default();
    sql.push_raw(
        "SELECT e.app_id AS app_id, a.name AS app_name, a.icon_path AS app_icon, \
         e.remote_ip AS remote_ip, e.remote_port AS remote_port, e.protocol AS protocol, \
         e.bytes_up AS bytes_up, e.bytes_down AS bytes_down, \
         e.connection_count AS connection_count, e.first_seen_at AS first_seen_at, \
         e.last_seen_at AS last_seen_at \
         FROM usage_endpoint_day e JOIN apps a ON a.id = e.app_id \
         WHERE e.bucket_start + 86400 <= ",
    );
    sql.push_int(cutoff);
    push_filters(&mut sql, q, false);
    if let Some(cursor) = &q.cursor {
        sql.push_raw(" AND ");
        push_keyset_agg(&mut sql, cursor, q.sort, q.order);
    }
    sql.push_raw(&format!(
        " ORDER BY e.{0} {1}, e.app_id ASC, e.remote_ip ASC, e.remote_port ASC, e.protocol ASC LIMIT ",
        q.sort.column(),
        q.order.sql()
    ));
    sql.push_int(q.limit + 1);

    let query = sql.bind_all(sqlx::query(&sql.text));
    let rows = query.fetch_all(pool).await.map_err(db_error)?;
    Ok(rows
        .into_iter()
        .map(|row| agg_fetched(&row, q.sort))
        .collect())
}

fn agg_fetched(row: &SqliteRow, sort: SortField) -> Fetched {
    let app_id: i64 = row.try_get("app_id").unwrap_or_default();
    let remote_ip: String = row.try_get("remote_ip").unwrap_or_default();
    let remote_port: i64 = row.try_get("remote_port").unwrap_or_default();
    let protocol: String = row.try_get("protocol").unwrap_or_default();
    let (int_val, text_val) = if sort.is_text() {
        (
            None,
            row.try_get::<Option<String>, _>(sort.column())
                .ok()
                .flatten(),
        )
    } else {
        (
            row.try_get::<Option<i64>, _>(sort.column()).ok().flatten(),
            None,
        )
    };
    Fetched {
        key: OrderKey {
            tag: 1,
            raw_id: None,
            int_val,
            text_val,
            agg_tie: Some((app_id, remote_ip.clone(), remote_port, protocol.clone())),
        },
        row: HistoryRow {
            id: None,
            kind: Some("aggregate"),
            app: HistoryApp {
                id: app_id,
                name: row.try_get("app_name").unwrap_or_default(),
                icon: row.try_get("app_icon").ok(),
            },
            pid: None,
            process_name: None,
            protocol,
            local_ip: None,
            local_port: None,
            remote_ip,
            remote_port,
            remote_host: None,
            bytes_up: row.try_get("bytes_up").unwrap_or_default(),
            bytes_down: row.try_get("bytes_down").unwrap_or_default(),
            packets_up: None,
            packets_down: None,
            connection_count: row.try_get("connection_count").ok(),
            first_seen_at: row.try_get("first_seen_at").unwrap_or_default(),
            last_seen_at: row.try_get("last_seen_at").unwrap_or_default(),
        },
    }
}

// ---------------------------------------------------------------------------
// C3 — GET /connections/{id} (raw window only, API.md §2.2)
// ---------------------------------------------------------------------------

/// Serve C3: one raw episode by id, plus the retention wording (API.md C3:
/// pruned episodes answer 404 with "outside retention window" — the UI maps
/// that to the DESIGN.md §8.6 empty state instead of a generic error).
///
/// # Errors
///
/// `NOT_FOUND` when the id does not exist or falls outside the raw window,
/// `DB_ERROR` for SQLite failures.
pub async fn connection_by_id(
    pool: &SqlitePool,
    id: i64,
    now: i64,
) -> Result<HistoryRow, ApiError> {
    let raw_window_days: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'raw_window_days'")
            .fetch_optional(pool)
            .await
            .map_err(db_error)?;
    let raw_window_days = raw_window_days
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(RAW_WINDOW_DAYS_DEFAULT);
    let cutoff = now - raw_window_days * DAY_S;

    let row = sqlx::query(
        "SELECT c.id AS id, c.app_id AS app_id, a.name AS app_name, \
         a.icon_path AS app_icon, c.pid AS pid, c.process_name AS process_name, \
         c.protocol AS protocol, c.local_ip AS local_ip, c.local_port AS local_port, \
         c.remote_ip AS remote_ip, c.remote_port AS remote_port, \
         c.remote_host AS remote_host, c.bytes_up AS bytes_up, \
         c.bytes_down AS bytes_down, c.packets_up AS packets_up, \
         c.packets_down AS packets_down, c.first_seen_at AS first_seen_at, \
         c.last_seen_at AS last_seen_at \
         FROM connections c JOIN apps a ON a.id = c.app_id WHERE c.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?;

    match row {
        Some(row) if row.try_get::<i64, _>("last_seen_at").unwrap_or_default() < cutoff => {
            // Retention pruner deletes beyond the raw window; this row is
            // still here (between pruner runs) but is already out of contract.
            Err(ApiError::new(
                ErrorCode::NotFound,
                axum::http::StatusCode::NOT_FOUND,
                format!(
                    "connection episode {id} is outside the retention window \
                     (raw episodes are kept {raw_window_days} days; only daily \
                     aggregates remain for that period)"
                ),
            ))
        }
        Some(row) => Ok(raw_fetched(&row, SortField::LastSeenAt).row),
        None => Err(ApiError::new(
            ErrorCode::NotFound,
            axum::http::StatusCode::NOT_FOUND,
            format!("no connection episode with id {id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    // -- parsing ------------------------------------------------------------

    #[test]
    fn defaults_are_last_seen_desc_limit_50() {
        let q = parse_query("").unwrap();
        assert_eq!(q.sort, SortField::LastSeenAt);
        assert_eq!(q.order, SortOrder::Desc);
        assert_eq!(q.limit, 50);
        assert!(q.cursor.is_none());
    }

    #[test]
    fn unknown_filter_is_400_naming_the_key() {
        let err = parse_query("bogus=1").unwrap_err();
        assert_eq!(err.code(), ErrorCode::QueryInvalid);
        assert!(err.to_string().contains("bogus"), "{err}");
    }

    #[test]
    fn bad_sort_and_protocol_are_400() {
        assert!(parse_query("sort=nope").is_err());
        assert!(parse_query("protocol=gre").is_err());
        assert!(parse_query("limit=abc").is_err());
        assert!(parse_query("limit=501").is_err());
        assert!(parse_query("open=maybe").is_err());
    }

    #[test]
    fn percent_encoded_params_decode() {
        let q = parse_query("app_name=Fire%20fox&open=true").unwrap();
        assert_eq!(q.app_name.as_deref(), Some("Fire fox"));
        assert_eq!(q.open, Some(true));
    }

    // -- free-tier rule (API.md C2) -----------------------------------------

    #[test]
    fn single_basic_filter_is_free() {
        for raw in ["app_id=1", "open=true", "protocol=tcp"] {
            let named = pro_required_filters(&parse_query(raw).unwrap());
            assert_eq!(named, Vec::<String>::new(), "{raw}");
        }
    }

    #[test]
    fn any_advanced_filter_requires_pro() {
        for raw in [
            "since=1",
            "until=2",
            "remote_ip=1.2.3.4",
            "remote_host=x",
            "remote_port=443",
        ] {
            let named = pro_required_filters(&parse_query(raw).unwrap());
            assert_eq!(named.len(), 1, "{raw}");
        }
    }

    #[test]
    fn two_basic_filters_require_pro() {
        let named = pro_required_filters(&parse_query("app_id=1&open=false").unwrap());
        assert_eq!(named, vec!["combined filters (≥2)"]);
    }

    // -- cursor codec ---------------------------------------------------------

    fn cursor_roundtrip(cursor: &Cursor, sort: SortField) -> Cursor {
        let encoded = cursor.encode();
        decode_cursor(&encoded, sort).unwrap()
    }

    #[test]
    fn raw_cursor_roundtrips() {
        let cursor = Cursor {
            source: CursorSource::Raw,
            value: CursorValue::Int(1_731_570_600),
            raw_id: Some(88231),
            agg_tie: None,
        };
        assert_eq!(cursor_roundtrip(&cursor, SortField::LastSeenAt), cursor);
    }

    #[test]
    fn aggregate_cursor_roundtrips() {
        let cursor = Cursor {
            source: CursorSource::Aggregate,
            value: CursorValue::Text("151.101.1.1".to_string()),
            raw_id: None,
            agg_tie: Some((1, "151.101.1.1".to_string(), 443, "tcp".to_string())),
        };
        assert_eq!(cursor_roundtrip(&cursor, SortField::RemoteIp), cursor);
    }

    #[test]
    fn cursor_value_type_must_match_sort() {
        let encoded = Cursor {
            source: CursorSource::Raw,
            value: CursorValue::Text("x".to_string()),
            raw_id: Some(1),
            agg_tie: None,
        }
        .encode();
        assert!(decode_cursor(&encoded, SortField::LastSeenAt).is_err());
    }

    // -- order key / merge ----------------------------------------------------

    fn raw_key(int_val: i64, raw_id: i64) -> OrderKey {
        OrderKey {
            tag: 0,
            int_val: Some(int_val),
            text_val: None,
            raw_id: Some(raw_id),
            agg_tie: None,
        }
    }

    fn agg_key(int_val: i64, tie: (i64, &str, i64, &str)) -> OrderKey {
        OrderKey {
            tag: 1,
            int_val: Some(int_val),
            text_val: None,
            raw_id: None,
            agg_tie: Some((tie.0, tie.1.to_string(), tie.2, tie.3.to_string())),
        }
    }

    #[test]
    fn equal_sort_values_raw_before_aggregate() {
        let raw = raw_key(100, 5);
        let agg = agg_key(100, (1, "1.1.1.1", 443, "tcp"));
        assert!(raw.sort_less(&agg, SortField::LastSeenAt, SortOrder::Desc));
        assert!(!agg.sort_less(&raw, SortField::LastSeenAt, SortOrder::Desc));
    }

    #[test]
    fn desc_order_compares_ints_reversed() {
        let older = raw_key(100, 1);
        let newer = raw_key(200, 2);
        assert!(newer.sort_less(&older, SortField::LastSeenAt, SortOrder::Desc));
        assert!(older.sort_less(&newer, SortField::LastSeenAt, SortOrder::Asc));
    }

    #[test]
    fn raw_ties_break_by_id_in_sort_direction() {
        let a = raw_key(100, 1);
        let b = raw_key(100, 2);
        assert!(a.sort_less(&b, SortField::LastSeenAt, SortOrder::Asc));
        assert!(b.sort_less(&a, SortField::LastSeenAt, SortOrder::Desc));
    }

    #[test]
    fn aggregate_ties_break_by_composite_identity_asc() {
        let a = agg_key(100, (1, "1.1.1.1", 443, "tcp"));
        let b = agg_key(100, (1, "1.1.1.1", 443, "udp"));
        let c = agg_key(100, (2, "1.1.1.1", 443, "tcp"));
        assert!(a.sort_less(&b, SortField::LastSeenAt, SortOrder::Desc));
        assert!(b.sort_less(&c, SortField::LastSeenAt, SortOrder::Desc));
    }

    // -- C3 single episode (API.md §2.2) ---------------------------------------

    #[tokio::test]
    async fn c3_returns_the_episode_with_packets() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        insert_raw(&pool, 1, CUTOFF + 100, 1234, "1.1.1.1").await;
        let row = connection_by_id(&pool, 1, NOW).await.unwrap();
        assert_eq!(row.id, Some(1));
        assert_eq!(row.app.name, "Firefox");
        assert_eq!(row.bytes_up, 1234);
        assert!(row.packets_up.is_some(), "C3 = C1 shape + packets");
        assert!(
            row.first_seen_at > 0,
            "history rows carry the activity span"
        );
    }

    #[tokio::test]
    async fn c3_unknown_id_is_404() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        insert_raw(&pool, 1, CUTOFF + 100, 1, "1.1.1.1").await;
        let err = connection_by_id(&pool, 999, NOW).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotFound);
        assert!(
            !err.to_string().contains("retention"),
            "plain 404 for fake ids: {err}"
        );
    }

    #[tokio::test]
    async fn c3_beyond_raw_window_404_names_retention() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        insert_raw(&pool, 1, CUTOFF - 100, 1, "1.1.1.1").await;
        let err = connection_by_id(&pool, 1, NOW).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotFound);
        assert!(
            err.to_string().contains("outside the retention window"),
            "pruned wording per API.md C3: {err}"
        );
    }

    // -- DB-backed pagination walk (raw + aggregate seam) ----------------------

    async fn db() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE license (id INTEGER PRIMARY KEY CHECK (id = 1), tier TEXT NOT NULL DEFAULT 'free')")
            .execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('raw_window_days', '7')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO license (id, tier) VALUES (1, 'free')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE apps (id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon_path TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE connections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_id INTEGER NOT NULL, pid INTEGER NOT NULL, process_name TEXT,
                protocol TEXT NOT NULL, local_ip TEXT NOT NULL, local_port INTEGER NOT NULL,
                remote_ip TEXT NOT NULL, remote_port INTEGER NOT NULL, remote_host TEXT,
                bytes_up INTEGER NOT NULL DEFAULT 0, bytes_down INTEGER NOT NULL DEFAULT 0,
                packets_up INTEGER NOT NULL DEFAULT 0, packets_down INTEGER NOT NULL DEFAULT 0,
                first_seen_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL,
                is_open INTEGER NOT NULL DEFAULT 0, interface_id INTEGER)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE usage_endpoint_day (
                app_id INTEGER NOT NULL, bucket_start INTEGER NOT NULL,
                remote_ip TEXT NOT NULL, remote_port INTEGER NOT NULL, protocol TEXT NOT NULL,
                bytes_up INTEGER NOT NULL DEFAULT 0, bytes_down INTEGER NOT NULL DEFAULT 0,
                connection_count INTEGER NOT NULL DEFAULT 0,
                first_seen_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL,
                PRIMARY KEY (app_id, bucket_start, remote_ip, remote_port, protocol))",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    async fn insert_app(pool: &SqlitePool, id: i64, name: &str) {
        sqlx::query("INSERT INTO apps (id, name) VALUES (?, ?)")
            .bind(id)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn insert_raw(
        pool: &SqlitePool,
        app_id: i64,
        last_seen: i64,
        bytes_up: i64,
        remote: &str,
    ) {
        sqlx::query(
            "INSERT INTO connections (app_id, pid, protocol, local_ip, local_port,
             remote_ip, remote_port, bytes_up, bytes_down, first_seen_at, last_seen_at)
             VALUES (?, 1, 'tcp', '10.0.0.2', 5, ?, 443, ?, 0, ?, ?)",
        )
        .bind(app_id)
        .bind(remote)
        .bind(bytes_up)
        .bind(last_seen - 60)
        .bind(last_seen)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_agg(
        pool: &SqlitePool,
        app_id: i64,
        bucket_start: i64,
        bytes_up: i64,
        remote: &str,
        count: i64,
    ) {
        sqlx::query(
            "INSERT INTO usage_endpoint_day (app_id, bucket_start, remote_ip, remote_port,
             protocol, bytes_up, bytes_down, connection_count, first_seen_at, last_seen_at)
             VALUES (?, ?, ?, 443, 'tcp', ?, 0, ?, ?, ?)",
        )
        .bind(app_id)
        .bind(bucket_start)
        .bind(remote)
        .bind(bytes_up)
        .bind(count)
        .bind(bucket_start)
        .bind(bucket_start + 86_400 - 1)
        .execute(pool)
        .await
        .unwrap();
    }

    const NOW: i64 = 1_800_000_000;
    /// Raw window: 7 days → cutoff seconds.
    const CUTOFF: i64 = NOW - 7 * 86_400;

    #[tokio::test]
    async fn raw_rows_only_when_all_inside_window() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        for i in 0..5 {
            insert_raw(&pool, 1, CUTOFF + 100 + i, 1000 + i, "1.1.1.1").await;
        }
        let page = connections_page(&pool, "", NOW).await.unwrap();
        assert_eq!(page.data.len(), 5);
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
        assert!(page.data.iter().all(|row| row.kind.is_none()));
        // Default sort last_seen_at desc → newest first.
        let seen: Vec<i64> = page.data.iter().map(|r| r.last_seen_at).collect();
        let mut sorted = seen.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(seen, sorted);
    }

    #[tokio::test]
    async fn pagination_walk_returns_stable_descending_order() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        for i in 0..12 {
            insert_raw(&pool, 1, CUTOFF + 100 + i, 1000, "1.1.1.1").await;
        }
        let mut collected: Vec<i64> = Vec::new();
        let mut cursor = String::new();
        loop {
            let raw = if cursor.is_empty() {
                "limit=5".to_string()
            } else {
                format!("limit=5&cursor={cursor}")
            };
            let page = connections_page(&pool, &raw, NOW).await.unwrap();
            collected.extend(page.data.iter().map(|r| r.last_seen_at));
            if page.has_more {
                cursor = page.next_cursor.unwrap();
            } else {
                break;
            }
        }
        assert_eq!(collected.len(), 12);
        let mut sorted = collected.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(collected, sorted, "stable descending walk, no dupes");
    }

    #[tokio::test]
    async fn walk_crosses_the_raw_aggregate_seam() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        // Raw rows inside the window.
        for i in 0..3 {
            insert_raw(&pool, 1, CUTOFF + 100 + i, 1000, "1.1.1.1").await;
        }
        // Aggregate days fully beyond the window (day ends before cutoff).
        let day = (CUTOFF / 86_400 - 1) * 86_400;
        for i in 0..3 {
            insert_agg(&pool, 1, day - i * 86_400, 5_000, "1.1.1.1", 4).await;
        }
        let mut kinds: Vec<String> = Vec::new();
        let mut cursor = String::new();
        loop {
            let raw = if cursor.is_empty() {
                "limit=2".to_string()
            } else {
                format!("limit=2&cursor={cursor}")
            };
            let page = connections_page(&pool, &raw, NOW).await.unwrap();
            for row in &page.data {
                kinds.push(row.kind.unwrap_or("raw").to_string());
            }
            if page.has_more {
                cursor = page.next_cursor.unwrap();
            } else {
                break;
            }
        }
        assert_eq!(
            kinds,
            vec!["raw", "raw", "raw", "aggregate", "aggregate", "aggregate"]
        );
        // Aggregates are flagged and carry the fold count, not packets.
        let page = connections_page(&pool, "limit=10", NOW).await.unwrap();
        let agg = page
            .data
            .iter()
            .find(|r| r.kind == Some("aggregate"))
            .unwrap();
        assert_eq!(agg.connection_count, Some(4));
        assert!(agg.packets_up.is_none());
        assert!(agg.id.is_none());
    }

    #[tokio::test]
    async fn advanced_filter_is_pro_gated_403() {
        let pool = db().await;
        let err = connections_page(&pool, "since=100", NOW).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::ExportProRequired);
        let err = connections_page(&pool, "app_id=1&open=true", NOW)
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::ExportProRequired);
    }

    #[tokio::test]
    async fn pro_license_unlocks_advanced_filters() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        insert_raw(&pool, 1, CUTOFF + 100, 1000, "9.9.9.9").await;
        sqlx::query("UPDATE license SET tier = 'pro' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        let page = connections_page(&pool, "since=0&remote_ip=9.9.9.9", NOW)
            .await
            .unwrap();
        assert_eq!(page.data.len(), 1);
    }

    #[tokio::test]
    async fn basic_filters_apply() {
        let pool = db().await;
        insert_app(&pool, 1, "Firefox").await;
        insert_app(&pool, 2, "Chromium").await;
        insert_raw(&pool, 1, CUTOFF + 100, 1000, "1.1.1.1").await;
        insert_raw(&pool, 2, CUTOFF + 200, 1000, "2.2.2.2").await;
        let page = connections_page(&pool, "app_name=fire", NOW).await.unwrap();
        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].app.name, "Firefox");
        let page = connections_page(&pool, "app_id=2", NOW).await.unwrap();
        assert_eq!(page.data[0].app.id, 2);
    }

    #[tokio::test]
    async fn search_wildcard_escaping_treats_special_characters_literally() {
        let pool = db().await;
        insert_app(&pool, 1, "App 100%").await;
        insert_app(&pool, 2, "App Other").await;
        insert_app(&pool, 3, "a_b").await;
        insert_app(&pool, 4, "axb").await;
        insert_raw(&pool, 1, CUTOFF + 100, 1000, "1.1.1.1").await;
        insert_raw(&pool, 2, CUTOFF + 200, 1000, "2.2.2.2").await;
        insert_raw(&pool, 3, CUTOFF + 300, 1000, "3.3.3.3").await;
        insert_raw(&pool, 4, CUTOFF + 400, 1000, "4.4.4.4").await;

        // Searching "100%" must not match every row; only "App 100%"
        let page_percent = connections_page(&pool, "app_name=100%25", NOW)
            .await
            .unwrap();
        assert_eq!(page_percent.data.len(), 1);
        assert_eq!(page_percent.data[0].app.name, "App 100%");

        // Searching "a_b" must not match "axb"; only "a_b"
        let page_underscore = connections_page(&pool, "app_name=a_b", NOW).await.unwrap();
        assert_eq!(page_underscore.data.len(), 1);
        assert_eq!(page_underscore.data[0].app.name, "a_b");
    }
}
