// App Detail store (ARCHITECTURE.md §6.1 tier 2): A2/A3/A4 plus an
// app-scoped C2 page for the Connections tab. Read-only, no local writes.

import { daemonStore } from './daemon.svelte';
import type { HistoryRow } from './history.svelte';
import { rangeStore } from './range.svelte';
import { settingsStore } from './settings.svelte';

export type UsagePair = { up: number; down: number };

export type AppDetailData = {
  id: number;
  name: string;
  icon: string | null;
  attribution_confidence: string;
  first_seen_at: number;
  last_seen_at: number;
  exe_path: string;
  deleted_at: number | null;
  usage: { today: UsagePair; month: UsagePair; range: UsagePair };
};

export type TimelinePoint = {
  bucket_start: number;
  up: number;
  down: number;
  connections: number;
};

export type EndpointRow = {
  remote_ip: string;
  remote_host: string | null;
  port: number;
  bytes_total: number;
  connections: number;
  last_seen_at: number;
};

export type Granularity = '15m' | 'hour' | 'day';

let detail = $state<AppDetailData | null>(null);
let timeline = $state<TimelinePoint[]>([]);
let endpoints = $state<EndpointRow[]>([]);
let connections = $state<HistoryRow[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let currentId: number | null = null;
let requestGeneration = 0;

function reset(): void {
  requestGeneration += 1;
  detail = null;
  timeline = [];
  endpoints = [];
  connections = [];
  loading = false;
  error = null;
}

async function load(id: number): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  currentId = id;
  const generation = ++requestGeneration;
  loading = true;
  error = null;
  try {
    const idText = String(id);
    const [nextDetail, nextTimeline, nextEndpoints, nextConnections] = await Promise.all([
      daemonStore.request('GET', `/api/v1/apps/${idText}`),
      daemonStore.request('GET', `/api/v1/apps/${idText}/usage?range=${rangeStore.preset}`),
      daemonStore.request('GET', `/api/v1/apps/${idText}/endpoints`),
      daemonStore.request('GET', `/api/v1/connections?app_id=${idText}&limit=100`),
    ]);
    if (generation !== requestGeneration) return;
    detail = nextDetail as AppDetailData;
    timeline = (nextTimeline as { data: TimelinePoint[] }).data;
    endpoints = nextEndpoints as EndpointRow[];
    connections = (nextConnections as { data: HistoryRow[] }).data;
  } catch (err) {
    if (generation !== requestGeneration) return;
    error = err instanceof Error ? err.message : 'App detail request failed';
  } finally {
    if (generation === requestGeneration) loading = false;
  }
}

let loadedPreset = rangeStore.preset;
settingsStore.onChange(() => {
  if (rangeStore.preset === loadedPreset) return;
  loadedPreset = rangeStore.preset;
  if (currentId !== null) void load(currentId);
});

daemonStore.onConnection((state) => {
  if (state === 'connected') {
    if (currentId !== null) void load(currentId);
  } else {
    reset();
  }
});

export const appDetailStore = {
  get detail(): AppDetailData | null {
    return detail;
  },
  get timeline(): TimelinePoint[] {
    return timeline;
  },
  get endpoints(): EndpointRow[] {
    return endpoints;
  },
  get connections(): HistoryRow[] {
    return connections;
  },
  get granularity(): Granularity {
    return rangeStore.granularity;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  load,
  retry(): void {
    if (currentId !== null) void load(currentId);
  },
};
