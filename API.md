# Net Watch Pro — API.md (Local API Reference)

**Version:** 1.2 · **Date:** 2026-11-15  
**Source of truth:** `PRD.md` (features, freemium gating) · `ARCHITECTURE.md` §5.3 (transport contract) · `DATABASE.md` (tables) · `docs/decisions.md` (owner decisions D1a–D5a)

**Version history:** 1.2 (2026-11-15): Unix-socket + peer-credential transport replaces loopback+token (F-06/D1a); usage_endpoint_day aggregate shape beyond raw window (F-02/D2a); /usage/summary returns total/attributed/unattributed (F-03/D3a); timezone/cycle behavior documented (F-04/D4a).

> **Binding:** every endpoint below maps to a PRD feature and a DATABASE.md table (§10 traceability matrix). Anything flagged as not doing so is explicitly justified there. Implementation of this file lives in `docs/api-contract.md` (ARCHITECTURE.md §4) and must not drift.

---

## 1. General Conventions

### 1.1 Auth & roles

| Role | Who | What |
|---|---|---|
| `anonymous` | any local process | `GET /health` only (bootstrapping) |
| `local-ui` | UI process + any local tool holding the per-boot token | everything else |

- **Mechanism:** peer-credential (`SO_PEERCRED`) authorization on the Unix socket — the daemon checks the connecting process's UID against a root-owned allowed-UID list written during the one-time onboarding pkexec step (D1a). No token, no bearer header, no Polkit helper, no per-launch prompt.
- **Pro entitlement is NOT a role.** It is a license-tier check against `DATABASE.md §2.7` enforced server-side (PRD R7). Gated endpoints return `403 EXPORT_PRO_REQUIRED` (see §2.3).
- **Yes/No per endpoint** is stated in each group's table ("Auth: yes (local-ui)" / "no").

### 1.2 Standard request headers

| Header | Required | Notes |
|---|---|---|
| `Authorization: Bearer <token>` | **removed** | Auth is now peer-credential on the Unix socket (D1a); no bearer header needed |
| `Content-Type: application/json` | yes on requests with a body | |
| `Accept` | optional | `application/json` (default) or `text/csv` on export content |
| `X-Request-Id` | optional, client-generated | echoed in errors for log correlation |

All responses are JSON (`application/json; charset=utf-8`) unless downloading export content.

### 1.3 Standard error format (used everywhere, no exceptions)

```json
{
  "error": {
    "code": "QUERY_INVALID",
    "message": "Unknown filter key 'proto'. Allowed: protocol.",
    "details": { "field": "proto" },
    "request_id": "b3a1f2c4"
  }
}
```

- `code`: stable string from ARCHITECTURE.md §7.2 (`DAEMON_UNREACHABLE`, `AUTH_INVALID`, `EBPF_ATTACH_FAILED`, `EBPF_UNSUPPORTED_KERNEL`, `PERMISSION_DENIED`, `DB_ERROR`, `QUERY_INVALID`, `EXPORT_PRO_REQUIRED`, `LICENSE_INVALID`, `RATE_LIMITED`) plus `NOT_FOUND`, `CONFLICT`, `METHOD_NOT_ALLOWED`.
- `message`: human-readable, safe to display (maps to DESIGN.md §7.3 treatments).
- `request_id`: echoes `X-Request-Id` or a generated id.
- **Status-code mapping (fixed):**

| Status | Codes |
|---|---|
| 400 | `QUERY_INVALID` |
| 401 | `AUTH_INVALID` (UID not in allowed-UID list; peer-cred check failed) |
| 403 | `PERMISSION_DENIED`, `EXPORT_PRO_REQUIRED`, `LICENSE_INVALID` |
| 404 | `NOT_FOUND` |
| 405 | `METHOD_NOT_ALLOWED` |
| 409 | `CONFLICT` |
| 429 | `RATE_LIMITED` |
| 500 | `DB_ERROR` |
| 503 | `DAEMON_UNREACHABLE`, `EBPF_ATTACH_FAILED`, `EBPF_UNSUPPORTED_KERNEL` |

### 1.4 Pagination, filtering, sorting

