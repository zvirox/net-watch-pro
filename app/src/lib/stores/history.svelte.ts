// History data store (ARCHITECTURE.md §6.1 tier 2, read-only): serves the
// History screen from `GET /api/v1/connections` (API.md C2) with keyset
// pagination. REST-poll, not WS — history is not a live channel. Free tier
// sends at most the `app_name` filter; the Pro filter set arrives in P6.7
// (the server already enforces the gate, API.md C2).

import { daemonStore } from './daemon.svelte';

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
let cursor: string | null = null;
let requestGeneration = 0;
let searchTimer: ReturnType<typeof setTimeout> | null = null;

function buildPath(nextCursor: string | null, search: string): string {
  // Manual query building (encodeURIComponent) — a mutable URLSearchParams
  // instance trips svelte/prefer-svelte-reactivity, and page URLs are local.
  const parts = [`limit=${String(PAGE_SIZE)}`];
  if (nextCursor !== null) parts.push(`cursor=${encodeURIComponent(nextCursor)}`);
  if (search.trim() !== '') parts.push(`app_name=${encodeURIComponent(search.trim())}`);
  return `/api/v1/connections?${parts.join('&')}`;
}

async function fetchPage(nextCursor: string | null, mode: 'first' | 'more'): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
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

  /** Set the app-name filter (free tier) and refetch page 1, debounced. */
  setAppSearch(value: string): void {
    appSearch = value;
    if (searchTimer !== null) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      searchTimer = null;
      void fetchPage(null, 'first');
    }, SEARCH_DEBOUNCE_MS);
  },

  /** Fetch the next page when the viewport nears the end (infinite scroll). */
  loadMore(): void {
    if (!hasMore || loading || loadingMore || cursor === null) return;
    void fetchPage(cursor, 'more');
  },

  /** Retry after an error: refetch page 1 with the current filter. */
  retry(): void {
    if (searchTimer !== null) clearTimeout(searchTimer);
    searchTimer = null;
    void fetchPage(null, 'first');
  },
};
