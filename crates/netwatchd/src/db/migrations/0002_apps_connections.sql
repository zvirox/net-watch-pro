-- Migration 0002: apps_connections (DATABASE.md §8 — FK parents before
-- children, indexes with their tables, no seed rows).
-- Tables: apps (+idx_apps_last_seen), connections (+4 §5 indexes).
-- idx_connections_remote_host intentionally deferred to migration 0003
-- (DATABASE.md §5: created when the column becomes queryable).

-- apps (DATABASE.md §2.2): one row per distinct executable identity.
-- app_key is the unique identity key (cgroup path / flatpak / snap / exe);
-- exe_path is non-unique display/provenance data (F-05/D5a).
CREATE TABLE apps (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    app_key TEXT NOT NULL UNIQUE,
    identity_kind TEXT NOT NULL DEFAULT 'exe' CHECK (identity_kind IN ('cgroup_app','flatpak','snap','exe')),
    exe_path TEXT NOT NULL,
    exe_hash TEXT,                    -- SHA-256 when resolvable; NULL allowed (updates change hash)
    icon_path TEXT,                   -- NULL → UI fallback container (DESIGN.md §6.2)
    attribution_confidence TEXT NOT NULL DEFAULT 'exact' CHECK (attribution_confidence IN ('exact','heuristic','unknown')),
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    deleted_at INTEGER,               -- soft delete; rows kept for history joins
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- idx_apps_last_seen: top-apps ranking; pruner's last-activity check for
-- soft delete (DATABASE.md §5).
CREATE INDEX idx_apps_last_seen ON apps(last_seen_at DESC);

-- connections (DATABASE.md §2.4): one row per connection episode (grain per
-- ARCHITECTURE.md §5.4). High-volume table; retention pruner target.
CREATE TABLE connections (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE RESTRICT, -- apps soft-deleted, never hard
    interface_id INTEGER REFERENCES interfaces(id) ON DELETE SET NULL, -- interface may vanish
    pid INTEGER NOT NULL,             -- snapshot at attribution; not a live reference
    process_name TEXT,
    protocol TEXT NOT NULL CHECK (protocol IN ('tcp','udp','icmp','other')),
    local_ip TEXT NOT NULL,
    local_port INTEGER NOT NULL CHECK (local_port BETWEEN 0 AND 65535),
    remote_ip TEXT NOT NULL,
    remote_port INTEGER NOT NULL CHECK (remote_port BETWEEN 0 AND 65535),
    remote_host TEXT,                 -- display name only; never overwrites remote_ip
    bytes_up INTEGER NOT NULL DEFAULT 0 CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL DEFAULT 0 CHECK (bytes_down >= 0),
    packets_up INTEGER NOT NULL DEFAULT 0 CHECK (packets_up >= 0),
    packets_down INTEGER NOT NULL DEFAULT 0 CHECK (packets_down >= 0),
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL CHECK (last_seen_at >= first_seen_at),
    is_open INTEGER NOT NULL DEFAULT 1 CHECK (is_open IN (0,1)),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- Indexes per DATABASE.md §5, each naming the query it serves:
-- idx_connections_app_first_seen: App detail timeline; History filtered by
-- app + date range (DESIGN.md §8.4/§8.5).
CREATE INDEX idx_connections_app_first_seen
    ON connections(app_id, first_seen_at DESC);

-- idx_connections_last_seen: Live view open-episode scan; retention pruning.
CREATE INDEX idx_connections_last_seen ON connections(last_seen_at);

-- idx_connections_remote_ip: endpoint summary "where did this app connect";
-- History filter by remote host/IP (Pro).
CREATE INDEX idx_connections_remote_ip ON connections(remote_ip);

-- idx_connections_open: live connections snapshot — the hottest query
-- (every second, DESIGN.md §8.4); partial index over open episodes only.
CREATE INDEX idx_connections_open
    ON connections(is_open, last_seen_at DESC) WHERE is_open = 1;

INSERT INTO schema_migrations (version, name, applied_at, created_at, updated_at)
VALUES (2, 'apps_connections', strftime('%s','now'), strftime('%s','now'), strftime('%s','now'));
