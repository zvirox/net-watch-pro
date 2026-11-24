import { describe, expect, it, beforeEach } from 'vitest';
import { holdOrder, liveAppsStore, type LiveAppRow } from './live-apps';

describe('live-apps store holdOrder', () => {
  beforeEach(() => {
    liveAppsStore.stop();
  });

  const appA: LiveAppRow = {
    cgroup_id: 101,
    name: 'Browser',
    down_bps: 1_000_000,
    up_bps: 500_000,
    confidence: 'high',
  };

  const appB: LiveAppRow = {
    cgroup_id: 102,
    name: 'Downloader',
    down_bps: 500_000,
    up_bps: 100_000,
    confidence: 'high',
  };

  const appC: LiveAppRow = {
    cgroup_id: 103,
    name: 'Chat',
    down_bps: 50_000,
    up_bps: 10_000,
    confidence: 'high',
  };

  it('orders by rate descending on initial fresh snapshot', () => {
    const rows = [appC, appA, appB];
    const ordered = holdOrder(rows, 2000);
    expect(ordered.map((r) => r.cgroup_id)).toEqual([101, 102, 103]);
  });

  it('holds previous order within holdMs even if app B momentarily exceeds app A', () => {
    // Initial snapshot establishing order: [101, 102, 103]
    holdOrder([appA, appB, appC], 2000);

    // App B speeds up to 2_000_000, App A slows down to 100_000
    const appAFaster: LiveAppRow = { ...appA, down_bps: 100_000 };
    const appBFaster: LiveAppRow = { ...appB, down_bps: 2_000_000 };

    const held = holdOrder([appBFaster, appAFaster, appC], 2000);
    // Order is held: 101 remains first, then 102, then 103
    expect(held.map((r) => r.cgroup_id)).toEqual([101, 102, 103]);
  });

  it('refreshes order when holdMs has elapsed', async () => {
    // Initial order: [101, 102, 103]
    holdOrder([appA, appB, appC], 50);

    // Wait > 50 ms
    await new Promise((resolve) => setTimeout(resolve, 60));

    // App B is now faster
    const appASlower: LiveAppRow = { ...appA, down_bps: 100_000 };
    const appBFaster: LiveAppRow = { ...appB, down_bps: 2_000_000 };

    const reordered = holdOrder([appASlower, appBFaster, appC], 50);
    expect(reordered.map((r) => r.cgroup_id)).toEqual([102, 101, 103]);
  });
});
