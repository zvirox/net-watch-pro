// Typed settings store (ARCHITECTURE.md §6.1 tier 2): wraps T1/T2 so screens
// never hand-build request bodies. Reads the server's own bounds and labels.

import { daemonStore } from './daemon.svelte';

export type SettingEntry = {
  key: string;
  value: string | number | boolean;
  type: 'int' | 'bool' | 'enum';
  min?: number;
  max?: number;
  pro_max?: number;
  values?: string[];
  help?: string;
};

let entries = $state<SettingEntry[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

async function load(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  loading = true;
  error = null;
  try {
    entries = (await daemonStore.request('GET', '/api/v1/settings')) as SettingEntry[];
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not read settings';
  } finally {
    loading = false;
  }
}

async function setValue(key: string, value: string | number | boolean): Promise<void> {
  error = null;
  try {
    const updated = (await daemonStore.request('PUT', `/api/v1/settings/${key}`, {
      value,
    })) as SettingEntry;
    const index = entries.findIndex((entry) => entry.key === key);
    if (index >= 0) entries[index] = updated;
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not save setting';
    throw err;
  }
}

daemonStore.onConnection((state) => {
  if (state === 'connected') void load();
  else entries = [];
});

export const settingsStore = {
  get entries(): SettingEntry[] {
    return entries;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  get(key: string): SettingEntry | undefined {
    return entries.find((entry) => entry.key === key);
  },
  value(key: string): string | number | boolean | undefined {
    return this.get(key)?.value;
  },
  load,
  setValue,
};
