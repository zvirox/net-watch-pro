// Shared byte formatter (DESIGN.md §4.3: mono, tabular, compact units).
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 1024) return `${String(Math.round(bytes))} B`;
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
