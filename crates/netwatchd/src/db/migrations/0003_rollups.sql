-- Migration 0003: rollups (DATABASE.md §8 — parents first, indexes with
-- their tables). Tables: usage_rollup_15m (§2.5), usage_endpoint_day (§2.5a),
-- usage_interface_15m (§2.5b) — F-02/D2a, F-03/D3a, F-04/D4a decisions.
-- Also: idx_connections_remote_host, deferred from 0002 (§5: created when
-- the column becomes queryable).
--
-- Grain notes: no surrogate ids — the composite grain IS the key (atomic
-- upserts). Timestamps stay epoch seconds UTC; day/15m bucket alignment is
-- computed at query time in the configured timezone (D4a), never stored as
-- separate per-day tables.

-- usage_rollup_15m (DATABASE.md §2.5): per-app 15-minute buckets — the base
-- grain. Dashboard timeline (Day/Hour toggle) and usage by period ride the
-- PK range scan; no extra index (§5).
CREATE TABLE usage_rollup_15m (
    app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE RESTRICT, -- apps soft-deleted, never hard
    bucket_start INTEGER NOT NULL,    -- 15-minute-aligned epoch (UTC)
    bytes_up INTEGER NOT NULL DEFAULT 0 CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL DEFAULT 0 CHECK (bytes_down >= 0),
    connection_count INTEGER NOT NULL DEFAULT 0 CHECK (connection_count >= 0),
    interfaces_mask TEXT,             -- comma-separated contributing interface names
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (app_id, bucket_start)
);

-- usage_endpoint_day (DATABASE.md §2.5a): per-app/day endpoint aggregates —
-- searchable history beyond the raw window (D2a). Cardinality cap (top N
-- endpoints/day, rest folded into one remote_ip='other' row) is enforced by
-- the compaction job, not the schema.
CREATE TABLE usage_endpoint_day (
    app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE RESTRICT,
    bucket_start INTEGER NOT NULL,    -- day-aligned epoch (UTC)
    remote_ip TEXT NOT NULL,          -- 'other' = fold row beyond the top-N cap
    remote_port INTEGER NOT NULL CHECK (remote_port BETWEEN 0 AND 65535),
    protocol TEXT NOT NULL CHECK (protocol IN ('tcp','udp','icmp','other')),
    bytes_up INTEGER NOT NULL DEFAULT 0 CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL DEFAULT 0 CHECK (bytes_down >= 0),
    connection_count INTEGER NOT NULL DEFAULT 0 CHECK (connection_count >= 0),
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL CHECK (last_seen_at >= first_seen_at),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (app_id, bucket_start, remote_ip, remote_port, protocol)
);

-- idx_usage_endpoint_day_app_day: endpoint summary per app/day (App detail,
-- History Pro filters) (DATABASE.md §5).
CREATE INDEX idx_usage_endpoint_day_app_day
    ON usage_endpoint_day(app_id, bucket_start DESC);

-- idx_usage_endpoint_day_remote: cross-app endpoint search (Pro advanced
-- filters); also serves the port filter (always co-occurs with host/ip —
-- anti-index note, DATABASE.md §5).
CREATE INDEX idx_usage_endpoint_day_remote
    ON usage_endpoint_day(remote_ip, remote_port, protocol);

-- usage_interface_15m (DATABASE.md §2.5b): per-interface 15-minute counters —
-- the source of truth for headline totals (ISP-billed bytes, D3a); feeds the
-- Unattributed gap row. `lo` (is_excluded=1) is never inserted; the sampler
-- task (P4.2) enforces that, the schema keeps the FK open to all interfaces.
CREATE TABLE usage_interface_15m (
    interface_id INTEGER NOT NULL REFERENCES interfaces(id) ON DELETE RESTRICT,
    bucket_start INTEGER NOT NULL,    -- 15-minute-aligned epoch (UTC)
    rx_bytes INTEGER NOT NULL DEFAULT 0 CHECK (rx_bytes >= 0),
    tx_bytes INTEGER NOT NULL DEFAULT 0 CHECK (tx_bytes >= 0),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (interface_id, bucket_start)
);

-- idx_usage_interface_15m_iface_bucket: headline totals per interface over
-- time (DATABASE.md §5).
CREATE INDEX idx_usage_interface_15m_iface_bucket
    ON usage_interface_15m(interface_id, bucket_start DESC);

-- idx_connections_remote_host: History hostname search (V2 DNS display) —
-- deferred from 0002 now that this migration lands (DATABASE.md §5).
CREATE INDEX idx_connections_remote_host ON connections(remote_host);

-- §1 updated_at maintenance: one standard trigger pair per table, created in
-- the table's migration. (0001/0002 tables predate this rule in migrations;
-- their updated_at is maintained by the writer's queries — see the doc-drift
-- note in the task summary.)
CREATE TRIGGER usage_rollup_15m_set_created_at
AFTER INSERT ON usage_rollup_15m
BEGIN
    UPDATE usage_rollup_15m SET created_at = strftime('%s','now')
    WHERE app_id = NEW.app_id AND bucket_start = NEW.bucket_start;
END;

CREATE TRIGGER usage_rollup_15m_set_updated_at
AFTER UPDATE ON usage_rollup_15m
BEGIN
    UPDATE usage_rollup_15m SET updated_at = strftime('%s','now')
    WHERE app_id = NEW.app_id AND bucket_start = NEW.bucket_start
      AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER usage_endpoint_day_set_created_at
AFTER INSERT ON usage_endpoint_day
BEGIN
    UPDATE usage_endpoint_day SET created_at = strftime('%s','now')
    WHERE app_id = NEW.app_id AND bucket_start = NEW.bucket_start
      AND remote_ip = NEW.remote_ip AND remote_port = NEW.remote_port
      AND protocol = NEW.protocol;
END;

CREATE TRIGGER usage_endpoint_day_set_updated_at
AFTER UPDATE ON usage_endpoint_day
BEGIN
    UPDATE usage_endpoint_day SET updated_at = strftime('%s','now')
    WHERE app_id = NEW.app_id AND bucket_start = NEW.bucket_start
      AND remote_ip = NEW.remote_ip AND remote_port = NEW.remote_port
      AND protocol = NEW.protocol
      AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER usage_interface_15m_set_created_at
AFTER INSERT ON usage_interface_15m
BEGIN
    UPDATE usage_interface_15m SET created_at = strftime('%s','now')
    WHERE interface_id = NEW.interface_id AND bucket_start = NEW.bucket_start;
END;

CREATE TRIGGER usage_interface_15m_set_updated_at
AFTER UPDATE ON usage_interface_15m
BEGIN
    UPDATE usage_interface_15m SET updated_at = strftime('%s','now')
    WHERE interface_id = NEW.interface_id AND bucket_start = NEW.bucket_start
      AND updated_at = OLD.updated_at;
END;

INSERT INTO schema_migrations (version, name, applied_at, created_at, updated_at)
VALUES (3, 'rollups', strftime('%s','now'), strftime('%s','now'), strftime('%s','now'));
