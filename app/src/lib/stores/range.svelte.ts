// Shared usage range preset (Phase D; DESIGN.md §7.13, API.md §1.4).
// One persisted choice drives Dashboard, Apps and App detail. The server is
// the source of truth: the value comes from T1 `range_default` and is saved
// through T2, exactly like `theme_default`/`speed_unit`.

import { settingsStore } from './settings.svelte';

export type RangePreset = '1h' | '24h' | '7d' | '30d' | 'cycle';
export type RangeGranularity = '15m' | 'hour' | 'day';

const PRESETS: RangePreset[] = ['1h', '24h', '7d', '30d', 'cycle'];
const labels: Record<RangePreset, string> = {
  '1h': '1 h',
  '24h': '24 h',
  '7d': '7 d',
  '30d': '30 d',
  cycle: 'Cycle',
};

function isPreset(value: unknown): value is RangePreset {
  return typeof value === 'string' && PRESETS.includes(value as RangePreset);
}

let preset = $state<RangePreset>('24h');

function sync(): void {
  const stored = settingsStore.value('range_default');
  if (isPreset(stored)) preset = stored;
}

settingsStore.onChange(sync);

async function set(next: RangePreset): Promise<void> {
  if (preset === next) return;
  preset = next; // optimistic; settingsStore.onChange re-syncs from the server
  try {
    await settingsStore.setValue('range_default', next);
  } catch {
    sync();
  }
}

/** Automatic chart bucket for the current preset (D3). */
const granularity = $derived<RangeGranularity>(
  preset === '1h' || preset === '24h' ? '15m' : preset === '7d' ? 'hour' : 'day',
);

const bucketLabel = $derived(
  granularity === '15m' ? '15 min' : granularity === 'hour' ? 'Hour' : 'Day',
);

export const rangeStore = {
  get preset(): RangePreset {
    return preset;
  },
  get presets(): RangePreset[] {
    return PRESETS;
  },
  get label(): string {
    return labels[preset];
  },
  labelFor(value: RangePreset): string {
    return labels[value];
  },
  get granularity(): RangeGranularity {
    return granularity;
  },
  get bucketLabel(): string {
    return bucketLabel;
  },
  set,
};
