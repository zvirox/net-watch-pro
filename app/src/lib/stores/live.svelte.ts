// Live connections data store (ARCHITECTURE.md §6.1 tier 2): server-pushed
// data kept verbatim, exposed as a derived sorted view so the screen stays
// dumb. Frames arrive over the WS `live.connections` channel (API.md §3):
// one snapshot on subscribe, then deltas keyed by row id. No client-side
// reconciliation of numbers; a lost connection clears the view — the fresh
// snapshot on resubscribe is the recovery mechanism.

import { daemonStore } from './daemon.svelte';

export type LiveProtocol = 'tcp' | 'udp' | 'icmp' | 'other';

export type LiveApp = {
  id: number;
  name: string;
  icon: string | null;
};

/** One live connection row (API.md C1; mirrors netwatch_common::LiveConnectionRow). */
export type LiveConnectionRow = {
  id: number;
  app: LiveApp;
  pid: number;
  process_name: string | null;
  protocol: LiveProtocol;
  local_ip: string;
  local_port: number;
  remote_ip: string;
  remote_port: number;
  remote_host: string | null;
  rate_up_bps: number;
  rate_down_bps: number;
  bytes_up: number;
  bytes_down: number;
};

type LiveSnapshotFrame = {
  channel: 'live.connections';
  kind: 'snapshot';
  data: LiveConnectionRow[];
};

type LiveDeltaFrame = {
  channel: 'live.connections';
  kind: 'delta';
  added: LiveConnectionRow[];
  updated: Array<{
    id: number;
    rate_up_bps: number;
    rate_down_bps: number;
    bytes_up: number;
    bytes_down: number;
  }>;
  removed: number[];
};

type LiveFrame = LiveSnapshotFrame | LiveDeltaFrame;

/** Sorted like the server's C1 default: activity (rate + bytes) desc, id asc. */
function byActivityDesc(a: LiveConnectionRow, b: LiveConnectionRow): number {
  const actA = a.rate_up_bps + a.rate_down_bps + a.bytes_up + a.bytes_down;
  const actB = b.rate_up_bps + b.rate_down_bps + b.bytes_up + b.bytes_down;
  return actB - actA || a.id - b.id;
}

const rowsById = $state(new Map<number, LiveConnectionRow>());
// Whether at least one snapshot arrived since the last (re)connect — the
// screen's loading state is "connected but no snapshot yet" (§7.9).
let loaded = $state(false);

daemonStore.onFrame((frame) => {
  if (!isLiveFrame(frame)) return;
  if (frame.kind === 'snapshot') {
    rowsById.clear();
    for (const row of frame.data) {
      rowsById.set(row.id, row);
    }
    loaded = true;
  } else {
    for (const row of frame.added) {
      rowsById.set(row.id, row);
    }
    for (const upd of frame.updated) {
      const existing = rowsById.get(upd.id);
      if (existing) {
        rowsById.set(upd.id, {
          ...existing,
          rate_up_bps: upd.rate_up_bps,
          rate_down_bps: upd.rate_down_bps,
          bytes_up: upd.bytes_up,
          bytes_down: upd.bytes_down,
        });
      }
    }
    for (const id of frame.removed) {
      rowsById.delete(id);
    }
  }
});

// Server is the source of truth (§6.1): a lost connection clears the view;
// on connect the fresh subscription delivers a new snapshot.
daemonStore.onConnection((state) => {
  if (state !== 'connected') {
    rowsById.clear();
    loaded = false;
    return;
  }
  void daemonStore.subscribe('live.connections').catch(() => {
    // Subscribe failure surfaces via the connection chip, not a screen error.
  });
});

export const liveStore = {
  /** Live rows, sorted activity-desc (the C1 default ordering). */
  get rows(): LiveConnectionRow[] {
    return [...rowsById.values()].sort(byActivityDesc);
  },
  /** True once a snapshot arrived after the latest (re)connect. */
  get loaded(): boolean {
    return loaded;
  },
  get connection(): string {
    return daemonStore.connection;
  },
};

function isLiveFrame(value: unknown): value is LiveFrame {
  if (typeof value !== 'object' || value === null) return false;
  const frame = value as { channel?: unknown; kind?: unknown };
  return (
    frame.channel === 'live.connections' && (frame.kind === 'snapshot' || frame.kind === 'delta')
  );
}