- **Pagination:** keyset cursor for high-volume resources (`connections`, `events`): `?limit=&cursor=` — response envelope `{ "data": [...], "next_cursor": "…", "has_more": true }`. `limit` default **50**, max **500**; `next_cursor` is opaque, pass back verbatim; omit when `has_more=false`. Small resources (`apps`, `interfaces`, `settings`, `exports`) return plain arrays — they never paginate (bounded in practice).
- **Filtering:** query params per endpoint (whitelist). Unknown or malformed params → `400 QUERY_INVALID` naming the offending key. Basic filters (app name/id, open-only) are free-tier; the *combined* advanced filter set (date range + remote host + port + protocol) is Pro-gated per PRD §6.1 #9 → `403 EXPORT_PRO_REQUIRED`.
- **Sorting:** `?sort=<field>&order=asc|desc` (default `desc` where sensible). Only fields from the endpoint's whitelist; anything else → 400.

### 1.5 Rate limits

Single trusted local client over a Unix socket, so limits are minimal and exist mainly to prevent runaway loops from a misbehaving UI:

| Scope | Limit | Exceeded |
|---|---|---|
| Read endpoints | 600 req/min per socket | 429 + `Retry-After` |
| Export creation | 5/min | 429 |
| Settings/license writes | 10/min | 429 |
| Export content download | 10/min | 429 |

429 responses use the §1.3 error format with code `RATE_LIMITED` and `details.retry_after_s`. Exceeding does not count as a security event (local only) but is surfaced in Diagnostics.

### 1.6 Versioning rules

1. **Path version:** `/api/v1/…` — the major version is part of every URL (ARCHITECTURE.md §5.3).
2. **Additive-only within v1:** new fields, new optional params, new endpoints are minor changes; **clients must ignore unknown fields** (forward-compatible by contract, ARCHITECTURE.md §5.3).
3. **Breaking changes** (removing/renaming fields, changing semantics, changing auth) require `/api/v2` and a deprecation window: v1 keeps working, responses add `Deprecation` + `Sunset` headers for 90 days, and the UI shows a one-time upgrade banner (DESIGN.md §7.9).
4. **No version negotiation headers** — the URL is the version. Simple and inspectable.

---
## 2. Endpoints — grouped by resource

### 2.1 System & Status

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| S1 | `GET /api/v1/health` | **no** | §6.1 #6 daemon health / Flow 5 | *(derived; no table — flagged §10)* |
| S2 | `GET /api/v1/status` | yes | §6.1 #6 degraded states → DESIGN.md chip | `daemon_events` (evidence) + derived state |
| S3 | `GET /api/v1/diagnostics` | yes | §6.1 #6 Diagnostics modal (DESIGN.md §8.9) | `daemon_events`, `interfaces`, `settings` |
| S4 | `GET /api/v1/events` | yes | §6.1 #6 diagnostics journal excerpt | `daemon_events` |

**S1 `GET /health`** — bootstrapping liveness. Success 200: `{"status":"ok","version":"1.0.0","api":"v1"}`. Errors: none (always 200 or connection refused).

**S2 `GET /status`** — drives the status chip + banners. Success 200:
```json
{ "daemon": "monitoring", "ebpf": "attached", "recording": true,
  "ws_clients": 1, "uptime_s": 3600, "drops": { "ring_buffer_dropped": 0 } }
```
`daemon`: `monitoring | degraded | stopped | starting` (maps 1:1 to DESIGN.md §7.5 chip states). Errors: 503 `EBPF_ATTACH_FAILED`/`EBPF_UNSUPPORTED_KERNEL` (degraded, body still returned with state), 401.

**S3 `GET /diagnostics`** — one payload for the Diagnostics modal. Success 200:
```json
{ "checks": [ {"name":"service_unit","result":"pass"},
              {"name":"ebpf_attach","result":"fail","guidance_url":"docs:kernel-btf"},
              {"name":"database","result":"pass","detail":"writable, 2.3 GB"} ],
  "journal_excerpt": [ {"occurred_at":1731570600,"level":"error","code":"EBPF_ATTACH_FAILED","message":"…"} ],
  "data": {"db_size_bytes":2469606195, "retention_days":90} }
```
Errors: 401, 429.

**S4 `GET /events`** — paginated `daemon_events`. Query: `?limit=&cursor=&level=warn|error&code=`. Success 200: envelope with rows `{id, occurred_at, level, code, message, detail_json}`. Errors: 400, 401, 429.

### 2.2 Connections (live + history)

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| C1 | `GET /api/v1/connections/live` | yes | §6.1 #1 Live Connections | `connections` (`is_open=1`) + in-memory deltas *(flagged §10)* |
| C2 | `GET /api/v1/connections` | yes | §6.1 #5 History (basic free / advanced Pro) | `connections` |
| C3 | `GET /api/v1/connections/{id}` | yes | §6.1 #5 drill-down | `connections` |

