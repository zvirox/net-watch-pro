# db-schema.md — Schema Mirror

**Binding source:** `../DATABASE.md` (§1–§5). Mirror for implementation reference; drift fails CI (P0.8).

---

## Engine & pragmas (set on every connection open)

SQLite ≥ 3.40 · `journal_mode=WAL` · `synchronous=NORMAL` · `foreign_keys=ON` · file at `$NETWATCH_DATA_DIR/netwatch.db`.

## Conventions

snake_case · plural tables · `INTEGER` UTC epoch seconds (`*_at`) · `created_at`+`updated_at` on every table (trigger pair `{table}_set_{created,updated}_at`) · surrogate `INTEGER PRIMARY KEY` unless composite stated · booleans INTEGER 0/1 + CHECK · IPs as canonical TEXT.

## Tables (DDL in `crates/netwatchd/src/db/migrations/*.sql` — forward-only)

| Table | Grain / key | Notes |
|---|---|---|
| `schema_migrations` | PK `version` | audit ledger; never exposed via API |
| `apps` | PK `id`; UK `exe_path` | soft delete `deleted_at`; confidence `exact/heuristic/unknown` |
| `interfaces` | PK `id`; UK `name` | `kind` enum; `is_metered`, `is_excluded` |
| `connections` | PK `id` | FK `app_id` RESTRICT, `interface_id` SET NULL; CHECKs on ports/bytes; `is_open`; **no soft delete** (pruning is the feature) |
| `usage_rollup_hour` | composite PK `(app_id, bucket_start)` | FK RESTRICT; `interfaces_mask` denormalized (documented) |
| `usage_rollup_day` | composite PK `(app_id, bucket_start)` | same shape |
| `settings` | PK `key` | value typed at application layer |
| `license` | PK `id` with `CHECK (id = 1)` | `tier free/pro`; fingerprint only — raw key never stored |
| `daemon_events` | PK `id`; immutexcept | ring: newest 5,000; no `updated_at` (immutable) |

## Indexes (each names its query — DATABASE.md §5)

`idx_connections_app_first_seen(app_id, first_seen_at DESC)` · `idx_connections_last_seen(last_seen_at)` · `idx_connections_open(is_open, last_seen_at DESC) WHERE is_open=1` · `idx_connections_remote_ip(remote_ip)` · `idx_connections_remote_host(remote_host)` (0003) · `idx_daemon_events_occurred(occurred_at DESC)` · `idx_apps_last_seen(last_seen_at DESC)`. Anti-indexes: none on remote_port/local_*/bytes_*/packets_*.

## Migration order

0001 init core (+seeds: settings defaults, license singleton, `lo` interface) → 0002 apps+connections (+indexes) → 0003 rollups (+remote_host index) → 0004 daemon_events → forward-only after. Unknown/newer schema → daemon refuses to start.
