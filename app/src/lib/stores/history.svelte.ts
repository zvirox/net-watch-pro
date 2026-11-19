// History data store (ARCHITECTURE.md §6.1 tier 2, read-only): serves the
// History screen from `GET /api/v1/connections` (API.md C2) with keyset
// pagination. REST-poll, not WS — history is not a live channel. The advanced
// filter keys are only sent while the license is Pro (API.md C2); the server
// independently enforces the same gate, never UI-only (DATABASE.md §6).

import { DEMO_ROWS } from '$lib/fixtures/demo';
import { daemonStore } from './daemon.svelte';
import { licenseStore } from './license.svelte';

export type HistoryApp = {
  id: number;
  name: string;
  icon: string | null;
};

/** One history row (API.md C2; mirrors netwatch_api::history::HistoryRow). */
export type HistoryRow = {
  id: number | null;
  kind?: 'aggregate';
  app: HistoryApp;
  pid: number | null;
  process_name: string | null;
  protocol: string;
  local_ip: string | null;
  local_port: number | null;
  remote_ip: string;
  remote_port: number;
  remote_host: string | null;
  bytes_up: number;
  bytes_down: number;
  packets_up: number | null;
  packets_down: number | null;
  connection_count: number | null;
  first_seen_at: number;
  last_seen_at: number;
};

type HistoryEnvelope = {
  data: HistoryRow[];
  next_cursor?: string;
  has_more: boolean;
};

/** Pro-only C2 filters (API.md C2 advanced set). */
export type ProFilterKey = 'since' | 'until' | 'remote_host' | 'remote_port' | 'protocol';

export type ProFilters = {
  since: number | null;
  until: number | null;
  remote_host: string;
  remote_port: string;
  protocol: string;
};

const EMPTY_PRO_FILTERS: ProFilters = {
  since: null,
  until: null,
  remote_host: '',
  remote_port: '',
  protocol: 'any',
};

/** Page size for history requests (API.md §1.4 default is 50; max 500). */
const PAGE_SIZE = 100;
/** Debounce for the app-search filter (type-then-query, not per-keystroke). */
const SEARCH_DEBOUNCE_MS = 300;

const rows = $state<HistoryRow[]>([]);
let hasMore = $state(false);
let loading = $state(false);
let loadingMore = $state(false);
let error = $state<string | null>(null);
let appSearch = $state('');
let proFilters = $state<ProFilters>({ ...EMPTY_PRO_FILTERS });
let cursor: string | null = null;
let requestGeneration = 0;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let demoActive = false;

function buildPath(nextCursor: string | null, search: string): string {
  // Manual query building (encodeURIComponent) — a mutable URLSearchParams
  // instance trips svelte/prefer-svelte-reactivity, and page URLs are local.
  const parts = [`limit=${String(PAGE_SIZE)}`];
  if (nextCursor !== null) parts.push(`cursor=${encodeURIComponent(nextCursor)}`);
  if (search.trim() !== '') parts.push(`app_name=${encodeURIComponent(search.trim())}`);
  // The advanced set leaves this client only while Pro; the API re-checks it.
  if (licenseStore.isPro) {
    if (proFilters.since !== null) parts.push(`since=${String(proFilters.since)}`);
    if (proFilters.until !== null) parts.push(`until=${String(proFilters.until)}`);
    if (proFilters.remote_host.trim() !== '') {
      parts.push(`remote_host=${encodeURIComponent(proFilters.remote_host.trim())}`);
    }
    if (proFilters.remote_port.trim() !== '') {
      parts.push(`remote_port=${encodeURIComponent(proFilters.remote_port.trim())}`);
    }
    if (proFilters.protocol !== 'any') parts.push(`protocol=${proFilters.protocol}`);
  }
  return `/api/v1/connections?${parts.join('&')}`;
}

/** Debounced page-1 refetch shared by every filter control. */
function scheduleFirstPage(): void {
  if (demoActive) return;
  if (searchTimer !== null) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    searchTimer = null;
    void fetchPage(null, 'first');
  }, SEARCH_DEBOUNCE_MS);
}

function clearProFilters(): void {
  const active =
    proFilters.since !== null ||
    proFilters.until !== null ||
    proFilters.remote_host !== '' ||
    proFilters.remote_port !== '' ||
    proFilters.protocol !== 'any';
  if (!active) return;
  proFilters = { ...EMPTY_PRO_FILTERS };
  scheduleFirstPage();
}

