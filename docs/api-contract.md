# api-contract.md — API Contract Mirror

**Binding source:** `../API.md` (§1–§3). This file mirrors it for implementation reference; **drift fails CI** (P0.8 check). If these differ, fix the doc or the code — never ignore (AGENTS.md §2).

---

## Base & transport

- `ws(s)://127.0.0.1:7897/api/v1` — loopback only; HTTP GET for queries, WS for streams (`/api/v1/stream`).
- Auth: `Authorization: Bearer <per-boot token>` from canonical production `/var/lib/netwatch-pro/api-token` (mode 0600); UI obtains it via the one-shot Polkit-authorized native helper. `NETWATCH_API_TOKEN_FILE` is daemon-only for dev/test and never changes the helper path; anonymous: `GET /health` only.
- WebSocket clients must send the Authorization header on upgrade. Browser clients use the Tauri native link bridge because the WebSocket API cannot set custom headers; the wire contract remains unchanged.
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
| Apps | `GET /apps` · `GET /apps/{id}` · `GET /apps/{id}/usage` · `GET /apps/{id}/endpoints` |
| Usage | `GET /usage/summary` · `GET /usage/timeline` · `GET /usage/interfaces` |
| Interfaces | `GET /interfaces` · `PATCH /interfaces/{id}` |
| Settings | `GET /settings` · `PUT /settings/{key}` |
| License | `GET /license` · `POST /license/activate` · `DELETE /license` |
| Stream | `(ws) /stream` → channels `live.connections`, `status`, `usage.totals` |

Rate limits: 120/min reads · 5/min export create · 10/min writes & downloads → 429 + `Retry-After`.