**C1 `GET /connections/live`** — one snapshot; the UI refreshes by re-polling (cheap; the streaming experience comes from WS `live.connections`, §3). Query: `?sort=rate&order=desc` (`rate|bytes|app|remote`), `?protocol=`, `?app_id=`. Success 200:
```json
{ "captured_at": 1731570600, "frozen": false,
  "data": [ { "id": 88231, "app": {"id":1,"name":"Firefox","icon":"ic:1"},
              "pid":2213,"process_name":"firefox","protocol":"tcp",
              "local_ip":"192.168.1.23","local_port":53210,
              "remote_ip":"151.101.1.1","remote_port":443,"remote_host":null,
              "rate_up_bps":40960,"rate_down_bps":262144,
              "bytes_up":240000,"bytes_down":1980000 } ] }
```
Errors: 400, 401, 429, 503 (daemon starting).

**C2 `GET /connections`** — history episodes, keyset-paginated. Filters (whitelist):
`app_id`, `app_name` (substring, free), `since`/`until` (epoch, Pro-set), `remote_ip`, `remote_host` (substring, Pro-set), `remote_port`, `protocol`, `interface_id`, `open` (bool). Sort whitelist: `first_seen_at`, `last_seen_at`, `bytes_up`, `bytes_down`, `remote_ip`. **Rule:** requests using ≥2 filters, or any of `since/until/remote_ip/remote_host/remote_port`, require Pro → otherwise `403 EXPORT_PRO_REQUIRED` with the missing entitlement named in `message`. **Beyond the raw window (7 days default), rows are aggregate rows from `usage_endpoint_day` (flagged `"kind": "aggregate"`), not connection episodes.** C3 `GET /connections/{id}` stays raw-window only. Success 200: envelope; rows shaped like C1 minus rates, plus `packets_up/packets_down` (raw) or `bytes_up/bytes_down/connection_count` (aggregate).
Errors: 400 `QUERY_INVALID`, 401, 403, 429, 500 `DB_ERROR`.

**C3 `GET /connections/{id}`** — single episode. Success 200: one row (C1 shape + packets). Errors: 404 `NOT_FOUND` (also when pruned by retention — message states "outside retention window"), 401, 429.

### 2.3 Exports (Pro)

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| E1 | `POST /api/v1/exports` | yes + **Pro** | §6.1 #9 CSV/JSON export (Flow 4) | `connections` + `usage_rollup_*` |
| E2 | `GET /api/v1/exports/{id}` | yes + **Pro** | §6.1 #9 | *(job state — in-memory; flagged §10)* |
| E3 | `GET /api/v1/exports/{id}/content` | yes + **Pro** | §6.1 #9 | `connections` / `usage_rollup_*` |

**E1 `POST /exports`** — creates an export job from a filter set (same whitelist as C2; at least one filter required to prevent accidental full dumps).
Request body:
```json
{ "scope": "connections", "format": "csv",
  "filters": { "app_id": 1, "since": 1731565200, "until": 1731570600 } }
```
`scope`: `connections | usage_hourly | usage_daily`; `format`: `csv | json`. Success **202**:
```json
{ "id": "exp_7f3a", "state": "queued", "expires_in_s": 600,
  "estimated_rows": 48211 }
```
Errors: 400 (empty/invalid filters), 401, 403 `EXPORT_PRO_REQUIRED`, 429, 500.

**E2 `GET /exports/{id}`** — job status. Success 200: `{"id":"exp_7f3a","state":"done","rows":48211,"size_bytes":2318541}` (`queued|running|done|expired`). Errors: 404, 403, 401, 429.

**E3 `GET /exports/{id}/content`** — the file. `Accept: text/csv` or `application/json`; responds `200` with streamed body + `Content-Disposition: attachment; filename="netwatch-pro-export-20261114.csv"`. Errors: 404 (or expired → 404 + message), 403, 401, 429. Client-side file saving is the UI's native dialog (PRD Flow 4) — the API never writes files.
### 2.4 Apps

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| A1 | `GET /api/v1/apps` | yes | §6.1 #2 per-app usage ranking / Top apps | `apps` (+ rollup join) |
| A2 | `GET /api/v1/apps/{id}` | yes | §6.1 #4 App detail page | `apps` |
| A3 | `GET /api/v1/apps/{id}/usage` | yes | §6.1 #4 per-app usage over time | `usage_rollup_hour`, `usage_rollup_day`, `connections` (raw window) |
| A4 | `GET /api/v1/apps/{id}/endpoints` | yes | §6.1 #4 endpoint summary | `connections` (aggregate) |

