<script lang="ts">
  // Live Connections screen — DESIGN.md §8.4 (content-only; the app shell
  // owns sidebar/topbar). Data comes from the live store (WS deltas); the
  // store always updates in background, pause freezes a display copy here.
  // Errors/loading/empty states per DESIGN.md §7.9/§8.4.
  import { fade } from 'svelte/transition';
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
  import { liveStore, type LiveConnectionRow } from '$lib/stores/live.svelte';

  // Manual windowing (~§8.4 "virtualized"): fixed row height, render only
  // the visible slice plus overscan; spacer rows keep the scroll height
  // stable so window shifts never move the scroll position.
  const ROW_H = 44;
  const OVERSCAN = 4;

  let search = $state('');
  let protocol = $state<'all' | 'tcp' | 'udp' | 'icmp' | 'other'>('all');
  let paused = $state(false);
  let frozenAt = $state('');
  let frozenRows = $state<LiveConnectionRow[]>([]);

  let scrollTop = $state(0);
  let viewportH = $state(480);

  const connected = $derived(liveStore.connection === 'connected');
  const displayRows = $derived(paused ? frozenRows : liveStore.rows);
  const filtered = $derived.by(() => {
    const needle = search.trim().toLowerCase();
    return displayRows.filter((row) => {
      if (protocol !== 'all' && row.protocol !== protocol) return false;
      if (needle === '') return true;
      return (
        row.app.name.toLowerCase().includes(needle) ||
        (row.process_name?.toLowerCase().includes(needle) ?? false) ||
        row.remote_ip.includes(needle) ||
        (row.remote_host?.toLowerCase().includes(needle) ?? false)
      );
    });
  });
  const maxRate = $derived(
    filtered.reduce((max, row) => Math.max(max, row.rate_up_bps + row.rate_down_bps), 0),
  );
  const totalRate = $derived(
    filtered.reduce((sum, row) => sum + row.rate_up_bps + row.rate_down_bps, 0),
  );
  const winStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const winEnd = $derived(
    Math.min(filtered.length, winStart + Math.ceil(viewportH / ROW_H) + 2 * OVERSCAN),
  );
  const windowed = $derived(filtered.slice(winStart, winEnd));

  function togglePause(): void {
    if (paused) {
      paused = false;
      return;
    }
    // Freeze the current display copy; the store keeps updating in the
    // background (server stays the source of truth, §6.1).
    frozenRows = liveStore.rows;
    frozenAt = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    paused = true;
  }

  function onSearchInput(event: Event): void {
    search = (event.currentTarget as HTMLInputElement).value;
  }

  function onScrollUpdate(event: Event & { currentTarget: HTMLDivElement }): void {
    scrollTop = event.currentTarget.scrollTop;
  }

  function fmtRate(bps: number): string {
    if (bps < 1024) return `${String(Math.round(bps))} B/s`;
    const units = ['KB/s', 'MB/s', 'GB/s', 'TB/s'];
    let value = bps / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    const unitLabel = units[Math.min(unit, units.length - 1)] ?? 'B/s';
    return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${unitLabel}`;
  }

  function appInitial(row: LiveConnectionRow): string {
    return (row.app.name || row.process_name || '?').charAt(0).toUpperCase();
  }
</script>

<section class="live" aria-label="Live connections">
  {#if !connected}
    <div class="state-card" role="status">
      <strong>Live view unavailable</strong>
      <p>The daemon is not linked. Live connections will appear once the connection is restored.</p>
    </div>
  {:else}
    <!-- Filter row (§8.4): search + protocol + mono live total + pause -->
    <div class="filter-row">
      <Input
        class="filter-search"
        search
        placeholder="Search apps, remotes…"
        aria-label="Filter live connections"
        value={search}
        oninput={onSearchInput}
      />
      <select class="filter-protocol" aria-label="Filter by protocol" bind:value={protocol}>
        <option value="all">All protocols</option>
        <option value="tcp">TCP</option>
        <option value="udp">UDP</option>
        <option value="icmp">ICMP</option>
        <option value="other">Other</option>
      </select>
      <span class="live-total" aria-live="off">{fmtRate(totalRate)}</span>
      <Button
        variant={paused ? 'primary' : 'secondary'}
        size="sm"
        aria-label={paused ? 'Resume live updates' : 'Pause live updates'}
        aria-pressed={paused}
        onclick={togglePause}
      >
        {paused ? '▶' : '⏸'}
      </Button>
    </div>

    {#if paused}
      <!-- Frozen banner (§8.4) -->
      <div class="frozen-banner" role="status">
        <span>Frozen at {frozenAt}</span>
        <Button variant="ghost" size="sm" onclick={togglePause}>Resume</Button>
      </div>
    {/if}

    {#if !liveStore.loaded}
      <!-- Loading: connected, waiting for the first snapshot (§7.9) -->
      <div class="state-card" role="status">
        <strong>Loading live data…</strong>
        <p>Waiting for the first snapshot from the daemon.</p>
      </div>
    {:else if filtered.length === 0}
      <!-- Empty (§7.9): honest — no traffic or nothing matches the filter -->
      <div class="state-card" role="status">
        <strong>No active connections</strong>
        <p>
          {search || protocol !== 'all'
            ? 'Nothing matches the current filters.'
            : 'No connections are active right now.'}
        </p>
      </div>
    {:else}
      <div
        class="viewport"
        onscroll={onScrollUpdate}
        bind:clientHeight={viewportH}
        role="region"
        aria-label="Live connection table"
      >
        <Table>
          <caption class="sr-only">Live network connections</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">App</TableHead>
              <TableHead scope="col" class="col-pid">PID</TableHead>
              <TableHead scope="col">Proto</TableHead>
              <TableHead scope="col">Remote</TableHead>
              <TableHead scope="col" numeric>Rate</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {#if winStart > 0}
              <tr class="spacer" aria-hidden="true" style="height:{winStart * ROW_H}px">
                <td colspan="5"></td>
              </tr>
            {/if}
            {#each windowed as row (row.id)}
              <tr class="live-row" in:fade={{ duration: 200 }} style="height:{ROW_H}px">
                <TableCell>
                  <span class="app-cell">
                    <span class="app-icon" aria-hidden="true">{appInitial(row)}</span>
                    <span class="app-name">
                      {row.app.name}
                      {#if row.process_name && row.process_name !== row.app.name}
                        <span class="app-process">{row.process_name}</span>
                      {/if}
                    </span>
                  </span>
                </TableCell>
                <TableCell mono class="col-pid">{row.pid}</TableCell>
                <TableCell mono class="proto-cell">{row.protocol.toUpperCase()}</TableCell>
                <TableCell mono>
                  <span class="remote-cell">
                    <span>{row.remote_ip}:{row.remote_port}</span>
                    {#if row.remote_host}
                      <span class="remote-host">{row.remote_host}</span>
                    {/if}
                  </span>
                </TableCell>
                <TableCell numeric>
                  <span class="rate-cell">
                    <span class="rate-bar-wrap" aria-hidden="true">
                      <span
                        class="rate-bar"
                        style="inline-size:{maxRate > 0
                          ? ((row.rate_up_bps + row.rate_down_bps) / maxRate) * 100
                          : 0}%"
                      ></span>
                    </span>
                    <span class="rate-text">{fmtRate(row.rate_up_bps + row.rate_down_bps)}</span>
                  </span>
                </TableCell>
              </tr>
            {/each}
            {#if filtered.length - winEnd > 0}
              <tr
                class="spacer"
                aria-hidden="true"
                style="height:{(filtered.length - winEnd) * ROW_H}px"
              >
                <td colspan="5"></td>
              </tr>
            {/if}
          </TableBody>
        </Table>
      </div>
    {/if}
  {/if}
</section>

<style>
  .live {
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
  .filter-protocol {
    inline-size: 140px;
    padding: var(--nwp-space-2);
    font: var(--nwp-size-body) var(--nwp-font-ui);
    color: var(--nwp-text);
    background: var(--nwp-surface);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
  }
  .live-total {
    margin-inline-start: auto;
    font: 500 var(--nwp-size-body) var(--nwp-font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--nwp-text-secondary);
  }

  .frozen-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--nwp-space-2) var(--nwp-space-3);
    font-size: var(--nwp-size-body);
    color: var(--nwp-warning);
    background: var(--nwp-warning-soft);
    border-radius: var(--nwp-radius-md);
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
     via :global under the parent-scoped row. */
  .live-row :global(.nwp-table-cell) {
    block-size: 100%;
    padding-block: 0;
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

  .live :global(.proto-cell) {
    color: var(--nwp-text-secondary);
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

  .rate-cell {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--nwp-space-2);
  }
  .rate-bar-wrap {
    display: inline-block;
    inline-size: 80px;
    block-size: 8px;
    background: var(--nwp-surface-2);
    border-radius: var(--nwp-radius-full);
    overflow: hidden;
  }
  .rate-bar {
    display: inline-block;
    block-size: 100%;
    background: var(--nwp-primary);
  }
  .rate-text {
    min-inline-size: 84px;
    text-align: end;
    font-variant-numeric: tabular-nums;
  }

  .sr-only {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Compact breakpoint hides PID (DESIGN.md §8.4). `.col-pid` is applied
     inside the vendored TableHead/TableCell components, hence :global. */
  @media (max-width: 720px) {
    .live :global(.col-pid) {
      display: none;
    }
  }
</style>
