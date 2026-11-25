// Shared byte & rate formatters (tabular numerals, compact units).
export function formatBytes(bytes: number): string {
  // A negative byte count is invalid, never traffic; show "—" rather than
  // dumping a raw negative integer into a headline card (D3a).
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes < 1024) return `${String(Math.round(bytes))} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const label = units[Math.min(unit, units.length - 1)] ?? 'B';
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${label}`;
}

export function formatRateParts(
  bps: number,
  speedUnit: 'mbps' | 'mbytes' = 'mbps',
): { value: string; unit: string } {
  if (!Number.isFinite(bps) || bps <= 0) {
    return { value: '0.0', unit: speedUnit === 'mbytes' ? 'MB/s' : 'Mbps' };
  }
  if (speedUnit === 'mbytes') {
    const bytesPerSec = bps / 8;
    if (bytesPerSec < 1000) return { value: String(Math.round(bytesPerSec)), unit: 'B/s' };
    if (bytesPerSec < 1_000_000) return { value: (bytesPerSec / 1000).toFixed(1), unit: 'KB/s' };
    const mb = bytesPerSec / 1_000_000;
    return { value: mb >= 100 ? mb.toFixed(0) : mb.toFixed(1), unit: 'MB/s' };
  }
  const mbps = bps / 1_000_000;
  if (mbps < 0.1) {
    const kbps = bps / 1000;
    return { value: kbps >= 10 ? kbps.toFixed(0) : kbps.toFixed(1), unit: 'Kbps' };
  }
  return { value: mbps >= 100 ? mbps.toFixed(0) : mbps.toFixed(1), unit: 'Mbps' };
}

export function formatRateText(bps: number, speedUnit: 'mbps' | 'mbytes' = 'mbps'): string {
  const parts = formatRateParts(bps, speedUnit);
  return `${parts.value} ${parts.unit}`;
}

export function formatDateTime(epochSec: number): string {
  if (!Number.isFinite(epochSec) || epochSec <= 0) return '—';
  return new Date(epochSec * 1000).toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function formatShortTime(epochSec: number): string {
  if (!Number.isFinite(epochSec) || epochSec <= 0) return '—';
  return new Date(epochSec * 1000).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });
}
