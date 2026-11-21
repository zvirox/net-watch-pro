// Live PC rate store (A5): reads only the daemon's `net.rates` stream. No
// sysfs access from the UI, no second data source (AGENTS.md §5 / API.md §3).

import { daemonStore } from './daemon.svelte';
import { settingsStore } from './settings.svelte';

export type RateFrame = {
  ts: number;
  down_bps: number;
  up_bps: number;
  interfaces: Array<{ name: string; down_bps: number; up_bps: number }>;
};

export type RatePoint = { down: number; up: number };

const STALE_MS = 3000;
const HISTORY_POINTS = 60;

let snapshot = $state<RateFrame | null>(null);
let history = $state<RatePoint[]>([]);
let now = $state(Date.now());
let lastWall = 0;
let timer: ReturnType<typeof setInterval> | null = null;
let unlisten: (() => void) | null = null;
let failed = $state(false);

function onFrame(frame: unknown): void {
  if (typeof frame !== 'object' || frame === null) return;
  const candidate = frame as { channel?: unknown; kind?: unknown; data?: unknown };
  if (candidate.channel !== 'net.rates' || candidate.kind !== 'update') return;
  const data = candidate.data as RateFrame | undefined;
  if (
    !data ||
    typeof data.down_bps !== 'number' ||
    typeof data.up_bps !== 'number' ||
    !Array.isArray(data.interfaces)
  ) {
    return;
  }
  snapshot = data;
  failed = false;
  lastWall = Date.now();
  now = lastWall;
  history = [...history.slice(-(HISTORY_POINTS - 1)), { down: data.down_bps, up: data.up_bps }];
}

function start(): void {
  if (unlisten === null) unlisten = daemonStore.onFrame(onFrame);
  void daemonStore.subscribe('net.rates').catch(() => {
    // No REST replacement exists for live rates; report the failure instead
    // of leaving the card on "Loading…" forever.
    failed = true;
  });
  if (timer === null) {
    timer = setInterval(() => {
      now = Date.now();
    }, 1000);
  }
}

function stop(): void {
  if (timer !== null) clearInterval(timer);
  timer = null;
  unlisten?.();
  unlisten = null;
  snapshot = null;
  history = [];
  lastWall = 0;
  failed = false;
}

export const ratesStore = {
  get snapshot(): RateFrame | null {
    return snapshot;
  },
  get history(): RatePoint[] {
    return history;
  },
  get failed(): boolean {
    return failed;
  },
  get stale(): boolean {
    return snapshot !== null && now - lastWall > STALE_MS;
  },
  get unit(): 'mbps' | 'mbytes' {
    return settingsStore.value('speed_unit') === 'mbytes' ? 'mbytes' : 'mbps';
  },
  get unitLabel(): string {
    return this.unit === 'mbytes' ? 'MB/s' : 'Mbps';
  },
  /** Decimal Mbps or MB/s from integer bits per second (API.md §3). */
  display(bps: number): string {
    if (!Number.isFinite(bps) || bps <= 0) return `0.0 ${this.unitLabel}`;
    const value = this.unit === 'mbytes' ? bps / 8 / 1_000_000 : bps / 1_000_000;
    return `${value.toFixed(value >= 100 ? 0 : 1)} ${this.unitLabel}`;
  },
  start,
  stop,
};
