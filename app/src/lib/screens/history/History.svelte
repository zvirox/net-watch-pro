<script lang="ts">
  // History screen — DESIGN.md §8.6 (content-only; the app shell owns
  // sidebar/topbar). Data comes from the history store (REST C2, keyset
  // pagination); infinite scroll reuses the Live screen's manual windowing
  // (fixed row height + spacers) and triggers the next page at the end.
  // Free tier: app search only; Pro filter chips are visibly locked and open
  // the upgrade modal (DESIGN.md §8.6). Errors/loading/empty per §7.9.
  import { onDestroy, onMount } from 'svelte';
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
  import { demoStore } from '$lib/stores/demo.svelte';
  import { historyStore, type HistoryRow } from '$lib/stores/history.svelte';
  import { licenseStore } from '$lib/stores/license.svelte';
  import { formatBytes } from '$lib/utils/format';

  // Manual windowing (same approach as Live §8.4): fixed row height, render
  // only the visible slice plus overscan; spacer rows keep the scroll height
  // stable so window shifts and page appends never move the scroll position.
  const ROW_H = 44;
  const OVERSCAN = 4;
  // Episodes can arrive after the first load (the daemon records them as
  // flows close); refresh while this screen is visible instead of showing a
  // stale empty state forever.
  const REFRESH_MS = 15_000;
  let refreshTimer: ReturnType<typeof window.setInterval> | undefined;

  onMount(() => {
    historyStore.load();
    refreshTimer = window.setInterval(() => {
      // A fresh page would reset the cursor and yank a reader out of older
      // pages; only poll while the first page is still empty.
      if (historyStore.rows.length === 0) historyStore.load();
    }, REFRESH_MS);
  });
  onDestroy(() => {
    if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  });

  let scrollTop = $state(0);
  let viewportH = $state(480);
  let expandedChip = $state<ChipKey | null>(null);

  type ChipKey = 'since' | 'remote_host' | 'remote_port' | 'protocol';
  type Chip = { key: ChipKey; label: string; tooltip: string };

  // Pro chips (API.md C2 advanced set). Locked chips stay visible so the
  // entitlement is discoverable instead of silently missing (DESIGN.md §7.5).
  const proChips: Chip[] = [
    { key: 'since', label: 'Date range', tooltip: 'Filter by start and end date' },
    { key: 'remote_host', label: 'Remote host', tooltip: 'Filter by hostname substring' },
    { key: 'remote_port', label: 'Port', tooltip: 'Filter by remote port' },
    { key: 'protocol', label: 'Protocol', tooltip: 'Filter by TCP, UDP, ICMP or other' },
  ];

  // Demo mode has no daemon but still renders its seeded rows (OQ4).
  const connected = $derived(demoStore.active || historyStore.connection === 'connected');
  const isPro = $derived(licenseStore.isPro);
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

  function onChipClick(key: ChipKey): void {
    if (!isPro) {
      // §7.5: a disabled Pro-gated control is never silent — it opens Upgrade.
      licenseStore.openUpgrade();
      return;
    }
    expandedChip = expandedChip === key ? null : key;
  }

  function dateToEpoch(value: string, endOfDay: boolean): number | null {
    if (!value) return null;
    const parsed = Date.parse(`${value}${endOfDay ? 'T23:59:59Z' : 'T00:00:00Z'}`);
    return Number.isNaN(parsed) ? null : Math.floor(parsed / 1000);
  }

  function onDateInput(key: 'since' | 'until', event: Event, endOfDay: boolean): void {
    const value = (event.currentTarget as HTMLInputElement).value;
    historyStore.setProFilter(key, dateToEpoch(value, endOfDay));
  }

  function onTextFilter(key: 'remote_host' | 'remote_port', event: Event): void {
    historyStore.setProFilter(key, (event.currentTarget as HTMLInputElement).value);
  }

  function onProtocol(event: Event): void {
    historyStore.setProFilter('protocol', (event.currentTarget as HTMLSelectElement).value);
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
    <!-- Filter row (§8.6): free search + visibly locked Pro chips -->
    <div class="filter-row">
      <Input
        class="filter-search"
        search
        placeholder="Search apps…"
        aria-label="Search history by app"
        oninput={onSearchInput}
      />
      <div class="chip-row" aria-label="Advanced filters">
        {#each proChips as chip (chip.key)}
          <span class="chip-wrap">
            <button
              type="button"
              class="chip"
              class:chip--locked={!isPro}
              class:chip--active={isPro && historyStore.hasProFilter(chip.key)}
              aria-disabled={!isPro}
              aria-expanded={isPro ? expandedChip === chip.key : undefined}
              aria-describedby={`tip-${chip.key}`}
              onclick={() => {
                onChipClick(chip.key);
              }}
            >
              {#if !isPro}
                <svg
                  class="lock"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  aria-hidden="true"
                >
                  <rect x="4" y="11" width="16" height="10" rx="2" /><path
                    d="M8 11V7a4 4 0 0 1 8 0v4"
                  />
                </svg>
              {/if}
              <span>{chip.label}</span>
            </button>
            <span id={`tip-${chip.key}`} class="tooltip" role="tooltip">
              {isPro ? chip.tooltip : `${chip.tooltip} · requires Pro`}
            </span>
          </span>
        {/each}
      </div>
    </div>

    {#if isPro && expandedChip !== null}
      <div class="filter-panel">
        {#if expandedChip === 'since'}
          <Input
            type="date"
            label="From"
            aria-label="History start date"
            oninput={(event: Event) => {
              onDateInput('since', event, false);
            }}
          />
          <Input
            type="date"
            label="To"
            aria-label="History end date"
            oninput={(event: Event) => {
              onDateInput('until', event, true);
            }}
          />
        {:else if expandedChip === 'remote_host'}
          <Input
            label="Hostname contains"
            placeholder="example.com"
            aria-label="Filter by remote host"
            oninput={(event: Event) => {
              onTextFilter('remote_host', event);
            }}
          />
        {:else if expandedChip === 'remote_port'}
          <Input
            type="number"
            min="0"
            max="65535"
            label="Remote port"
            placeholder="443"
            aria-label="Filter by remote port"
            oninput={(event: Event) => {
              onTextFilter('remote_port', event);
            }}
          />
        {:else}
          <label class="panel-field">
            <span>Protocol</span>
            <select
              class="filter-protocol"
              aria-label="Filter by protocol"
              value={historyStore.proFilters.protocol}
              onchange={onProtocol}
            >
              <option value="any">Any</option>
              <option value="tcp">TCP</option>
              <option value="udp">UDP</option>
              <option value="icmp">ICMP</option>
              <option value="other">Other</option>
            </select>
          </label>
        {/if}
      </div>
    {/if}

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
          <p class="more-note">
            End of results · {rows.length} rows · retention: {licenseStore.entitlements
              .retention_max_days} days
          </p>
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

  .filter-protocol:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
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

  /* §7.7/§8.6: Pro chips stay visible when locked (§7.5) and open Upgrade. */
  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--nwp-space-2);
  }
  .chip-wrap {
    position: relative;
    display: inline-flex;
  }
  .chip {
    display: inline-flex;
    min-height: 28px;
    align-items: center;
    gap: var(--nwp-space-2);
    padding: 2px var(--nwp-space-3);
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-surface);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
    cursor: pointer;
  }
  .chip:hover {
    border-color: var(--nwp-primary);
    color: var(--nwp-text);
  }
  .chip:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .chip--locked {
    border-color: var(--nwp-secondary-soft);
    background: var(--nwp-secondary-soft);
    color: var(--nwp-secondary);
  }
  .chip--locked:hover {
    border-color: var(--nwp-secondary);
    color: var(--nwp-secondary);
  }
  .chip--active {
    border-color: var(--nwp-primary);
    background: var(--nwp-primary-soft);
    color: var(--nwp-primary);
  }
  .lock {
    inline-size: 14px;
    block-size: 14px;
    flex: none;
  }

  /* §7.12: 300ms show / 0ms hide, hover + keyboard focus, no interactive content. */
  .tooltip {
    position: absolute;
    top: calc(100% + var(--nwp-space-2));
    left: 0;
    z-index: 20;
    max-inline-size: 280px;
    padding: var(--nwp-space-2);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    box-shadow: var(--nwp-shadow-md);
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition:
      opacity var(--nwp-dur-fast) var(--nwp-ease-standard),
      visibility 0ms linear var(--nwp-dur-fast);
  }
  .chip-wrap:hover .tooltip,
  .chip-wrap:focus-within .tooltip {
    opacity: 1;
    visibility: visible;
    transition:
      opacity var(--nwp-dur-base) var(--nwp-ease-standard) 300ms,
      visibility 0ms;
  }

  .filter-panel {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--nwp-space-3);
    padding: var(--nwp-space-3);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface-2);
  }
  .filter-panel :global(.nwp-field) {
    flex: 1 1 180px;
  }
  .panel-field {
    display: flex;
    flex: 1 1 180px;
    flex-direction: column;
    gap: var(--nwp-space-2);
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
  }
  .panel-field select {
    block-size: 36px;
    padding: var(--nwp-space-2);
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    color: var(--nwp-text);
    font: inherit;
  }
  @media (prefers-reduced-motion: reduce) {
    .tooltip {
      transition: none;
    }
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
    display: block;
    inline-size: 1px;
    block-size: 1px;
    max-inline-size: 1px;
    max-block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
