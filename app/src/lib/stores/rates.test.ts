import { describe, expect, it, beforeEach } from 'vitest';
import { ratesStore, onRatesFrame, tickRatesNow } from './rates.svelte';

describe('ratesStore', () => {
  beforeEach(() => {
    ratesStore.stop();
  });

  describe('display formatting', () => {
    it('formats zero or negative values as 0.0', () => {
      expect(ratesStore.display(0)).toMatch(/^0\.0/);
      expect(ratesStore.display(-500)).toMatch(/^0\.0/);
      expect(ratesStore.display(NaN)).toMatch(/^0\.0/);
    });

    it('formats normal bps rates into Mbps', () => {
      // 10,000,000 bps -> 10.0 Mbps
      expect(ratesStore.display(10_000_000)).toMatch(/10\.0/);
    });

    it('formats huge rates without decimal places', () => {
      // 500,000,000 bps -> 500 Mbps
      expect(ratesStore.display(500_000_000)).toMatch(/500/);
    });
  });

  describe('stale logic', () => {
    it('is not stale when no snapshot exists', () => {
      expect(ratesStore.snapshot).toBeNull();
      expect(ratesStore.stale).toBe(false);
    });

    it('is not stale immediately upon receiving a frame', () => {
      const now = Date.now();
      tickRatesNow(now);
      onRatesFrame({
        channel: 'net.rates',
        kind: 'update',
        data: {
          ts: Math.floor(now / 1000),
          down_bps: 1_000_000,
          up_bps: 500_000,
          interfaces: [],
        },
      });

      expect(ratesStore.snapshot).not.toBeNull();
      expect(ratesStore.stale).toBe(false);
    });

    it('becomes stale when more than 3000 ms elapse without new frames', () => {
      const now = Date.now();
      onRatesFrame({
        channel: 'net.rates',
        kind: 'update',
        data: {
          ts: Math.floor(now / 1000),
          down_bps: 1_000_000,
          up_bps: 500_000,
          interfaces: [],
        },
      });

      expect(ratesStore.stale).toBe(false);

      // Advance time by 3001 ms
      tickRatesNow(Date.now() + 3001);
      expect(ratesStore.stale).toBe(true);
    });
  });
});
