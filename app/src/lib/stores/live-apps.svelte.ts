// Live per-app rates store (Phase C C7): reads only the daemon's `live.apps`
// stream (API.md §3). No sysfs/files access from the UI. Rows are keyed by
// socket cgroup id; names and confidence come pre-resolved from the daemon.

import { daemonStore } from './daemon.svelte';

export type LiveAppRow = {
  cgroup_id: number;
  name: string;
  down_bps: number;
  up_bps: number;
  confidence: 'high' | 'low';
};

export type LiveAppsData = {
  ts: number;
  apps: LiveAppRow[];
  unattributed: { down_bps: number; up_bps: number };
};

const STALE_MS = 3000;

// Hold the previous ordering for ~2 s so small rate changes don't reshuffle
// the list every second (DESIGN.md §8.4 / Phase C C7).
let heldOrder: number[] = [];
let heldAt = 0;

function holdOrder(rows: LiveAppRow[], holdMs: number): LiveAppRow[] {
  const byRate = [...rows].sort((a, b) => b.down_bps + b.up_bps - (a.down_bps + a.up_bps));
  const fresh = heldOrder.length === 0 || Date.now() - heldAt >= holdMs;
  if (fresh) {
    heldOrder = byRate.map((row) => row.cgroup_id);
    heldAt = Date.now();
    return byRate;
  }
  const rank = new Map(heldOrder.map((id, index) => [id, index]));
  return byRate.sort(
    (a, b) =>
      (rank.get(a.cgroup_id) ?? Number.MAX_SAFE_INTEGER) -
      (rank.get(b.cgroup_id) ?? Number.MAX_SAFE_INTEGER),
  );
}

let data = $state<LiveAppsData | null>(null);
let now = $state(Date.now());
let lastWall = 0;
let unlisten: (() => void) | null = null;
let timer: ReturnType<typeof setInterval> | null = null;
let unsub: Promise<void> | null = null;
let failed = $state(false);

function onFrame(frame: unknown): void {
  if (typeof frame !== 'object' || frame === null) return;
  const candidate = frame as { channel?: unknown; kind?: unknown; data?: unknown };
  if (candidate.channel !== 'live.apps' || candidate.kind !== 'update') return;
  const raw: unknown = candidate.data;
  if (typeof raw !== 'object' || raw === null) return;
  const d = raw as Partial<LiveAppsData>;
  if (!Array.isArray(d.apps) || !d.unattributed || typeof d.ts !== 'number') return;
  failed = false;
  data = {
    ts: d.ts,
    apps: d.apps,
    unattributed: d.unattributed,
  };
  lastWall = Date.now();
  now = lastWall;
}

function start(): void {
  if (unlisten === null) unlisten = daemonStore.onFrame(onFrame);
  if (unsub === null) {
    // A subscribe failure must not become an unhandled rejection: client.send()
    // already flips the connection state, and the dashboard still has honest
    // no-collector/empty states for missing live rates.
    unsub = daemonStore.subscribe('live.apps').catch(() => {
      failed = true;
    });
  }
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
  data = null;
  lastWall = 0;
  failed = false;
  unsub = null;
  heldOrder = [];
  heldAt = 0;
}

export const liveAppsStore = {
  get data(): LiveAppsData | null {
    return data;
  },
  get rows(): LiveAppRow[] {
    return data?.apps ?? [];
  },
  get failed(): boolean {
    return failed;
  },
  get stale(): boolean {
    return data !== null && now - lastWall > STALE_MS;
  },
  /** Rows sorted by current total rate, with a ~2 s hold to limit jitter. */
  sorted(holdMs = 2000): LiveAppRow[] {
    const rows = [...this.rows];
    return holdOrder(rows, holdMs);
  },
  start,
  stop,
};
