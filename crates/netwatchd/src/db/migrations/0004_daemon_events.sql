-- Migration 0004: daemon_events (DATABASE.md §8).
-- Bounded immutable diagnostics log for degraded-state evidence. No
-- updated_at: event rows are never modified after insertion.

CREATE TABLE daemon_events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    occurred_at INTEGER NOT NULL,
    level TEXT NOT NULL DEFAULT 'info' CHECK (level IN ('info','warn','error')),
    code TEXT NOT NULL,
    message TEXT NOT NULL CHECK (length(message) <= 2000),
    detail_json TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- idx_daemon_events_occurred: Diagnostics modal dump, newest first (§5).
CREATE INDEX idx_daemon_events_occurred ON daemon_events(occurred_at DESC);

INSERT INTO schema_migrations (version, name, applied_at, created_at, updated_at)
VALUES (4, 'daemon_events', strftime('%s','now'), strftime('%s','now'), strftime('%s','now'));
