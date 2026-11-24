export interface RateSnapshot {
  ts: number;
  down_bps: number;
  up_bps: number;
  interfaces?: any[];
}

let lastFrameTime = 0;

export const ratesStore = {
  active: false,
  snapshot: null as RateSnapshot | null,
  stale: false,
  stop() {
    this.active = false;
    this.snapshot = null;
    this.stale = false;
    lastFrameTime = 0;
  },
  display(bps: number): string {
    if (!Number.isFinite(bps) || bps <= 0) return '0.0 Mbps';
    const mbps = bps / 1_000_000;
    if (mbps >= 100) return `${Math.round(mbps)} Mbps`;
    return `${mbps.toFixed(1)} Mbps`;
  },
};

export function onRatesFrame(frame: any) {
  if (frame && frame.channel === 'net.rates' && frame.data) {
    ratesStore.snapshot = frame.data;
    lastFrameTime = Date.now();
    ratesStore.stale = false;
  }
}

export function tickRatesNow(nowMs: number = Date.now()) {
  if (ratesStore.snapshot === null) {
    ratesStore.stale = false;
    return;
  }
  if (nowMs - lastFrameTime > 3000) {
    ratesStore.stale = true;
  } else {
    ratesStore.stale = false;
  }
}
