// OQ4 decision (P8.6): small sample-data set for the read-only demo mode.
// No daemon, no files, no network — all data lives in memory. Anchored to load
// time so timestamps read as a recent, coherent 24 h window rather than 1970.

export interface HistoryRow {
  id?: number;
  pid?: number;
  process_name?: string;
  episode_id?: number;
  app_id?: number;
  app_name?: string;
  app?: { id: number; name: string; icon: string | null };
  binary_path?: string;
  cmdline?: string;
  remote_endpoint?: string;
  local_ip?: string;
  local_port?: number;
  remote_ip?: string;
  remote_host?: string | null;
  remote_port?: number;
  protocol?: string;
  proto?: string;
  bytes_up: number;
  bytes_down: number;
  packets_up?: number;
  packets_down?: number;
  connection_count?: number | null;
  first_seen_at?: number;
  last_seen_at?: number;
  started_at?: number;
  closed_at?: number;
  duration_sec?: number;
}

export interface InterfaceUsage {
  id?: number;
  name: string;
  kind?: string;
  is_metered?: boolean;
  bytes?: { up: number; down: number };
  signal_dbm?: number | null;
  link_mbps?: number | null;
  bytes_rx?: number;
  bytes_tx?: number;
}

export interface TimelinePoint {
  bucket_start: number;
  up: number;
  down: number;
  connections: number;
}

export interface Summary {
  state?: string;
  today: { total: number; attributed: number; unattributed: number };
  month: { total: number; attributed: number; unattributed: number };
  range: {
    preset: string;
    start: number;
    end: number;
    totals: { total: number; attributed: number; unattributed: number };
  };
  cycle: { start: number; end: number; days_left: number; used_bytes: number };
  budgets: {
    total_daily: {
      used_bytes: number;
      limit_bytes: number;
      level: 'ok' | 'warning' | 'critical' | 'breach';
    };
    total_monthly: {
      used_bytes: number;
      limit_bytes: number;
      level: 'ok' | 'warning' | 'critical' | 'breach';
    };
  };
  top_apps: { id: number; name: string; bytes_total: number }[];
}

const MINUTE = 60;
const QUARTER = 15 * MINUTE;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const NOW = Math.floor(Date.now() / 1000 / QUARTER) * QUARTER;

export const DEMO_SUMMARY: Summary = {
  today: { total: 1_240_000_000, attributed: 1_080_000_000, unattributed: 160_000_000 },
  month: { total: 18_600_000_000, attributed: 16_900_000_000, unattributed: 1_700_000_000 },
  range: {
    preset: '24h',
    start: NOW - DAY,
    end: NOW,
    totals: { total: 1_240_000_000, attributed: 1_080_000_000, unattributed: 160_000_000 },
  },
  cycle: { start: NOW - 22 * DAY, end: NOW + 8 * DAY, days_left: 8, used_bytes: 18_600_000_000 },
  budgets: {
    total_daily: { used_bytes: 0, limit_bytes: 0, level: 'ok' },
    total_monthly: { used_bytes: 18_600_000_000, limit_bytes: 0, level: 'ok' },
  },
  top_apps: [
    { id: 1, name: 'Firefox', bytes_total: 600_000_000 },
    { id: 2, name: 'Chrome', bytes_total: 300_000_000 },
    { id: 3, name: 'Signal', bytes_total: 150_000_000 },
  ],
  state: 'ok',
};

// 96 × 15 min = 24 h. The gentle day/night wave keeps the summary's ~1.2 GB
// total coherent and gives the axis advancing times instead of a repeated one.
export const DEMO_TIMELINE: TimelinePoint[] = Array.from({ length: 96 }, (_, index) => {
  const wave = Math.sin((index / 96) * Math.PI * 2 - Math.PI / 2);
  return {
    bucket_start: NOW - (95 - index) * QUARTER,
    up: Math.round(1_200_000 + 500_000 * wave),
    down: Math.round(9_000_000 + 4_000_000 * wave),
    connections: 120 + Math.round(40 * wave),
  };
});

export const DEMO_INTERFACES: InterfaceUsage[] = [
  {
    id: 2,
    name: 'wlan0',
    kind: 'wifi',
    is_metered: true,
    bytes: { up: 240_000_000, down: 1_000_000_000 },
    signal_dbm: -57,
    link_mbps: 866,
  },
  {
    id: 3,
    name: 'eth0',
    kind: 'ethernet',
    is_metered: false,
    bytes: { up: 2_000_000, down: 8_000_000 },
    signal_dbm: null,
    link_mbps: null,
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
    first_seen_at: NOW - 2 * HOUR,
    last_seen_at: NOW - HOUR,
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
    first_seen_at: NOW - 5 * HOUR,
    last_seen_at: NOW - 4 * HOUR,
  },
];
