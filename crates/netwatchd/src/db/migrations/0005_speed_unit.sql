-- Migration 0005: live-throughput display unit (DATABASE.md §2.6).
-- Rates are always stored and sent as integer bits/second; this setting only
-- controls how the Dashboard "Network now" card labels them.

INSERT INTO settings (key, value, created_at, updated_at)
VALUES ('speed_unit', 'mbps', strftime('%s','now'), strftime('%s','now'));