function enterDemo(): void {
  demoActive = true;
  rows.splice(0, rows.length, ...DEMO_ROWS);
  hasMore = false;
  cursor = null;
  loading = false;
  loadingMore = false;
  error = null;
}

function exitDemo(): void {
  demoActive = false;
  void fetchPage(null, 'first');
}

async function fetchPage(nextCursor: string | null, mode: 'first' | 'more'): Promise<void> {
  if (demoActive || daemonStore.connection !== 'connected') return;
  const generation = ++requestGeneration;
  if (mode === 'first') {
    loading = true;
  } else {
    loadingMore = true;
  }
  error = null;
  try {
    const result = await daemonStore.request('GET', buildPath(nextCursor, appSearch));
    // A newer request superseded this one (search changed mid-flight) —
    // discard the stale page instead of appending it out of order.
    if (generation !== requestGeneration) return;
    const envelope = result as HistoryEnvelope;
    if (mode === 'first') {
      rows.splice(0, rows.length, ...envelope.data);
    } else {
      rows.push(...envelope.data);
    }
    cursor = envelope.has_more ? (envelope.next_cursor ?? null) : null;
    hasMore = envelope.has_more && cursor !== null;
  } catch (err) {
    if (generation !== requestGeneration) return;
    error = err instanceof Error ? err.message : 'History request failed';
  } finally {
    if (generation === requestGeneration) {
      loading = false;
      loadingMore = false;
    }
  }
}

daemonStore.onConnection((state) => {
  if (demoActive) return;
  if (state === 'connected') {
    // Fresh link — refetch page 1 so the view reflects the daemon's data.
    void fetchPage(null, 'first');
  } else {
    requestGeneration += 1; // invalidate in-flight requests
    rows.splice(0, rows.length);
    cursor = null;
    hasMore = false;
    loading = false;
    loadingMore = false;
    error = null;
  }
});

// Losing Pro returns every lock: drop advanced filters so free-tier requests
// never carry them (the API would reject them with 403).
licenseStore.onTierChange((tier) => {
  if (tier === 'pro') {
    void fetchPage(null, 'first');
  } else {
    clearProFilters();
  }
});

export const historyStore = {
  /** Accumulated rows, oldest page first (server-ordered, API.md §1.4). */
  get rows(): HistoryRow[] {
    return rows;
  },
  /** True while the first page is in flight (§7.9 loading state). */
  get loading(): boolean {
    return loading;
  },
  /** True while a follow-up page is in flight (infinite-scroll footer). */
  get loadingMore(): boolean {
    return loadingMore;
  },
  /** Error message from the last request, or null (§7.9 error state). */
  get error(): string | null {
    return error;
  },
  /** Whether more pages exist beyond the accumulated rows. */
  get hasMore(): boolean {
    return hasMore;
  },
  get connection(): string {
    return daemonStore.connection;
  },
  /** Current app-name substring (also used as an export filter). */
  get appSearch(): string {
    return appSearch;
  },
  /** Advanced filter values (only sent while Pro — API.md C2). */
  get proFilters(): ProFilters {
    return proFilters;
  },
  hasProFilter(key: ProFilterKey): boolean {
    const value = proFilters[key];
    return value !== null && value !== '' && value !== 'any';
  },

  enterDemo,
  exitDemo,
  /** Set the app-name filter (free tier) and refetch page 1, debounced. */
  setAppSearch(value: string): void {
    appSearch = value;
    scheduleFirstPage();
  },

  /** Set one advanced filter and refetch page 1, debounced. */
  setProFilter(key: ProFilterKey, value: number | string | null): void {
    if (key === 'since' || key === 'until') {
      const seconds = typeof value === 'number' && Number.isFinite(value) ? value : null;
      proFilters = { ...proFilters, [key]: seconds };
    } else if (key === 'protocol') {
      proFilters = { ...proFilters, protocol: typeof value === 'string' ? value : 'any' };
    } else {
      proFilters = { ...proFilters, [key]: typeof value === 'string' ? value : '' };
    }
    scheduleFirstPage();
  },

  /** Fetch the next page when the viewport nears the end (infinite scroll). */
  loadMore(): void {
    if (demoActive || !hasMore || loading || loadingMore || cursor === null) return;
    void fetchPage(cursor, 'more');
  },

  /** Retry after an error: refetch page 1 with the current filter. */
  retry(): void {
    if (searchTimer !== null) clearTimeout(searchTimer);
    searchTimer = null;
    void fetchPage(null, 'first');
  },
};
