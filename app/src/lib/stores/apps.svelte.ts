// Apps list screen data (Phase D D4; DESIGN.md §8.10). Reads A1 through the
// peer-credential API only — never SQLite, /proc or sysfs from the UI.

import { daemonStore } from './daemon.svelte';
import { rangeStore } from './range.svelte';
import { settingsStore } from './settings.svelte';

export type DirectionPair = { up: number; down: number };

export type AppRow = {
  id: number;
  name: string;
  icon: string | null;
  attribution_confidence: string;
  first_seen_at: number;
  last_seen_at: number;
  usage: { today: DirectionPair; month: DirectionPair; range: DirectionPair };
};

export type AppsSort = 'speed' | 'total' | 'name';

let rows = $state<AppRow[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let search = $state('');
let sort = $state<AppsSort>('speed');
let generation = 0;

async function load(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  const current = ++generation;
  loading = true;
  error = null;
  try {
    const result = (await daemonStore.request(
      'GET',
      `/api/v1/apps?range=${rangeStore.preset}&sort=usage_range&order=desc&limit=200`,
    )) as AppRow[];
    if (current !== generation) return;
    rows = result;
  } catch (err) {
    if (current !== generation) return;
    error = err instanceof Error ? err.message : 'Could not load apps';
  } finally {
    if (current === generation) loading = false;
  }
}

let loadedPreset = rangeStore.preset;
settingsStore.onChange(() => {
  if (rangeStore.preset === loadedPreset) return;
  loadedPreset = rangeStore.preset;
  void load();
});

daemonStore.onConnection((state) => {
  if (state === 'connected') void load();
  else {
    generation += 1;
    rows = [];
    loading = false;
    error = null;
  }
});

export const appsStore = {
  get rows(): AppRow[] {
    return rows;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  get search(): string {
    return search;
  },
  get sort(): AppsSort {
    return sort;
  },
  setSearch(value: string): void {
    search = value;
  },
  setSort(value: AppsSort): void {
    sort = value;
  },
  load,
};
