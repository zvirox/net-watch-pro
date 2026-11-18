// Export store (P6.8): turns the History filter set into an E1 job, waits for
// E2, reads E3 as raw text, then saves it through the OS picker (API.md §2.3).
// Pro is required for every export; the API independently re-checks the tier.

import { isTauri } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { saveExportFile } from '$lib/api/transport';
import { daemonStore } from './daemon.svelte';
import { historyStore } from './history.svelte';
import { licenseStore } from './license.svelte';

export type ExportFormat = 'csv' | 'json';

type ExportFilters = Record<string, string | number>;

let busy = $state(false);
let status = $state<string | null>(null);
let error = $state<string | null>(null);

/** Reuse the filters currently shaping the History view (C2 whitelist). */
function currentFilters(): ExportFilters {
  const filters: ExportFilters = {};
  const app = historyStore.appSearch.trim();
  if (app !== '') filters.app_name = app;
  const pro = historyStore.proFilters;
  if (pro.since !== null) filters.since = pro.since;
  if (pro.until !== null) filters.until = pro.until;
  if (pro.remote_host.trim() !== '') filters.remote_host = pro.remote_host.trim();
  if (pro.remote_port.trim() !== '') {
    const port = Number(pro.remote_port);
    if (Number.isInteger(port) && port >= 0 && port <= 65_535) filters.remote_port = port;
  }
  if (pro.protocol !== 'any') filters.protocol = pro.protocol;
  return filters;
}

function dateStamp(): string {
  return new Date().toISOString().slice(0, 10).replaceAll('-', '');
}

async function download(format: ExportFormat): Promise<void> {
  if (!licenseStore.isPro) {
    licenseStore.openUpgrade();
    return;
  }
  if (daemonStore.connection !== 'connected') {
    error = 'The daemon is not linked. Export is unavailable.';
    return;
  }
  if (!isTauri()) {
    error = 'Saving an export requires the desktop app.';
    return;
  }
  const filters = currentFilters();
  if (Object.keys(filters).length === 0) {
    error = 'Set at least one History filter before exporting — full dumps are blocked.';
    return;
  }

  busy = true;
  error = null;
  status = 'Preparing export…';
  try {
    const created = (await daemonStore.request('POST', '/api/v1/exports', {
      scope: 'connections',
      format,
      filters,
    })) as { id: string };

    status = 'Generating file…';
    let state = 'queued';
    for (let attempt = 0; attempt < 20; attempt += 1) {
      const job = (await daemonStore.request('GET', `/api/v1/exports/${created.id}`)) as {
        state: string;
      };
      state = job.state;
      if (state === 'done') break;
      if (state === 'expired') throw new Error('The export job expired');
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 150);
      });
    }
    if (state !== 'done') throw new Error('The export did not finish in time');

    const content = await daemonStore.requestText('GET', `/api/v1/exports/${created.id}/content`);
    const extension = format === 'csv' ? 'csv' : 'json';
    const filename = `netwatch-pro-export-${dateStamp()}.${extension}`;
    const path = await save({
      title: 'Save export',
      defaultPath: filename,
      filters: [
        {
          name: format === 'csv' ? 'CSV' : 'JSON',
          extensions: [extension],
        },
      ],
    });
    if (path === null) {
      status = 'Save cancelled';
      return;
    }
    await saveExportFile(path, content);
    status = `Saved ${path.split('/').pop() ?? path}`;
  } catch (err) {
    error = err instanceof Error ? err.message : 'Export failed';
    status = null;
  } finally {
    busy = false;
  }
}

function clear(): void {
  status = null;
  error = null;
}

export const exportStore = {
  get busy(): boolean {
    return busy;
  },
  get status(): string | null {
    return status;
  },
  get error(): string | null {
    return error;
  },
  download,
  clear,
};
