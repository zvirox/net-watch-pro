// Interfaces store (P7.4): I1 list + I2 flag updates. Metered labels feed
// D6a budgets; exclusion changes headline totals, so usage is reloaded after
// every successful patch (ARCHITECTURE.md §6.1 tier 2).

import { daemonStore } from './daemon.svelte';
import { usageStore } from './usage.svelte';

export type InterfaceRecord = {
  id: number;
  name: string;
  kind: string;
  is_metered: boolean;
  is_excluded: boolean;
  bytes: { up: number; down: number };
};

let rows = $state<InterfaceRecord[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

async function load(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  loading = true;
  error = null;
  try {
    rows = (await daemonStore.request('GET', '/api/v1/interfaces')) as InterfaceRecord[];
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not read interfaces';
  } finally {
    loading = false;
  }
}

async function setFlag(
  id: number,
  key: 'is_metered' | 'is_excluded',
  value: boolean,
): Promise<void> {
  error = null;
  try {
    const updated = (await daemonStore.request('PATCH', `/api/v1/interfaces/${String(id)}`, {
      [key]: value,
    })) as InterfaceRecord;
    const index = rows.findIndex((row) => row.id === id);
    if (index >= 0) rows[index] = updated;
    // Dashboard cards + budget totals are derived from U3/U1 — refresh them.
    void usageStore.load();
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not update interface';
  }
}

daemonStore.onConnection((state) => {
  if (state === 'connected') void load();
  else rows = [];
});

export const interfacesStore = {
  get rows(): InterfaceRecord[] {
    return rows;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  load,
  setFlag,
};
