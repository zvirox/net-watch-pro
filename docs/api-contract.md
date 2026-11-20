# api-contract.md — API Contract Mirror

**Binding source:** `../API.md` (§1–§3). This file mirrors it for implementation reference; **drift fails CI** (P0.8 check). If these differ, fix the doc or the code — never ignore (AGENTS.md §2).

---

## Base & transport

- `unix:/run/netwatch-pro/api.sock` by default (`NETWATCH_API_SOCKET` env var) — Unix domain socket only; HTTP GET for queries, multiplexed frame stream for live channels.
- Auth: peer-credential (`SO_PEERCRED`) UID check vs root-owned allowed-UID list; one admin prompt at onboarding pkexec, none per launch. No bearer token, no Polkit helper, no `NETWATCH_API_TOKEN_FILE`; anonymous: `GET /health` only.
- All payloads versioned JSON; unknown fields ignored on read, rejected on write.
- Version: path-based `/api/v1`; additive-only; breaking → `/api/v2` + 90-day deprecation.

## Error envelope (everywhere, no exceptions)

```json
{ "error": { "code": "<CODE>", "message": "…", "details": {}, "request_id": "…" } }
```

Codes: `DAEMON_UNREACHABLE` `AUTH_INVALID` `EBPF_ATTACH_FAILED` `EBPF_UNSUPPORTED_KERNEL` `PERMISSION_DENIED` `DB_ERROR` `QUERY_INVALID` `EXPORT_PRO_REQUIRED` `LICENSE_INVALID` `RATE_LIMITED` `NOT_FOUND` `CONFLICT` `METHOD_NOT_ALLOWED`.

Status mapping: 400 QUERY_INVALID · 401 AUTH_INVALID · 403 PERMISSION_DENIED/EXPORT_PRO_REQUIRED/LICENSE_INVALID · 404 NOT_FOUND · 405 · 409 CONFLICT · 429 RATE_LIMITED · 500 DB_ERROR · 503 DAEMON_UNREACHABLE/EBPF_*.

## Pagination envelope

`?limit=` (default 50, max 500) + opaque `?cursor=` → `{ "data": [...], "next_cursor": "…", "has_more": true }` for `connections`, `events`; plain arrays for bounded resources.

## Endpoints (v1.0 set — see API.md §2 for full request/response examples)

| Group | Endpoints |
|---|---|
| System | `GET /health` (no auth) · `GET /status` · `GET /diagnostics` · `GET /events` |
| Connections | `GET /connections` · `GET /connections/{id}` · `GET /connections/live` |
| Exports (Pro) | `POST /exports` · `GET /exports/{id}` · `GET /exports/{id}/content` |
| Apps | `GET /apps` · `GET /apps/{id}` · `GET /apps/{id}/icon` · `GET /apps/{id}/usage` · `GET /apps/{id}/endpoints` |
| Usage | `GET /usage/summary` · `GET /usage/timeline` · `GET /usage/interfaces` |
| Interfaces | `GET /interfaces` · `PATCH /interfaces/{id}` |
| Settings | `GET /settings` · `PUT /settings/{key}` |
| License | `GET /license` · `POST /license/activate` · `DELETE /license` |
| Stream | socket frame stream → channels `live.connections`, `status`, `usage.totals`, `net.rates`, `live.apps` |

Rate limits: 600/min reads · 5/min export create · 10/min writes & downloads → 429 + `Retry-After`.

## Live throughput (API.md §3, Phase A)

- `net.rates` frame: `{ "channel":"net.rates","kind":"update","data":{ "ts":<epoch>,"down_bps":<int>,"up_bps":<int>,"interfaces":[{ "name","down_bps","up_bps" }] } }`; snapshot on subscribe, then 1 Hz.
- `live.apps` frame: `{ "channel":"live.apps","kind":"update","data":{ "ts":<epoch>,"apps":[{ "cgroup_id":<int>,"name":<str>,"down_bps":<int>,"up_bps":<int>,"confidence":"high"|"low" }],"unattributed":{"down_bps":<int>,"up_bps":<int>} } }`; keyed by socket cgroup id, 1 Hz, empty rows when no collector.
- Rates are decimal bits per second; total is the sum over non-excluded interfaces with the tunnel dedup rule. Frames older than 3 s are stale on the client.
- `GET /status` `ebpf` may be `no_collector` (no collector attached); the UI must not show a green chip for that state.
- Settings key `speed_unit`: enum `mbps` (default) | `mbytes`; forward-only migration 0005.
- `GET /usage/rate-history` → `{ "samples": [{ts, down_bps, up_bps}], "in_memory": true }`; 60 one-minute samples covering the last hour, lost on restart (owner L-OQ1).
- U3 rows may carry optional `signal_dbm` / `link_mbps` from local sysfs (`wireless/link`, `speed`); `null` means unavailable — render "—" (owner L-OQ4).
