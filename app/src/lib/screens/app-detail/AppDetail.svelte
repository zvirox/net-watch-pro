<!-- App Detail screen — DESIGN.md §8.5 (content-only; app shell owns chrome). -->
<script lang="ts">
  import type { EChartsCoreOption } from 'echarts/core';
  import { Chart } from '$lib/components/charts';
  import {
    Button,
    EmptyState,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
  } from '$lib/components/ui';
  import { appDetailStore, type Granularity } from '$lib/stores/app-detail.svelte';
  import { formatBytes } from '$lib/utils/format';

  type Tab = 'overview' | 'connections' | 'endpoints';

  let { appId }: { appId: number } = $props();

  const tabs: Array<{ id: Tab; label: string }> = [
    { id: 'overview', label: 'Overview' },
    { id: 'connections', label: 'Connections' },
    { id: 'endpoints', label: 'Endpoints' },
  ];

  let activeTab = $state<Tab>('overview');
  let iconFailed = $state(false);

  $effect(() => {
    void appDetailStore.load(appId);
  });

  const detail = $derived(appDetailStore.detail);
  const appName = $derived(
    detail === null ? '' : detail.name === 'unknown' ? 'Unknown process' : detail.name,
  );
  const timeline = $derived(appDetailStore.timeline);
  const endpoints = $derived(appDetailStore.endpoints);
  const connections = $derived(appDetailStore.connections);
  const loading = $derived(appDetailStore.loading);
  const error = $derived(appDetailStore.error);

  function labelFor(bucket: number, granularity: Granularity): string {
    const date = new Date(bucket * 1000);
    return granularity === 'day'
      ? date.toLocaleDateString([], { month: 'short', day: 'numeric' })
      : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function formatAxis(value: number): string {
    return formatBytes(value);
  }

  function dateFor(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleDateString([], {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  const timelineOption: EChartsCoreOption = $derived({
    tooltip: { trigger: 'axis' },
    legend: { data: ['Download', 'Upload'] },
    xAxis: {
      type: 'category',
      data: timeline.map((point) => labelFor(point.bucket_start, appDetailStore.granularity)),
    },
    yAxis: { type: 'value', axisLabel: { formatter: formatAxis } },
    series: [
      { name: 'Download', type: 'line', data: timeline.map((point) => point.down), areaStyle: {} },
      { name: 'Upload', type: 'line', data: timeline.map((point) => point.up), areaStyle: {} },
    ],
  });

  function onTabKeydown(event: KeyboardEvent, index: number): void {
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = tabs.length - 1;
    else return;
    event.preventDefault();
    activeTab = tabs[next]?.id ?? 'overview';
    const buttons = (event.currentTarget as HTMLElement).parentElement?.querySelectorAll('button');
    buttons?.[next]?.focus();
  }
</script>

<section class="app-detail" aria-label="App detail">
  {#if error}
    <div class="error-banner" role="alert">
      <span>Could not load app detail. {error}</span>
      <Button
        variant="ghost"
        onclick={() => {
          appDetailStore.retry();
        }}>Retry</Button
      >
    </div>
  {/if}

  {#if loading && !detail}
    <div class="state-card" role="status">Loading app detail…</div>
  {:else if !detail}
    <EmptyState
      title="App unavailable"
      description="Connect to the daemon to inspect application usage."
      icon="grid"
    />
  {:else}
    <header class="app-header">
      <a class="back" href="#apps" aria-label="Back to apps">‹ Apps</a>
      <div class="identity">
        <span class="app-icon" aria-hidden="true">
          {#if detail.icon && !iconFailed}
            <img
              src="/api/v1/apps/{String(detail.id)}/icon"
              alt=""
              onerror={() => {
                iconFailed = true;
              }}
            />
          {:else}
            {appName.charAt(0).toUpperCase()}
          {/if}
        </span>
        <div>
          <h2>{appName}</h2>
          <p class="meta">
            {#if detail.attribution_confidence !== 'exact'}
              <span class="confidence {detail.attribution_confidence}"
                >{detail.attribution_confidence}</span
              >
            {/if}
            {#if detail.deleted_at !== null}
              <span class="deleted"
                >No longer active — last seen {dateFor(detail.last_seen_at)}</span
              >
            {/if}
          </p>
        </div>
      </div>
      <div class="headline">
        <span>TODAY</span>
        <strong>{formatBytes(detail.usage.today.up + detail.usage.today.down)}</strong>
      </div>
    </header>

    <div class="tabs" role="tablist" aria-label="App detail sections">
      {#each tabs as tab, index (tab.id)}
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === tab.id}
          tabindex={activeTab === tab.id ? 0 : -1}
          class:active={activeTab === tab.id}
          onclick={() => (activeTab = tab.id)}
          onkeydown={(event) => {
            onTabKeydown(event, index);
          }}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div role="tabpanel" aria-label={tabs.find((tab) => tab.id === activeTab)?.label}>
      {#if activeTab === 'overview'}
        <div class="overview">
          <div class="stat-row">
            <article class="stat-card">
              <span>FIRST SEEN</span>
              <strong>{dateFor(detail.first_seen_at)}</strong>
            </article>
            <article class="stat-card">
              <span>LAST ACTIVE</span>
              <strong>{dateFor(detail.last_seen_at)}</strong>
            </article>
            <article class="stat-card">
              <span>MONTH</span>
              <strong>{formatBytes(detail.usage.month.up + detail.usage.month.down)}</strong>
            </article>
          </div>
          <div class="panel">
            <div class="panel-heading">
              <h3>Usage timeline</h3>
              <div class="granularity" role="group" aria-label="Timeline granularity">
                {#each ['day', 'hour'] as option (option)}
                  <button
                    type="button"
                    class:active={appDetailStore.granularity === option}
                    aria-pressed={appDetailStore.granularity === option}
                    onclick={() => {
                      appDetailStore.setGranularity(option as Granularity);
                    }}
                  >
                    {option === 'day' ? 'Day' : 'Hour'}
                  </button>
                {/each}
              </div>
            </div>
            {#if timeline.length > 0}
              <div class="chart-frame">
                <Chart
                  ariaLabel="Usage for {appName} by {appDetailStore.granularity}"
                  option={timelineOption}
                />
              </div>
              <table class="sr-only">
                <caption>App usage timeline values</caption>
                <thead><tr><th>Bucket</th><th>Download</th><th>Upload</th></tr></thead>
                <tbody>
                  {#each timeline as point (point.bucket_start)}
                    <tr>
                      <td>{labelFor(point.bucket_start, appDetailStore.granularity)}</td>
                      <td>{formatBytes(point.down)}</td>
                      <td>{formatBytes(point.up)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <EmptyState
                title="No usage in this period"
                description="This app has no recorded traffic for the selected range."
                icon="activity"
              />
            {/if}
          </div>
        </div>
      {:else if activeTab === 'connections'}
        <div class="panel">
          {#if connections.length > 0}
            <Table>
              <caption class="sr-only">Connections for {appName}</caption>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">Time</TableHead>
                  <TableHead scope="col">Remote</TableHead>
                  <TableHead scope="col">Protocol</TableHead>
                  <TableHead scope="col" numeric>▲ Up</TableHead>
                  <TableHead scope="col" numeric>▼ Down</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {#each connections as row (row.id ?? `${String(row.first_seen_at)}-${row.remote_ip}`)}
                  <TableRow>
                    <TableCell mono>{dateFor(row.last_seen_at)}</TableCell>
                    <TableCell mono>{row.remote_ip}:{row.remote_port}</TableCell>
                    <TableCell>{row.protocol}</TableCell>
                    <TableCell mono numeric>{formatBytes(row.bytes_up)}</TableCell>
                    <TableCell mono numeric>{formatBytes(row.bytes_down)}</TableCell>
                  </TableRow>
                {/each}
              </TableBody>
            </Table>
          {:else}
            <EmptyState
              title="No connections yet"
              description="Episodes for this app will appear here once traffic is recorded."
              icon="activity"
            />
          {/if}
        </div>
      {:else}
        <div class="panel">
          {#if endpoints.length > 0}
            <Table>
              <caption class="sr-only">Top endpoints for {appName}</caption>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">Remote host / IP</TableHead>
                  <TableHead scope="col">Port</TableHead>
                  <TableHead scope="col" numeric>Connections</TableHead>
                  <TableHead scope="col" numeric>Bytes</TableHead>
                  <TableHead scope="col">Last seen</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {#each endpoints as endpoint (endpoint.remote_ip + ':' + String(endpoint.port))}
                  <TableRow>
                    <TableCell mono>
                      <span>{endpoint.remote_host ?? endpoint.remote_ip}</span>
                      {#if endpoint.remote_host}
                        <span class="endpoint-ip">{endpoint.remote_ip}</span>
                      {/if}
                    </TableCell>
                    <TableCell mono>{endpoint.port}</TableCell>
                    <TableCell mono numeric>{endpoint.connections}</TableCell>
                    <TableCell mono numeric>{formatBytes(endpoint.bytes_total)}</TableCell>
                    <TableCell mono>{dateFor(endpoint.last_seen_at)}</TableCell>
                  </TableRow>
                {/each}
              </TableBody>
            </Table>
          {:else}
            <EmptyState
              title="No endpoints yet"
              description="Remote hosts contacted by this app will be ranked here."
              icon="grid"
            />
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .app-detail {
    display: grid;
    gap: var(--nwp-space-4);
  }
  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    border: 1px solid var(--nwp-error);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-error-soft);
    padding: var(--nwp-space-3) var(--nwp-space-4);
    font-size: var(--nwp-size-label);
  }
  .state-card {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
    padding: var(--nwp-space-5);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .app-header {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-4);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
    padding: var(--nwp-space-5);
  }
  .back {
    color: var(--nwp-primary);
    font-size: var(--nwp-size-label);
    text-decoration: none;
  }
  .back:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .identity {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--nwp-space-3);
    min-width: 0;
  }
  .app-icon {
    display: grid;
    width: var(--nwp-space-8);
    height: var(--nwp-space-8);
    flex: 0 0 var(--nwp-space-8);
    place-items: center;
    overflow: hidden;
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-surface-2);
    color: var(--nwp-primary);
    font-weight: 600;
  }
  .app-icon img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  h2 {
    margin: 0;
    font-size: var(--nwp-size-title-2);
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--nwp-space-2);
    margin: var(--nwp-space-1) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .confidence {
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-primary-soft);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-primary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .confidence.unknown {
    background: var(--nwp-error-soft);
    color: var(--nwp-error);
  }
  .deleted {
    color: var(--nwp-warning);
  }
  .headline {
    display: grid;
    justify-items: end;
    gap: var(--nwp-space-1);
  }
  .headline span {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .headline strong {
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-data-lg);
    font-variant-numeric: tabular-nums;
  }
  .tabs {
    display: flex;
    gap: var(--nwp-space-4);
    border-bottom: 1px solid var(--nwp-border);
  }
  .tabs button {
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    padding: var(--nwp-space-2) 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-body);
    cursor: pointer;
    transition:
      color var(--nwp-dur-fast) var(--nwp-ease-standard),
      border-color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .tabs button:hover:not(.active) {
    color: var(--nwp-text);
  }
  .tabs button.active {
    border-bottom-color: var(--nwp-primary);
    color: var(--nwp-text);
    font-weight: 600;
  }
  .tabs button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .overview {
    display: grid;
    gap: var(--nwp-space-4);
  }
  .stat-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--nwp-space-4);
  }
  .stat-card,
  .panel {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
    padding: var(--nwp-space-5);
  }
  .stat-card {
    display: grid;
    gap: var(--nwp-space-2);
  }
  .stat-card span {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .stat-card strong {
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-body);
    font-variant-numeric: tabular-nums;
  }
  .panel-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--nwp-space-4);
  }
  h3 {
    margin: 0;
    font-size: var(--nwp-size-title-3);
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
    padding: var(--nwp-space-1) var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
    cursor: pointer;
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
  .chart-frame {
    height: 240px;
  }
  .endpoint-ip {
    display: block;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }
  @media (max-width: 900px) {
    .stat-row {
      grid-template-columns: 1fr;
    }
  }
</style>
