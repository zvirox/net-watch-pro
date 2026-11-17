<script lang="ts">
  // History screen — DESIGN.md §8.6 (content-only; the app shell owns
  // sidebar/topbar). Data comes from the history store (REST C2, keyset
  // pagination); infinite scroll reuses the Live screen's manual windowing
  // (fixed row height + spacers) and triggers the next page at the end.
  // Free tier: app search only — the Pro filter chips/Export land in P6.7/
  // P6.8 (§8.6). Errors/loading/empty states per DESIGN.md §7.9/§8.6.
  import {
    Button,
    Input,
    Table,
    TableHeader,
    TableBody,
    TableRow,
    TableHead,
    TableCell,
  } from '$lib/components/ui';
  import { historyStore, type HistoryRow } from '$lib/stores/history.svelte';
  import { formatBytes } from '$lib/utils/format';

  // Manual windowing (same approach as Live §8.4): fixed row height, render
  // only the visible slice plus overscan; spacer rows keep the scroll height
  // stable so window shifts and page appends never move the scroll position.
  const ROW_H = 44;
  const OVERSCAN = 4;

  let scrollTop = $state(0);
  let viewportH = $state(480);

  const connected = $derived(historyStore.connection === 'connected');
  const rows = $derived(historyStore.rows);
  const winStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const winEnd = $derived(
    Math.min(rows.length, winStart + Math.ceil(viewportH / ROW_H) + 2 * OVERSCAN),
  );
  const windowed = $derived(rows.slice(winStart, winEnd));

  function onScroll(event: Event & { currentTarget: HTMLDivElement }): void {
    scrollTop = event.currentTarget.scrollTop;
    // Infinite scroll: the window reached the end of the loaded rows and the
    // server has more — fetch the next page (§8.6 pagination).
    if (winEnd >= rows.length && historyStore.hasMore) historyStore.loadMore();
  }

  function onSearchInput(event: Event): void {
    historyStore.setAppSearch((event.currentTarget as HTMLInputElement).value);
  }

  function fmtDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleDateString([], {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }

  function fmtTime(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function appInitial(row: HistoryRow): string {
    return (row.app.name || row.process_name || '?').charAt(0).toUpperCase();
  }
</script>

<section class="history" aria-label="Usage history">
  {#if !connected}
    <div class="state-card" role="status">
      <strong>History unavailable</strong>
      <p>The daemon is not linked. History will appear once the connection is restored.</p>
    </div>
  {:else}
    <!-- Filter row (§8.6): free tier = app search; Pro chips land in P6.7 -->
    <div class="filter-row">
      <Input
        class="filter-search"
        search
        placeholder="Search apps…"
        aria-label="Search history by app"
        oninput={onSearchInput}
      />
    </div>

    {#if historyStore.loading}
      <!-- Loading (§7.9): first page in flight -->
      <div class="state-card" role="status">
        <strong>Loading history…</strong>
        <p>Querying recorded usage on this device.</p>
      </div>
    {:else if historyStore.error !== null}
      <!-- Error (§7.9): honest message + retry, never a dead end -->
      <div class="state-card state-card--error" role="alert">
        <strong>History could not be loaded</strong>
        <p>{historyStore.error}</p>
        <Button
          variant="secondary"
          size="sm"
          onclick={() => {
            historyStore.retry();
          }}
        >
          Try again
        </Button>
      </div>
    {:else if rows.length === 0}
      <!-- Empty (§7.9): honest — no history yet or nothing matches -->
      <div class="state-card" role="status">
        <strong>No usage recorded</strong>
        <p>
          Raw episodes are kept for the raw window and daily aggregates beyond it. History appears
          here as the daemon records traffic.
        </p>
      </div>
    {:else}
      <div
        class="viewport"
        onscroll={onScroll}
        bind:clientHeight={viewportH}
        role="region"
        aria-label="Usage history table"
      >
        <Table>
          <caption class="sr-only">Usage history</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">Date</TableHead>
              <TableHead scope="col">App</TableHead>
              <TableHead scope="col">Remote</TableHead>
              <TableHead scope="col" numeric>▲ Up</TableHead>
              <TableHead scope="col" numeric>▼ Down</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {#if winStart > 0}
              <tr class="spacer" aria-hidden="true" style="height:{winStart * ROW_H}px">
                <td colspan="5"></td>
              </tr>
            {/if}
            {#each windowed as row (row.id ?? `agg-${String(row.first_seen_at)}-${row.remote_ip}`)}
              <tr class="history-row" style="height:{ROW_H}px">
                <TableCell>
                  <span class="date-cell">
                    <span
                      >{row.kind === 'aggregate'
                        ? fmtDate(row.first_seen_at)
                        : fmtDate(row.last_seen_at)}</span
                    >
                    {#if row.kind === 'aggregate'}
                      <span class="date-sub">day total</span>
                    {:else}
                      <span class="date-sub"
                        >{fmtTime(row.first_seen_at)}–{fmtTime(row.last_seen_at)}</span
                      >
                    {/if}
                  </span>
                </TableCell>
                <TableCell>
                  <span class="app-cell">
                    <span class="app-icon" aria-hidden="true">{appInitial(row)}</span>
                    <span class="app-name">
                      {row.app.name}
                      {#if row.kind === 'aggregate'}
                        <span class="agg-tag">aggregated</span>
                      {:else if row.process_name && row.process_name !== row.app.name}
                        <span class="app-process">{row.process_name}</span>
                      {/if}
                    </span>
                  </span>
                </TableCell>
                <TableCell mono>
                  <span class="remote-cell">
                    <span>{row.remote_ip}:{row.remote_port}</span>
                    {#if row.remote_host}
                      <span class="remote-host">{row.remote_host}</span>
                    {/if}
                  </span>
                </TableCell>
                <TableCell mono numeric>{formatBytes(row.bytes_up)}</TableCell>
                <TableCell mono numeric>{formatBytes(row.bytes_down)}</TableCell>
              </tr>
            {/each}
            {#if rows.length - winEnd > 0}
              <tr
                class="spacer"
                aria-hidden="true"
                style="height:{(rows.length - winEnd) * ROW_H}px"
              >
                <td colspan="5"></td>
              </tr>
            {/if}
          </TableBody>
        </Table>
        {#if historyStore.loadingMore}
          <p class="more-note" role="status">Loading more…</p>
        {:else if !historyStore.hasMore}
          <p class="more-note">End of results · {rows.length} rows loaded</p>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: var(--nwp-space-3);
    min-block-size: 0;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  /* The class lives on the Input child-component wrapper — :global needed. */
  .filter-row :global(.filter-search) {
    flex: 1 1 240px;
    max-inline-size: 340px;
  }

  .state-card {
    display: flex;
    flex-direction: column;
    gap: var(--nwp-space-1);
    padding: var(--nwp-space-5);
    background: var(--nwp-surface);
    border-radius: var(--nwp-radius-lg);
  }
  .state-card strong {
    font-size: var(--nwp-size-title-3);
    color: var(--nwp-text);
  }
  .state-card p {
    margin: 0;
    font-size: var(--nwp-size-body);
    color: var(--nwp-text-secondary);
  }
  .state-card--error strong {
    color: var(--nwp-danger);
  }

  .viewport {
    max-block-size: 60vh;
    overflow-y: auto;
    background: var(--nwp-surface);
    border-radius: var(--nwp-radius-lg);
  }

  .spacer td {
    padding: 0;
    border: none;
  }

  /* Body cells live inside the vendored TableCell component — reach them
     via :global under the parent-scoped row (same as Live §8.4). */
  .history-row :global(.nwp-table-cell) {
    block-size: 100%;
    padding-block: 0;
  }

  .date-cell {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    white-space: nowrap;
  }
  .date-sub {
    font-size: var(--nwp-size-caption);
    color: var(--nwp-text-secondary);
  }

  .app-cell {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  .app-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 28px;
    block-size: 28px;
    flex: none;
    font: 600 var(--nwp-size-label) var(--nwp-font-ui);
    color: var(--nwp-text);
    background: var(--nwp-surface-2);
    border-radius: var(--nwp-radius-md);
  }
  .app-name {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    overflow: hidden;
  }
  .app-process {
    font-size: var(--nwp-size-caption);
    color: var(--nwp-text-secondary);
  }
  .agg-tag {
    align-self: flex-start;
    padding: 0 var(--nwp-space-2);
    font: var(--nwp-size-caption) var(--nwp-font-mono);
    color: var(--nwp-text-secondary);
    background: var(--nwp-surface-2);
    border-radius: var(--nwp-radius-full);
  }

  .remote-cell {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    overflow: hidden;
  }
  .remote-host {
    font-size: var(--nwp-size-caption);
    color: var(--nwp-text-secondary);
  }

  .more-note {
    margin: 0;
    padding: var(--nwp-space-2) var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }

  .sr-only {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
