<!-- Apps list — DESIGN.md §8.10 (Phase D D4). Content-only; app shell owns chrome. -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    EmptyState,
    Input,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
  } from '$lib/components/ui';
  import { appsStore } from '$lib/stores/apps.svelte';
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { liveAppsStore } from '$lib/stores/live-apps.svelte';
  import { rangeStore } from '$lib/stores/range.svelte';
  import { usageStore } from '$lib/stores/usage.svelte';
  import { formatBytes } from '$lib/utils/format';

  onMount(() => {
    liveAppsStore.start();
    void appsStore.load();
  });
  onDestroy(() => {
    liveAppsStore.stop();
  });

  const rows = $derived(appsStore.rows);
  const live = $derived(liveAppsStore.rows);
  const noCollector = $derived(daemonStore.collectorMissing);
  const loading = $derived(appsStore.loading);
  const error = $derived(appsStore.error);

  const filtered = $derived.by(() => {
    const needle = appsStore.search.trim().toLowerCase();
    const result = rows.filter((row) => row.name.toLowerCase().includes(needle));
    if (appsStore.sort === 'name') {
      return [...result].sort((a, b) => a.name.localeCompare(b.name));
    }
    if (appsStore.sort === 'speed') {
      const rate = (name: string): number =>
        live
          .filter((row) => row.name.toLowerCase() === name.toLowerCase())
          .reduce((sum, row) => sum + row.down_bps + row.up_bps, 0);
      return [...result].sort((a, b) => rate(b.name) - rate(a.name));
    }
    return result; // the API already sorted by usage_range desc
  });

  const liveFor = (name: string) =>
    live.find((row) => row.name.toLowerCase() === name.toLowerCase());

  function rate(value: number | undefined): string {
    if (value === undefined) return '—';
    if (value <= 0) return '0 B/s';
    const units = ['KB/s', 'MB/s', 'GB/s'];
    let amount = value / 1024;
    let unit = 0;
    while (amount >= 1024 && unit < units.length - 1) {
      amount /= 1024;
      unit += 1;
    }
    return `${amount >= 100 ? amount.toFixed(0) : amount.toFixed(1)} ${units[unit] ?? 'B/s'}`;
  }

  function onRangeKeydown(event: KeyboardEvent): void {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const presets = rangeStore.presets;
    const index = presets.indexOf(rangeStore.preset);
    let next = index;
    if (event.key === 'ArrowLeft') next = (index - 1 + presets.length) % presets.length;
    if (event.key === 'ArrowRight') next = (index + 1) % presets.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = presets.length - 1;
    const value = presets[next];
    if (value) void rangeStore.set(value);
  }

  function onSearchInput(event: Event): void {
    appsStore.setSearch((event.currentTarget as HTMLInputElement).value);
  }
</script>

