<!-- Dashboard screen — DESIGN.md §8.3 (content-only; app shell owns chrome). -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { EChartsCoreOption } from 'echarts/core';
  import { Chart } from '$lib/components/charts';
  import { Button, EmptyState } from '$lib/components/ui';
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { ratesStore } from '$lib/stores/rates.svelte';
  import { liveAppsStore } from '$lib/stores/live-apps.svelte';
  import { usageStore, type Granularity, type InterfaceUsage } from '$lib/stores/usage.svelte';
  import { formatBytes } from '$lib/utils/format';

  const rates = $derived(ratesStore.snapshot);
  const rateHistory = $derived(ratesStore.history);
  const rateStale = $derived(ratesStore.stale);
  const noCollector = $derived(daemonStore.collectorMissing);
  const liveApps = $derived(liveAppsStore.sorted());
  const liveAppsStale = $derived(liveAppsStore.stale);

  const today = $derived(usageStore.summary?.today ?? null);
  const month = $derived(usageStore.summary?.month ?? null);
  const topApps = $derived(usageStore.summary?.top_apps ?? []);
  const points = $derived(usageStore.timeline);
  const interfaces = $derived(usageStore.interfaces);
  const error = $derived(usageStore.error);
  const loading = $derived(usageStore.loading);
  const empty = $derived(usageStore.collecting);

  onMount(() => {
    void usageStore.load();
    ratesStore.start();
    liveAppsStore.start();
  });
  onDestroy(() => {
    ratesStore.stop();
    liveAppsStore.stop();
  });

  // Owner L-OQ4: Wi-Fi signal/link from local sysfs; "—" when unavailable.
  function wifiDetail(iface: InterfaceUsage): string | null {
    if (iface.kind !== 'wifi') return null;
    const signal = iface.signal_dbm === null ? '—' : `${String(iface.signal_dbm)} dBm`;
    const link = iface.link_mbps === null ? '—' : `${String(iface.link_mbps)} Mbps link`;
    return `${signal} · ${link}`;
  }

  function labelFor(bucket: number, granularity: Granularity): string {
    const date = new Date(bucket * 1000);
    return granularity === 'day'
      ? date.toLocaleDateString([], { month: 'short', day: 'numeric' })
      : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function formatAxis(value: number): string {
    return formatBytes(value);
  }

  function headline(value: number | undefined): string {
    return empty || value === undefined ? '—' : formatBytes(value);
  }

  const timelineOption: EChartsCoreOption = $derived({
    tooltip: { trigger: 'axis' },
    legend: { data: ['Download', 'Upload'] },
    xAxis: {
      type: 'category',
      data: points.map((point) => labelFor(point.bucket_start, usageStore.granularity)),
    },
    yAxis: {
      type: 'value',
      axisLabel: { formatter: formatAxis },
    },
    series: [
      {
        name: 'Download',
        type: 'line',
        data: points.map((point) => point.down),
        areaStyle: {},
      },
      {
        name: 'Upload',
        type: 'line',
        data: points.map((point) => point.up),
        areaStyle: {},
      },
    ],
  });

  const topAppsOption: EChartsCoreOption = $derived({
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { left: '4%', right: '8%', top: 8, bottom: 8 },
    xAxis: { type: 'value', axisLabel: { formatter: formatAxis } },
    yAxis: {
      type: 'category',
      data: topApps.map((app) => app.name),
      inverse: true,
    },
    series: [
      {
        name: 'Bytes',
        type: 'bar',
        data: topApps.map((app) => app.bytes_total),
      },
    ],
  });

  // Network now sparkline: 60 one-second samples, theme series, no labels.
  const sparkOption: EChartsCoreOption = $derived({
    animation: false,
    grid: { left: 0, right: 0, top: 2, bottom: 2 },
    xAxis: { type: 'category', show: false, data: rateHistory.map((_, index) => index) },
    yAxis: { type: 'value', show: false },
    series: [
      { name: 'Download', type: 'line', showSymbol: false, data: rateHistory.map((p) => p.down) },
      { name: 'Upload', type: 'line', showSymbol: false, data: rateHistory.map((p) => p.up) },
    ],
  });

  function onAppClick(params: unknown): void {
    if (typeof params !== 'object' || params === null || !('dataIndex' in params)) return;
    const index = params.dataIndex;
    if (typeof index !== 'number') return;
    const app = topApps[index];
    if (app) window.location.hash = `apps/${String(app.id)}`;
  }
</script>

<section class="dashboard" aria-label="Network usage dashboard">
  {#if error}
    <div class="error-banner" role="alert">
      <span>Could not load usage data. {error}</span>
      <Button
        variant="ghost"
        onclick={() => {
          usageStore.retry();
        }}>Retry</Button
      >
    </div>
  {/if}

  <article class="panel network-now" aria-label="Network now">
    <div class="network-now__head">
      <p class="eyebrow">NETWORK NOW</p>
      {#if rates === null}
        <span class="network-now__state" role="status">Loading…</span>
      {:else if rateStale}
        <span class="network-now__state network-now__state--stale" role="status">Stale</span>
      {:else}
        <span class="network-now__state network-now__state--live">Live</span>
      {/if}
    </div>
    <div class="network-now__body">
      <div class="network-now__numbers">
        <div>
          <span class="network-now__dir">▼ Download</span>
          <strong class="mono">{rates ? ratesStore.display(rates.down_bps) : '—'}</strong>
        </div>
        <div>
          <span class="network-now__dir">▲ Upload</span>
          <strong class="mono">{rates ? ratesStore.display(rates.up_bps) : '—'}</strong>
        </div>
      </div>
      <div class="network-now__spark" aria-hidden="true">
        {#if rateHistory.length > 0}
          <Chart option={sparkOption} ariaLabel="Last 60 seconds of download and upload speed" />
        {:else}
          <div class="sparkline-placeholder"></div>
        {/if}
      </div>
    </div>
    {#if noCollector}
      <p class="network-now__note">
        PC speed works without eBPF; per-app live rates need a collector.
      </p>
    {/if}
  </article>

  <div class="metrics" aria-label="Usage headline">
    <article class="metric-card">
      <p class="metric-label">DATA TODAY</p>
      <strong>{headline(today?.total)}</strong>
      <p class="metric-caption">Kernel interface counters</p>
    </article>
    <article class="metric-card">
      <p class="metric-label">THIS MONTH</p>
      <strong>{headline(month?.total)}</strong>
      <p class="metric-caption">Billing cycle total</p>
    </article>
    <article class="metric-card metric-card--gap">
      <p class="metric-label">
        UNATTRIBUTED
        <span
          class="gap-dot"
          title="Interface counter bytes minus attributed app bytes: headers, ACKs, retransmits, kernel/VPN traffic"
          aria-hidden="true"
        ></span>
      </p>
      <strong>{headline(month?.unattributed)}</strong>
      <p class="metric-caption">System traffic, shown honestly</p>
    </article>
  </div>

  <div class="lower-grid">
    <article class="panel timeline-panel">
      <div class="panel-heading">
        <div>
          <p class="eyebrow">NETWORK ACTIVITY</p>
          <h2>Usage timeline</h2>
        </div>
        <div class="granularity" role="group" aria-label="Timeline granularity">
          {#each ['day', 'hour'] as option (option)}
            <button
              type="button"
              class:active={usageStore.granularity === option}
              aria-pressed={usageStore.granularity === option}
              onclick={() => {
                usageStore.setGranularity(option as Granularity);
              }}
            >
              {option === 'day' ? 'Day' : 'Hour'}
            </button>
          {/each}
        </div>
      </div>

      {#if loading && points.length === 0}
        <div class="placeholder" role="status">Loading usage timeline…</div>
      {:else if empty}
        <EmptyState
          title="Waiting for first data"
          description="Once monitoring is enabled, your activity will appear here."
          icon="activity"
        />
      {:else if points.length > 0}
        <div class="chart-frame timeline-chart">
          <Chart
            ariaLabel="Usage timeline: download and upload bytes per {usageStore.granularity}"
            option={timelineOption}
          />
        </div>
        <table class="sr-only">
          <caption>Usage timeline values</caption>
          <thead><tr><th>Bucket</th><th>Download</th><th>Upload</th></tr></thead>
          <tbody>
            {#each points as point (point.bucket_start)}
              <tr>
                <td>{labelFor(point.bucket_start, usageStore.granularity)}</td>
                <td>{formatBytes(point.down)}</td>
                <td>{formatBytes(point.up)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <EmptyState
          title="No usage in this period"
          description="Try another day or hour range."
          icon="clock"
        />
      {/if}
    </article>

    <article class="panel apps-panel">
      <div class="panel-heading">
        <div>
          <p class="eyebrow">BY APPLICATION</p>
          <h2>Top apps</h2>
        </div>
      </div>
      {#if topApps.length > 0}
        <div class="chart-frame apps-chart">
          <Chart
            ariaLabel="Top applications by total bytes"
            option={topAppsOption}
            onEvents={{ click: onAppClick }}
          />
        </div>
        <table class="sr-only">
          <caption>Top applications by bytes</caption>
          <thead><tr><th>App</th><th>Bytes</th></tr></thead>
          <tbody>
            {#each topApps as app (app.id)}
              <tr><td>{app.name}</td><td>{formatBytes(app.bytes_total)}</td></tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <EmptyState
          title="No attributed apps yet"
          description="App usage will appear here after monitoring begins."
          icon="grid"
        />
      {/if}
      <!-- Phase C C7: live per-app speed from the `live.apps` stream. -->
      <div class="live-apps" aria-label="Live per-app speed">
        <div class="live-apps__head">
          <p class="eyebrow">LIVE SPEED</p>
          {#if liveAppsStale}
            <span class="network-now__state network-now__state--stale" role="status">Stale</span>
          {/if}
        </div>
        {#if noCollector}
          <p class="live-apps__note">
            Per-app live speed needs an eBPF collector; PC speed above still works.
          </p>
        {:else if liveApps.length === 0}
          <p class="live-apps__note">Waiting for the first per-app sample…</p>
        {:else}
          <ul>
            {#each liveApps as app (app.cgroup_id)}
              <li class:live-apps__idle={app.down_bps + app.up_bps === 0}>
                <span class="live-apps__name">
                  {app.name}
                  {#if app.confidence === 'low'}<span class="live-apps__confidence"
                      >low confidence</span
                    >{/if}
                </span>
                <span class="live-apps__rates mono">
                  <span>↓ {ratesStore.display(app.down_bps)}</span>
                  <span>↑ {ratesStore.display(app.up_bps)}</span>
                </span>
              </li>
            {/each}
          </ul>
          <p class="live-apps__note">
            Unattributed: ↓ {ratesStore.display(liveAppsStore.data?.unattributed.down_bps ?? 0)} · ↑ {ratesStore.display(
              liveAppsStore.data?.unattributed.up_bps ?? 0,
            )}
          </p>
        {/if}
      </div>
      <div class="gap-row">
        <span class="gap-dot" aria-hidden="true"></span>
        <span>Unattributed / system</span>
        <strong>{headline(month?.unattributed)}</strong>
      </div>
    </article>

    <article class="panel interfaces-panel">
      <div class="panel-heading">
        <div>
          <p class="eyebrow">INTERFACES</p>
          <h2>Where it went</h2>
        </div>
      </div>
      {#if interfaces.length > 0}
        <ul>
          {#each interfaces as iface (iface.id)}
            {@const detail = wifiDetail(iface)}
            <li>
              <div>
                <div class="iface-name">
                  <span>{iface.name}</span>
                  {#if iface.is_metered}<span class="metered">METERED</span>{/if}
                </div>
                {#if detail !== null}
                  <div class="iface-detail">{detail}</div>
                {/if}
              </div>
              <div class="iface-bytes">
                <span>↓ {formatBytes(iface.bytes.down)}</span>
                <span>↑ {formatBytes(iface.bytes.up)}</span>
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <EmptyState
          title="No interface samples yet"
          description="Counter samples will appear here once the daemon starts."
          icon="activity"
        />
      {/if}
    </article>
  </div>
</section>

<style>
  .dashboard {
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
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
  }
  .network-now {
    padding: var(--nwp-space-5);
  }
  .network-now__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--nwp-space-3);
  }
  .network-now__state {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .network-now__state--live {
    color: var(--nwp-success);
  }
  .network-now__state--stale {
    color: var(--nwp-warning);
  }
  .network-now__body {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-6);
  }
  .network-now__numbers {
    display: flex;
    gap: var(--nwp-space-8);
  }
  .network-now__dir {
    display: block;
    margin-bottom: var(--nwp-space-1);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .network-now__numbers strong {
    font-size: var(--nwp-size-data-lg);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .network-now__spark {
    width: 40%;
    min-width: 180px;
    height: 56px;
  }
  .sparkline-placeholder {
    width: 100%;
    height: 100%;
  }
  .network-now__note {
    margin: var(--nwp-space-3) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--nwp-space-4);
  }
  .metric-card,
  .panel {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
  }
  .metric-card {
    padding: var(--nwp-space-5);
  }
  .metric-card--gap {
    background: var(--nwp-primary-soft);
  }
  .metric-label {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
    margin: 0;
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .metric-caption {
    margin: var(--nwp-space-1) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .metric-card > strong {
    display: block;
    margin-top: var(--nwp-space-3);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-data-lg);
    font-variant-numeric: tabular-nums;
  }
  .gap-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-primary);
  }
  .lower-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.7fr) minmax(290px, 1fr);
    gap: var(--nwp-space-4);
  }
  .panel {
    min-height: 280px;
    padding: var(--nwp-space-5);
  }
  .interfaces-panel {
    grid-column: 1 / -1;
  }
  .panel-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--nwp-space-4);
  }
  .eyebrow {
    margin: 0 0 var(--nwp-space-1);
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  h2 {
    margin: 0;
    font-size: var(--nwp-size-title-3);
  }
  .placeholder {
    display: grid;
    min-height: 200px;
    place-content: center;
    text-align: center;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
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
  .granularity button:hover:not(.active) {
    color: var(--nwp-text);
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
  .timeline-chart {
    height: 220px;
  }
  .apps-chart {
    height: 180px;
  }
  .live-apps {
    margin-top: var(--nwp-space-4);
    border-top: 1px solid var(--nwp-border);
    padding-top: var(--nwp-space-3);
  }
  .live-apps__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--nwp-space-2);
  }
  .live-apps ul {
    display: grid;
    gap: var(--nwp-space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .live-apps li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-2);
    font-size: var(--nwp-size-label);
  }
  .live-apps__idle {
    /* §9.1: dim with a token, not opacity — dimmed text must stay legible. */
    color: var(--nwp-text-disabled);
  }
  .live-apps__name {
    display: inline-flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  .live-apps__confidence {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .live-apps__rates {
    display: inline-flex;
    gap: var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .live-apps__note {
    margin: var(--nwp-space-2) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .gap-row {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
    margin-top: var(--nwp-space-3);
    border-top: 1px solid var(--nwp-border);
    padding-top: var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .gap-row strong {
    margin-left: auto;
    color: var(--nwp-text);
    font-family: var(--nwp-font-mono);
    font-variant-numeric: tabular-nums;
  }
  .interfaces-panel ul {
    display: grid;
    gap: var(--nwp-space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .interfaces-panel li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--nwp-border);
    padding-bottom: var(--nwp-space-2);
    font-size: var(--nwp-size-label);
  }
  .iface-name {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  .iface-detail {
    margin-top: var(--nwp-space-1);
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    font-variant-numeric: tabular-nums;
  }
  .metered {
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-warning-soft);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-warning);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .iface-bytes {
    display: flex;
    gap: var(--nwp-space-4);
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-variant-numeric: tabular-nums;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }
  /* §5.4 compact: keep the live numbers readable before the columns stack. */
  @media (max-width: 1100px) {
    .network-now__body {
      flex-wrap: wrap;
      gap: var(--nwp-space-4);
    }
    .network-now__spark {
      width: 100%;
    }
  }
  @media (max-width: 900px) {
    .metrics {
      grid-template-columns: 1fr;
    }
    .lower-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
