-- Migration 0006: remembered usage range preset (DATABASE.md §2.6, Phase D).
-- One value shared by Dashboard, Apps and App detail; API callers may still
-- pass an explicit `range` query parameter to override it per request.

INSERT INTO settings (key, value, created_at, updated_at)
VALUES ('range_default', '24h', strftime('%s','now'), strftime('%s','now'));