**A1 `GET /apps`** — ranked list. Query: `?sort=usage_month|usage_today|last_seen_at&order=`, `?limit=` (max 200), `?include_deleted=true` (history joins after soft delete). Success 200:
```json
[ { "id": 1, "name": "Firefox", "icon": "ic:1", "attribution_confidence": "exact",
    "first_seen_at": 1731567000, "last_seen_at": 1731570600,
    "usage": { "today": {"up":240000,"down":1980000}, "month": {"up":52428800,"down":934000000} } } ]
```
Errors: 400, 401, 429, 500.

**A2 `GET /apps/{id}`** — one app. Success 200: A1 row + `exe_path` (never exposes icon bytes; icons resolve via `icon` handle → `profile/icon`-style resource served by the same server). Errors: 404 (pruned/not found), 401, 429.

**A3 `GET /apps/{id}/usage`** — series for the App-detail chart. Query: `?granularity=hour|day&since=&until=&limit=` (day default 90; hour default 168). Rows come from `usage_rollup_15m`; the raw window (< 7 days) is aggregated live from `connections`. Boundaries computed at query time in the configured timezone (`DATABASE.md §2.6`). Success 200:
```json
{ "granularity": "day", "data": [
    { "bucket_start": 1731504000, "up": 1048576, "down": 31457280, "connections": 412 } ] }
```
Errors: 400, 401, 404, 429, 500.

**A4 `GET /apps/{id}/endpoints`** — ranked remote endpoints. Query: `?limit=` (max 100), `?order=bytes`. Success 200:
```json
[ { "remote_ip": "151.101.1.1", "remote_host": null, "port": 443,
    "bytes_total": 2220000, "connections": 17, "last_seen_at": 1731570600 } ]
```
Errors: 400, 401, 404, 429.

### 2.5 Usage (dashboard aggregates)

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| U1 | `GET /api/v1/usage/summary` | yes | §6.1 #3 dashboard headline + top apps | `usage_interface_15m`, `usage_rollup_15m`, `apps` |
| U2 | `GET /api/v1/usage/timeline` | yes | §6.1 #3 usage timeline (Day/Hour toggle) | `usage_rollup_15m`, `connections` |
| U3 | `GET /api/v1/usage/interfaces` | yes | §6.1 #3 per-interface breakdown | `usage_interface_15m`, `interfaces` |

**U1 `GET /usage/summary`** — totals + top-talker list. Query: `?top_limit=` (default 5, max 10). Success 200:
```json
{ "today": {"total": 986432000, "attributed": 912345600, "unattributed": 74086400},
  "month": {"total": 9864320000, "attributed": 9123456000, "unattributed": 740864000},
  "top_apps": [ {"id":1,"name":"Firefox","bytes_total":986432000} ] }
```
**`total`** comes from `usage_interface_15m` (interface kernel counters — ISP-billed bytes). **`attributed`** is the sum of per-app rollups; **`unattributed`** = total − attributed (headers, ACKs, retransmits, kernel-handled/VPN traffic — shown as the "Unattributed / system" row in DESIGN.md §8.3).
Errors: 401, 429, 500, 503 (no data yet → 200 with zeroed totals + `"state":"collecting"`, DESIGN.md §8.3 empty state).

**U2 `GET /usage/timeline`** — dashboard chart series. Query: `?granularity=hour|day&since=&until=&interface_id=`. Shape identical to A3 minus `app_id` (system-wide). Errors: 400, 401, 429, 500.

**U3 `GET /usage/interfaces`** — per-interface totals for the dashboard card; excludes `is_excluded=1` interfaces. Query: `?since=&until=`. Success 200:
```json
[ { "id": 2, "name": "wlan0", "kind": "wifi", "is_metered": true,
    "bytes": {"up": 41943040, "down": 734003200} } ]
```
Errors: 401, 429, 500.

### 2.6 Interfaces

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| I1 | `GET /api/v1/interfaces` | yes | §6.1 #8 metered-interface labeling (OQ5) | `interfaces` |
| I2 | `PATCH /api/v1/interfaces/{id}` | yes | §6.1 #8 labeling | `interfaces` |