<section class="apps" aria-label="Apps usage list">
  <div class="range-bar">
    <div class="granularity" role="group" aria-label="Usage range">
      {#each rangeStore.presets as option (option)}
        <button
          type="button"
          class:active={rangeStore.preset === option}
          aria-pressed={rangeStore.preset === option}
          onclick={() => void rangeStore.set(option)}
          onkeydown={onRangeKeydown}
        >
          {rangeStore.labelFor(option)}
        </button>
      {/each}
    </div>
    <span class="range-meta">{rangeStore.bucketLabel} buckets · local time</span>
  </div>

  <div class="filter-row">
    <Input
      class="filter-search"
      search
      placeholder="Search apps…"
      aria-label="Search apps"
      value={appsStore.search}
      oninput={onSearchInput}
    />
    <div class="granularity" role="group" aria-label="Sort apps">
      {#each [['speed', 'Speed'], ['total', 'Total'], ['name', 'Name']] as [value, label] (value)}
        <button
          type="button"
          class:active={appsStore.sort === value}
          aria-pressed={appsStore.sort === value}
          onclick={() => {
            appsStore.setSort(value as 'speed' | 'total' | 'name');
          }}
        >
          {label}
        </button>
      {/each}
    </div>
  </div>

  {#if noCollector}
    <div class="state-card" role="status">
      Live per-app speeds need an eBPF collector; PC speed and recorded totals still work.
    </div>
  {/if}

  {#if error}
    <div class="state-card state-card--error" role="alert">
      <span>Could not load apps. {error}</span>
      <button
        type="button"
        onclick={() => {
          void appsStore.load();
        }}>Retry</button
      >
    </div>
  {:else if loading && rows.length === 0}
    <div class="state-card" role="status">Loading apps…</div>
  {:else if filtered.length === 0}
    <EmptyState
      title="No attributed apps yet"
      description="App usage will appear here after monitoring begins."
      icon="grid"
    />
  {:else}
    <div class="viewport" role="region" aria-label="Apps table">
      <Table>
        <caption class="sr-only">Per-app usage for {rangeStore.label}</caption>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">App</TableHead>
            <TableHead scope="col" numeric>Live ↓</TableHead>
            <TableHead scope="col" numeric>Live ↑</TableHead>
            <TableHead scope="col" numeric>Total · {rangeStore.label}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {#each filtered as row (row.id)}
            {@const liveRow = liveFor(row.name)}
            <TableRow
              role="link"
              tabindex={0}
              aria-label="Open details for {row.name}"
              onclick={() => {
                window.location.hash = `apps/${String(row.id)}`;
              }}
              onkeydown={(event: KeyboardEvent) => {
                if (event.key !== 'Enter' && event.key !== ' ') return;
                event.preventDefault();
                window.location.hash = `apps/${String(row.id)}`;
              }}
            >
              <TableCell>
                <span class="app-name">
                  {row.name === 'unknown' ? 'Unknown process' : row.name}
                  {#if row.attribution_confidence === 'low'}
                    <span class="confidence">low confidence</span>
                  {/if}
                </span>
              </TableCell>
              <TableCell numeric>{rate(liveRow?.down_bps)}</TableCell>
              <TableCell numeric>{rate(liveRow?.up_bps)}</TableCell>
              <TableCell numeric>{formatBytes(row.usage.range.up + row.usage.range.down)}</TableCell
              >
            </TableRow>
          {/each}
          <TableRow class="unattributed-row">
            <TableCell>Unattributed / system</TableCell>
            <TableCell numeric>{rate(liveAppsStore.data?.unattributed.down_bps)}</TableCell>
            <TableCell numeric>{rate(liveAppsStore.data?.unattributed.up_bps)}</TableCell>
            <TableCell numeric
              >{formatBytes(usageStore.summary?.range.totals.unattributed ?? 0)}</TableCell
            >
          </TableRow>
        </TableBody>
      </Table>
    </div>
  {/if}
</section>

<style>
  .apps {
    display: grid;
    gap: var(--nwp-space-4);
  }
  .range-bar,
  .filter-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
  }
  .filter-row :global(.filter-search) {
    max-inline-size: 340px;
    flex: 1 1 240px;
  }
  .range-meta {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .granularity {
    display: flex;
    gap: var(--nwp-space-1);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface-2);
    padding: 2px;
  }
  .granularity button {
    border: 0;
    border-radius: var(--nwp-radius-sm);
    background: transparent;
    padding: var(--nwp-space-2) var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
    cursor: pointer;
    transition:
      background-color var(--nwp-dur-fast) var(--nwp-ease-standard),
      color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .granularity button.active {
    background: var(--nwp-surface);
    color: var(--nwp-primary);
    font-weight: 600;
  }
  .granularity button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .state-card {
    border: 1px solid var(--nwp-warning);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-warning-soft);
    padding: var(--nwp-space-3) var(--nwp-space-4);
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
  }
  .state-card--error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    border-color: var(--nwp-error);
    background: var(--nwp-error-soft);
  }
  .state-card--error button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .state-card--error button {
    border: 0;
    background: transparent;
    color: var(--nwp-error);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .viewport {
    overflow-x: auto;
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
  }
  .app-name {
    display: inline-flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  .confidence {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .viewport :global(.unattributed-row) {
    background: var(--nwp-primary-soft);
    color: var(--nwp-text-secondary);
  }
  .sr-only {
    position: absolute;
    display: block;
    width: 1px;
    height: 1px;
    max-width: 1px;
    max-height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }
  @media (max-width: 1100px) {
    .range-bar,
    .filter-row {
      flex-wrap: wrap;
    }
  }
</style>
