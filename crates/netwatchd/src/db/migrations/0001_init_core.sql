-- Migration 0001: init_core (DATABASE.md §8 — parents first, seeds last)
-- Tables: schema_migrations, settings (+seed), license (+seed), interfaces (+lo seed).

-- Human/audit migration ledger (DATABASE.md §2.1). sqlx keeps its own
-- `_sqlx_migrations`; this table is the human-readable complement, written
-- by each migration inside its own transaction.
CREATE TABLE schema_migrations (
    version INTEGER NOT NULL PRIMARY KEY, -- monotonic, matches migration file number
    name TEXT NOT NULL,
    applied_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

INSERT INTO schema_migrations (version, name, applied_at, created_at, updated_at)
VALUES (1, 'init_core', strftime('%s','now'), strftime('%s','now'), strftime('%s','now'));

-- settings (DATABASE.md §2.6): daemon-side mirror; typed at the application
-- layer. Seeded with the closed key list defaults (§2.6 / ARCHITECTURE.md §8.3).
CREATE TABLE settings (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

INSERT INTO settings (key, value, created_at, updated_at) VALUES
    ('retention_days',    '90',     strftime('%s','now'), strftime('%s','now')), -- free-tier max 90 (PRD §6.1 #5)
    ('start_on_boot',     '1',      strftime('%s','now'), strftime('%s','now')),
    ('theme_default',     'system', strftime('%s','now'), strftime('%s','now')),
    ('metered_policy',    'ask',    strftime('%s','now'), strftime('%s','now')),
    ('raw_window_days',   '7',      strftime('%s','now'), strftime('%s','now')),
    ('timezone',          'local',  strftime('%s','now'), strftime('%s','now')), -- sentinel: daemon resolves system-local tz at query time (D4a)
    ('billing_cycle_day', '1',      strftime('%s','now'), strftime('%s','now'));

-- license (DATABASE.md §2.7): singleton, free tier until offline activation.
CREATE TABLE license (
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    tier TEXT NOT NULL DEFAULT 'free' CHECK (tier IN ('free','pro')),
    key_fingerprint TEXT,             -- NOT NULL when tier='pro'
    activated_at INTEGER,             -- NOT NULL when tier='pro'
    expires_at INTEGER,               -- NULL = perpetual (offline validation, no server check)
    signature_verified_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

INSERT INTO license (id, tier, created_at, updated_at)
VALUES (1, 'free', strftime('%s','now'), strftime('%s','now'));

-- interfaces (DATABASE.md §2.3): known interfaces; drives per-interface
-- breakdown and metered labeling (PRD §6.1 #8, OQ5).
CREATE TABLE interfaces (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL DEFAULT 'ethernet' CHECK (kind IN ('ethernet','wifi','tun','loopback','other')),
    is_metered INTEGER NOT NULL DEFAULT 0 CHECK (is_metered IN (0,1)),
    is_excluded INTEGER NOT NULL DEFAULT 0 CHECK (is_excluded IN (0,1)), -- e.g. loopback excluded from totals
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- Seed loopback; excluded from totals by default (DATABASE.md §2.5b).
INSERT INTO interfaces (name, kind, is_metered, is_excluded, created_at, updated_at)
VALUES ('lo', 'loopback', 0, 1, strftime('%s','now'), strftime('%s','now'));