**I1** — list; no pagination. Success 200: array of U3-shaped rows + `is_excluded`. Errors: 401, 429.

**I2** — update flags. Request body (any subset):
```json
{ "is_metered": true, "is_excluded": false }
```
Success 200: full updated row. Errors: 400 (`QUERY_INVALID` — no other fields allowed), 404, 401, 429. Emitting a WS `status` update if exclusion changes totals semantics.
### 2.7 Settings

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| T1 | `GET /api/v1/settings` | yes | §6.1 #8 lean settings | `settings` |
| T2 | `PUT /api/v1/settings/{key}` | yes | §6.1 #8 (retention, theme, start_on_boot…) | `settings` |

**T1** — all settings with types and bounds. Success 200:
```json
[ { "key": "retention_days", "value": 90, "type": "int", "min": 7, "max": 90,
    "pro_max": 365, "help": "History kept before automatic pruning" },
  { "key": "start_on_boot", "value": true, "type": "bool" },
  { "key": "theme_default", "value": "system", "type": "enum", "values": ["system","light","dark"] } ]
```
Note `max` vs `pro_max` — the server reports the tier ceiling; the pruner enforces it (DATABASE.md §6). Errors: 401, 429.

**T2** — set one key. Request body: `{"value": 120}` (JSON-typed per `type`). Success 200: updated entry (T1 shape). Errors: 400 (wrong type / out of range / unknown key / **retention > 90 on free tier** → `403 EXPORT_PRO_REQUIRED`), 401, 429. Rejected writes never partially apply (single-row transaction).

### 2.8 License

| # | Endpoint | Auth | PRD feature | DB table |
|---|---|---|---|---|
| L1 | `GET /api/v1/license` | yes | §6.1 #9 freemium state | `license` |
| L2 | `POST /api/v1/license/activate` | yes | §6.1 #9 / Flow 4 step 4 | `license` |
| L3 | `DELETE /api/v1/license` | yes | §6.1 #9 (deactivate) | `license` |

**L1** — current tier + entitlements (the UI never parses license state itself). Success 200:
```json
{ "tier": "free",
  "entitlements": { "advanced_filters": false, "export": false, "retention_max_days": 90 } }
```
Errors: 401, 429.

