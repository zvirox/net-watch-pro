// Diagnostics modal state (Phase 9 P9.1; DESIGN.md §8.9). S3/S4 are read
// through the peer-credential API only; recovery reuses the one-time
// `enable_monitoring` pkexec command (idempotent enable --now + restart).

import { enableMonitoring } from '$lib/api/transport';
import { daemonStore } from './daemon.svelte';

export type CheckResult = 'pass' | 'fail' | 'skip';

export type Check = {
  name: string;
  result: CheckResult;
  detail?: string;
  guidance_url?: string;
};

export type JournalRow = {
  occurred_at: number;
  level: 'info' | 'warn' | 'error';
  code: string;
  message: string;
};

export type DiagnosticsPayload = {
  checks: Check[];
  journal_excerpt: JournalRow[];
  data: { db_size_bytes: number; retention_days: number };
};

let open = $state(false);
let payload = $state<DiagnosticsPayload | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let acting = $state(false);

async function load(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  loading = true;
  error = null;
  try {
    payload = (await daemonStore.request('GET', '/api/v1/diagnostics')) as DiagnosticsPayload;
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not load diagnostics';
  } finally {
    loading = false;
  }
}

function openModal(): void {
  open = true;
  void load();
}

function closeModal(): void {
  open = false;
  error = null;
}

/** One pkexec prompt: registers the UID if needed and enables/restarts the unit. */
async function startDaemon(): Promise<void> {
  acting = true;
  error = null;
  try {
    await enableMonitoring();
    await daemonStore.authorize();
    await load();
  } catch (err) {
    error = err instanceof Error ? err.message : 'Could not start the daemon';
  } finally {
    acting = false;
  }
}

export const diagnosticsStore = {
  get open(): boolean {
    return open;
  },
  get payload(): DiagnosticsPayload | null {
    return payload;
  },
  get loading(): boolean {
    return loading;
  },
  get error(): string | null {
    return error;
  },
  get acting(): boolean {
    return acting;
  },
  openModal,
  closeModal,
  load,
  startDaemon,
};
