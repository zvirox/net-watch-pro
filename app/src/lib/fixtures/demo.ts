// OQ4 decision (P8.6): small deterministic sample-data set for the read-only
// demo mode. No daemon, no files, no network — all data lives in memory.

import type { HistoryRow } from '$lib/stores/history.svelte';
import type { InterfaceUsage, Summary, TimelinePoint } from '$lib/stores/usage.svelte';

const DAY = 86_400;
const HOUR = 3_600;

export const DEMO_SUMMARY: Summary = {
  today: { total: 1_240_000_000, attributed: 1_080_000_000, unattributed: 160_000_000 },
  month: { total: 18_600_000_000, attributed: 16_900_000_000, unattributed: 1_700_000_000 },
  top_apps: [
    { id: 1, name: 'Firefox', bytes_total: 7_400_000_000 },
    { id: 2, name: 'Chrome', bytes_total: 4_100_000_000 },
    { id: 3, name: 'Signal', bytes_total: 1_300_000_000 },
  ],
  state: 'ok',
};

export const DEMO_TIMELINE: TimelinePoint[] = Array.from({ length: 14 }, (_, index) => ({
  bucket_start: index * DAY,
  up: 30_000_000 + index * 2_400_000,
  down: 210_000_000 + index * 9_600_000,
  connections: 120 + index * 14,
}));

export const DEMO_INTERFACES: InterfaceUsage[] = [
  {
    id: 2,
    name: 'wlan0',
    kind: 'wifi',
    is_metered: true,
    bytes: { up: 240_000_000, down: 1_000_000_000 },
  },
  {
    id: 3,
    name: 'eth0',
    kind: 'ethernet',
    is_metered: false,
    bytes: { up: 2_000_000, down: 8_000_000 },
  },
];

export const DEMO_ROWS: HistoryRow[] = [
  {
    id: 101,
    app: { id: 1, name: 'Firefox', icon: null },
    pid: 2213,
    process_name: 'firefox',
    protocol: 'tcp',
    local_ip: '192.168.1.23',
    local_port: 53210,
    remote_ip: '151.101.1.1',
    remote_port: 443,
    remote_host: 'example.com',
    bytes_up: 1_800_000,
    bytes_down: 24_000_000,
    packets_up: 1400,
    packets_down: 3200,
    connection_count: null,
    first_seen_at: 10 * DAY,
    last_seen_at: 10 * DAY + HOUR,
  },
  {
    id: 102,
    app: { id: 2, name: 'Chrome', icon: null },
    pid: 3401,
    process_name: 'chrome',
    protocol: 'tcp',
    local_ip: '192.168.1.23',
    local_port: 53212,
    remote_ip: '142.250.70.14',
    remote_port: 443,
    remote_host: null,
    bytes_up: 640_000,
    bytes_down: 9_400_000,
    packets_up: 600,
    packets_down: 1100,
    connection_count: null,
    first_seen_at: 9 * DAY,
    last_seen_at: 9 * DAY + 2 * HOUR,
  },
];