**L2** — activate offline-signed key. Request body: `{"key": "NWP1-XXXX-XXXX-XXXX-XXXX"}`. Server verifies signature with the embedded public key (`packaging/licenses/`, no network — PRD §6.1 #9). Success 200: L1 shape with `tier:"pro"`; WS `status` update broadcast so Pro features unlock instantly without restart (PRD Flow 4). Errors: 400 (malformed), 403 `LICENSE_INVALID` (bad signature/expired), 409 `CONFLICT` (already pro — message offers no-op), 401, 429.

**L3** — reset to free. Request body: none. Success 200: L1 shape, `tier:"free"`. Errors: 401, 429. UI wraps this in a destructive confirm (DESIGN.md §7.4); the API is immediate.

### 3. WebSocket Channels (streaming half of the API)

Same base socket, multiplexed frame stream (no HTTP upgrade — the Unix socket carries both request/response and streaming frames). One connection multiplexes all channels; frames are versioned JSON. The Tauri native link layer opens the socket and relays frames to the main webview through its native bridge; this transport adaptation does not alter the API contract.

Client → server:
```json
{ "op": "subscribe", "channel": "live.connections" }
{ "op": "unsubscribe", "channel": "usage.totals" }
{ "op": "ping" }
```
Server → client:
```json
{ "channel": "live.connections", "kind": "snapshot", "data": [ /* C1 rows */ ] }
{ "channel": "live.connections", "kind": "delta",
  "added": [ /* row */ ], "updated": [ {"id":88231,"rate_down_bps":262144,"bytes_down":1980000} ],
  "removed": [ 90112 ] }
{ "channel": "status", "kind": "update",
  "data": { "daemon": "monitoring", "ebpf": "attached", "recording": true } }
{ "channel": "usage.totals", "kind": "update",
  "data": { "today": {"up": 52428800, "down": 934000000} } }
{ "op": "error", "error": { "code": "QUERY_INVALID", "message": "Unknown channel 'x'" } }
```

| Channel | Kind(s) | Rate | PRD feature | DB table |
|---|---|---|---|---|
| `live.connections` | `snapshot` on subscribe, then `delta` | ≤ 1 msg/s | §6.1 #1 Live view | `connections` + memory *(flagged §10)* |
| `status` | `update` on change | on change | §6.1 #6 chip/banners | derived + `daemon_events` |
| `usage.totals` | `update` | ≤ 1 msg/s | §6.1 #2/3 dashboard headline | `connections`/rollups |

Rules: deltas replace by row `id` (keyed, not positional); server pushes at most one message per channel per second (UI renders from its store, DESIGN.md §7.6 in-place updates); on reconnect the client re-subscribes and receives a fresh snapshot — no gap replay (honest-state UI shows "Connecting…" until it lands). Socket authorization failure (UID not in allowlist) → connection refused.
---

## 4. Quick Reference (methods)

| Resource | GET | POST | PUT/PATCH | DELETE |
|---|---|---|---|---|
| System | `/health`, `/status`, `/diagnostics`, `/events` | — | — | — |
| Connections | `/connections`, `/connections/{id}`, `/connections/live` | — | — | — |
| Apps | `/apps`, `/apps/{id}`, `/apps/{id}/usage`, `/apps/{id}/endpoints` | — | — | — |
| Usage | `/usage/summary`, `/usage/timeline`, `/usage/interfaces` | — | — | — |
| Exports | `/exports/{id}`, `/exports/{id}/content` | `/exports` | — | — |
| Interfaces | `/interfaces` | — | `PATCH /interfaces/{id}` | — |
| Settings | `/settings` | — | `PUT /settings/{key}` | — |
| License | `/license` | `/license/activate` | — | `DELETE /license` |
| Stream | `(ws) /stream` | — | — | — |

**23 REST endpoints + 1 WS upgrade + 3 channels.** No create/update/delete on connections or apps — those are *observed* data written only by the daemon (monitor-only MVP; the firewall phase-in, PRD §12, adds command endpoints without touching these).

---

## 5. Endpoint ↔ PRD ↔ DATABASE Traceability

**Flags (things that do not map 1:1 — explicitly justified):**

| Flag | Item | Justification |
|---|---|---|
| F1 | `GET /health`, `GET /status`, WS `status` | Derived runtime state (daemon/eBPF/recording), persisted only as evidence in `daemon_events`. Correct per DESIGN.md §1 Principle 3 — status is honest and transient, not a table grain. |
| F2 | `GET /connections/live`, WS `live.connections` | Serves current-second data from in-memory ring-buffer aggregates; the durable grain is `connections` (`is_open=1` overlaps). Avoids a 1 Hz write amplification into SQLite (ARCHITECTURE.md §9). |
| F3 | Export job state (E2) | In-memory job table (expires 10 min); results are `connections`/rollup projections. Not worth a DB table. |
| F4 | `schema_migrations` (DATABASE.md §2.1) | Intentionally **never exposed** via the API — internal ledger; visible only through Diagnostics' version line. |
| F5 | `usage/timeline` crossing the raw window | Merges live-aggregated `connections` (< 7 days) with `usage_rollup_*` (> 7 days) into one series — a read-time union, not a schema concept. |

**Everything else maps directly:** every endpoint's PRD feature and DB table(s) are stated in its group header (§2.1–§2.8, §3).

Coverage check (every DATABASE table is served):

| Table | Serving endpoints |
|---|---|
| `apps` | A1–A4 (C2, U1 join) |
| `interfaces` | I1–I2, U3 |
| `connections` | C1–C3, E1/E3, A3/A4/U2 (raw window) |
| `usage_rollup_15m` | U1 (attributed), U2, A3, E1/E3 |
| `usage_endpoint_day` | C2 (aggregate rows beyond raw window), A4 |
| `usage_interface_15m` | U1 (total), U3 |
| `settings` | T1–T2, S3 |
| `license` | L1–L3 (entitlement checks in C2, E1–E3, T2) |
| `daemon_events` | S2–S4 |
| `schema_migrations` | *(none — flag F4)* |

---

## 6. Governance

- Binding like ARCHITECTURE.md §10: changes bump this file's version; `docs/api-contract.md` mirrors §1–§3 and drift fails CI.
- New endpoints must enter the §5 traceability matrix (with a PRD feature reference) or carry a justified flag before implementation.
- Error codes, status mapping (§1.3), pagination envelope (§1.4), and versioning (§1.6) are frozen for v1; extending them is a v2 decision.

---

*End of API.md. No application code — contracts and examples only.*
