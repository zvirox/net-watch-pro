// Dashboard usage store (ARCHITECTURE.md §6.1 tier 2, read-only): REST U1–U3
// plus the `usage.totals` WS channel for the live headline (API.md §3).

import { setTrayTooltip } from '$lib/api/transport';
import { DEMO_INTERFACES, DEMO_SUMMARY, DEMO_TIMELINE } from '$lib/fixtures/demo';
import { formatBytes } from '$lib/utils/format';
import { daemonStore } from './daemon.svelte';

export type Totals = {
  total: number;
  attributed: number;
  unattributed: number;
};

export type Summary = {
  today: Totals;
  month: Totals;
  top_apps: Array<{ id: number; name: string; bytes_total: number }>;
  state: 'ok' | 'collecting';
};

export type TimelinePoint = {
  bucket_start: number;
  up: number;
  down: number;
  connections: number;
};

export type InterfaceUsage = {
  id: number;
  name: string;
  kind: string;
  is_metered: boolean;
  bytes: { up: number; down: number };
  /** Wi-Fi signal in dBm; null when sysfs does not expose it (owner L-OQ4). */
  signal_dbm: number | null;
  /** Negotiated link speed in Mbps; null when unavailable. */
  link_mbps: number | null;
};

export type Granularity = 'hour' | 'day';

let summary = $state<Summary | null>(null);
let timeline = $state<TimelinePoint[]>([]);
let interfaces = $state<InterfaceUsage[]>([]);
let granularity = $state<Granularity>('day');
let streamToday: { up: number; down: number } | null = $state(null);
let loading = $state(false);
let error = $state<string | null>(null);
let requestGeneration = 0;
let demoActive = false;

function reset(): void {
  requestGeneration += 1;
  summary = null;
  timeline = [];
  interfaces = [];
  streamToday = null;
  loading = false;
  error = null;
  void setTrayTooltip('Net Watch Pro — local only').catch(() => undefined);
}

function applyStreamTotals(): void {
  if (streamToday === null || summary === null) return;
  const total = streamToday.up + streamToday.down;
  summary.today = {
    ...summary.today,
    total,
    unattributed: Math.max(0, total - summary.today.attributed),
  };
}

function handleFrame(frame: unknown): void {
  if (demoActive) return;
  if (typeof frame !== 'object' || frame === null) return;
  if (!('channel' in frame) || !('kind' in frame) || !('data' in frame)) return;
  if (frame.channel !== 'usage.totals' || frame.kind !== 'update') return;
  if (typeof frame.data !== 'object' || frame.data === null || !('today' in frame.data)) return;
  const today = frame.data.today;
  if (typeof today !== 'object' || today === null || !('up' in today) || !('down' in today)) return;
  if (typeof today.up !== 'number' || typeof today.down !== 'number') return;
  streamToday = { up: today.up, down: today.down };
  applyStreamTotals();
  // P7.5: the tray is owned by the shell; feed it today's total, never a
  // second data source. Ignore failures when running in the browser preview.
  void setTrayTooltip(`Net Watch Pro — today ${formatBytes(today.up + today.down)}`).catch(
    () => undefined,
  );
}

function enterDemo(): void {
  demoActive = true;
  summary = DEMO_SUMMARY;
  timeline = DEMO_TIMELINE;
  interfaces = DEMO_INTERFACES;
  streamToday = null;
  loading = false;
  error = null;
}

function exitDemo(): void {
  demoActive = false;
  void load();
}

async function load(): Promise<void> {
  if (demoActive || daemonStore.connection !== 'connected') return;
  const generation = ++requestGeneration;
  loading = true;
  error = null;
  try {
    const [nextSummary, nextTimeline, nextInterfaces] = await Promise.all([
      daemonStore.request('GET', '/api/v1/usage/summary'),
      daemonStore.request('GET', `/api/v1/usage/timeline?granularity=${granularity}`),
      daemonStore.request('GET', '/api/v1/usage/interfaces'),
    ]);
    if (generation !== requestGeneration) return;
    summary = nextSummary as Summary;
    timeline = (nextTimeline as { data: TimelinePoint[] }).data;
    interfaces = nextInterfaces as InterfaceUsage[];
    applyStreamTotals();
  } catch (err) {
    if (generation !== requestGeneration) return;
    error = err instanceof Error ? err.message : 'Usage request failed';
  } finally {
    if (generation === requestGeneration) loading = false;
  }
}

daemonStore.onFrame(handleFrame);

daemonStore.onConnection((state) => {
  if (demoActive) return;
  if (state === 'connected') {
    void daemonStore.subscribe('usage.totals').catch(() => undefined);
    void load();
  } else {
    reset();
  }
});

export const usageStore = {
  get summary(): Summary | null {
    return summary;
  },
  get timeline(): TimelinePoint[] {
    return timeline;
  },
  get interfaces(): InterfaceUsage[] {
    return interfaces;
  },
  get granularity(): Granularity {
    return granularity;
  },
  get streamToday(): { up: number; down: number } | null {
    return streamToday;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  get connection(): string {
    return daemonStore.connection;
  },
  get collecting(): boolean {
    return summary?.state === 'collecting' && timeline.length === 0 && interfaces.length === 0;
  },
  load,
  enterDemo,
  exitDemo,
  setGranularity(value: Granularity): void {
    if (granularity === value) return;
    granularity = value;
    void load();
  },
  retry(): void {
    void load();
  },
};
