<script lang="ts">
  // Settings screen — DESIGN.md §8.7 sections as underline tabs (§7.13).
  // Budgets landed in P6.9; the remaining general/data/daemon/license/about
  // sections are wired to the typed T1/T2 settings contract (API.md §2.7).
  import { onDestroy, onMount } from 'svelte';
  import { Button, Input, Switch } from '$lib/components/ui';
  import { budgetsStore, type BudgetLevel, type BudgetState } from '$lib/stores/budgets.svelte';
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { interfacesStore } from '$lib/stores/interfaces.svelte';
  import { licenseStore } from '$lib/stores/license.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { settingsUIStore, type ThemePreference } from '$lib/stores/settings-ui.svelte';
  import { formatBytes } from '$lib/utils/format';

  const sections = [
    'General',
    'Data',
    'Budgets',
    'Interfaces',
    'Daemon',
    'License',
    'About',
  ] as const;
  type Section = (typeof sections)[number];

  const themes: Array<{ value: ThemePreference; label: string }> = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];

  const meteredPolicies: Array<{ value: string; label: string }> = [
    { value: 'ask', label: 'Ask' },
    { value: 'label', label: 'Label' },
    { value: 'auto', label: 'Auto' },
  ];

  const dataFields = [
    { key: 'retention_days', label: 'Retention (days)' },
    { key: 'raw_window_days', label: 'Raw connection window (days)' },
    { key: 'billing_cycle_day', label: 'Billing cycle day' },
  ];

  const budgetFields = [
    { key: 'budget_total_daily_mb', label: 'Daily total budget (MB)' },
    { key: 'budget_total_monthly_mb', label: 'Monthly total budget (MB)' },
    { key: 'budget_per_app_daily_mb', label: 'Per-app daily budget (MB)' },
    { key: 'budget_per_app_monthly_mb', label: 'Per-app monthly budget (MB)' },
    { key: 'budget_warn_pct', label: 'Warn at (%)' },
  ];

  type BudgetBar = { label: string; budget: BudgetState };
  type AppAlert = {
    app_id: number;
    name: string;
    level: BudgetLevel;
    scope: string;
    used_bytes: number;
    limit_bytes: number;
  };

  let active = $state<Section>('General');
  let savedKey = $state<string | null>(null);
  let version = $state('—');
  let tablist: HTMLDivElement | undefined = $state();

  const alerts = $derived<AppAlert[]>(
    budgetsStore.data === null
      ? []
      : [
          ...budgetsStore.data.per_app_daily
            .filter((app) => app.level !== 'ok')
            .map((app) => ({ ...app, scope: 'daily' })),
          ...budgetsStore.data.per_app_monthly
            .filter((app) => app.level !== 'ok')
            .map((app) => ({ ...app, scope: 'monthly' })),
        ],
  );
  const bars = $derived<BudgetBar[]>(
    budgetsStore.data === null
      ? []
      : [
          { label: 'Daily total', budget: budgetsStore.data.total_daily },
          { label: 'Monthly total', budget: budgetsStore.data.total_monthly },
        ],
  );

  onMount(() => {
    void settingsStore.load();
    void interfacesStore.load();
    budgetsStore.start();
    void daemonStore
      .request('GET', '/api/v1/health')
      .then((health) => {
        version = (health as { version?: string }).version ?? '—';
      })
      .catch(() => undefined);
  });
  onDestroy(() => {
    budgetsStore.stop();
  });

  async function save(key: string, value: string | number | boolean): Promise<void> {
    try {
      await settingsStore.setValue(key, value);
      savedKey = key;
      window.setTimeout(() => {
        if (savedKey === key) savedKey = null;
      }, 2500);
      if (key.startsWith('budget_')) await budgetsStore.refresh();
    } catch {
      // The store keeps the server's message for rendering.
    }
  }

  function numberValue(event: Event): number | null {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    return Number.isFinite(value) ? value : null;
  }

  function onTabKeydown(event: KeyboardEvent): void {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const tabs = Array.from(tablist?.querySelectorAll<HTMLButtonElement>('[role="tab"]') ?? []);
    const current = tabs.findIndex((tab) => tab.getAttribute('aria-selected') === 'true');
    let next = current;
    if (event.key === 'ArrowRight') next = (current + 1) % tabs.length;
    if (event.key === 'ArrowLeft') next = (current - 1 + tabs.length) % tabs.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = tabs.length - 1;
    event.preventDefault();
    active = sections[next] ?? 'General';
    tabs[next]?.focus();
  }
</script>

<section class="settings" aria-label="Settings">
  <div
    class="tabs"
    role="tablist"
    aria-label="Settings sections"
    tabindex="-1"
    bind:this={tablist}
    onkeydown={onTabKeydown}
  >
    {#each sections as section (section)}
      <button
        type="button"
        role="tab"
        class="tab"
        class:tab--active={active === section}
        aria-selected={active === section}
        tabindex={active === section ? 0 : -1}
        onclick={() => {
          active = section;
        }}
      >
        {section}
      </button>
    {/each}
  </div>

  {#if savedKey !== null}<p class="saved" role="status">Saved</p>{/if}
  {#if settingsStore.loading}
    <p class="muted" role="status">Reading settings…</p>
  {:else if settingsStore.error !== null}
    <div class="state-error" role="alert">
      <strong>Settings could not be loaded</strong>
      <p>{settingsStore.error}</p>
    </div>
  {:else}
    <div class="panel" role="tabpanel" aria-label={`${active} settings`}>
      {#if active === 'General'}
        <div class="card">
          <header class="card__head"><h2>Appearance</h2></header>
          <p class="field-label">Theme</p>
          <div class="segmented" role="group" aria-label="Theme">
            {#each themes as theme (theme.value)}
              <button
                type="button"
                aria-pressed={settingsUIStore.theme === theme.value}
                class:segment--active={settingsUIStore.theme === theme.value}
                onclick={() => {
                  void settingsUIStore.setTheme(theme.value);
                }}
              >
                {theme.label}
              </button>
            {/each}
          </div>
          <p class="hint">System follows your desktop preference. No fonts are fetched online.</p>
        </div>
      {:else if active === 'Data'}
        <div class="card">
          <header class="card__head"><h2>Data &amp; history</h2></header>
          <div class="fields">
            {#each dataFields as field (field.key)}
              {@const entry = settingsStore.get(field.key)}
              {#if entry}
                <Input
                  type="number"
                  min={entry.min}
                  max={entry.max}
                  label={field.label}
                  helper={entry.help}
                  value={String(entry.value)}
                  onchange={(event: Event) => {
                    const value = numberValue(event);
                    if (value !== null) void save(field.key, value);
                  }}
                />
              {/if}
            {/each}
            <Input
              label="Timezone"
              placeholder="UTC"
              helper="IANA name (e.g. Europe/Berlin); drives day and billing boundaries."
              value={String(settingsStore.value('timezone') ?? '')}
              onchange={(event: Event) => {
                const value = (event.currentTarget as HTMLInputElement).value;
                void save('timezone', value);
              }}
            />
          </div>
          <div class="danger">
            <div>
              <strong>Clear local data</strong>
              <p>
                Deletes this device's recorded history. Not available until the data-management API
                lands.
              </p>
            </div>
            <Button variant="destructive" size="sm" disabled>Clear data</Button>
          </div>
        </div>
      {:else if active === 'Budgets'}
        <div class="card">
          <header class="card__head">
            <div>
              <h2>Budgets &amp; alerts</h2>
              <p>Desktop notification when usage approaches or exceeds a cap. Free tier.</p>
            </div>
          </header>
          <div class="fields">
            {#each budgetFields as field (field.key)}
              {@const entry = settingsStore.get(field.key)}
              {#if entry}
                <Input
                  type="number"
                  min={entry.min}
                  max={entry.max}
                  label={field.label}
                  helper={entry.help}
                  value={String(entry.value)}
                  onchange={(event: Event) => {
                    const value = numberValue(event);
                    if (value !== null) void save(field.key, value);
                  }}
                />
              {/if}
            {/each}
          </div>
          <p class="hint">
            0 disables a budget. Counts use metered interfaces when any are marked metered.
          </p>
        </div>

        {#if budgetsStore.data}
          <div class="card">
            <header class="card__head"><h2>Current usage</h2></header>
            {#each bars as bar (bar.label)}
              <div class="bar-block">
                <div class="bar-head">
                  <span>{bar.label}</span>
                  <span class="mono"
                    >{bar.budget.limit_bytes > 0
                      ? `${String(budgetsStore.percent(bar.budget))}%`
                      : 'no budget'}</span
                  >
                </div>
                <div
                  class="bar"
                  role="progressbar"
                  aria-label={`${bar.label} budget used`}
                  aria-valuemin="0"
                  aria-valuemax="100"
                  aria-valuenow={Math.min(100, budgetsStore.percent(bar.budget))}
                >
                  <div
                    class="bar__fill"
                    class:bar__fill--warning={bar.budget.level === 'warning'}
                    class:bar__fill--breach={bar.budget.level === 'breach'}
                    style="width:{Math.min(100, budgetsStore.percent(bar.budget))}%"
                  ></div>
                </div>
                <p class="bar-caption">
                  {formatBytes(bar.budget.used_bytes)} used{#if bar.budget.limit_bytes > 0}
                    of {formatBytes(bar.budget.limit_bytes)}{/if}
                  <span class="bar-level">{bar.budget.level}</span>
                </p>
              </div>
            {/each}
            {#if alerts.length > 0}
              <ul class="alerts">
                {#each alerts as alert (`${String(alert.app_id)}-${alert.scope}`)}
                  <li>
                    <strong>{alert.name}</strong> · {alert.scope}
                    {alert.level} · {String(budgetsStore.percent(alert))}%
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      {:else if active === 'Interfaces'}
        <div class="card">
          <header class="card__head">
            <div>
              <h2>Metered policy</h2>
              <p>How a newly discovered interface is labeled.</p>
            </div>
          </header>
          <div class="segmented" role="group" aria-label="Metered policy">
            {#each meteredPolicies as policy (policy.value)}
              <button
                type="button"
                aria-pressed={settingsStore.value('metered_policy') === policy.value}
                class:segment--active={settingsStore.value('metered_policy') === policy.value}
                onclick={() => {
                  void save('metered_policy', policy.value);
                }}
              >
                {policy.label}
              </button>
            {/each}
          </div>
          <p class="hint">
            Ask: label manually. Label: mark new links metered. Auto needs NetworkManager detection,
            not in MVP — it behaves like Ask.
          </p>
        </div>
        <div class="card">
          <header class="card__head">
            <div>
              <h2>Interfaces</h2>
              <p>
                Mark metered links so budgets count the right traffic; excluded links leave headline
                totals.
              </p>
            </div>
          </header>
          {#if interfacesStore.loading}
            <p class="muted" role="status">Reading interfaces…</p>
          {:else if interfacesStore.error !== null}
            <div class="state-error" role="alert">
              <strong>Interfaces could not be loaded</strong>
              <p>{interfacesStore.error}</p>
            </div>
          {:else if interfacesStore.rows.length === 0}
            <p class="muted" role="status">
              No interfaces reported yet. They appear once the daemon is linked.
            </p>
          {:else}
            <div class="iface-list">
              {#each interfacesStore.rows as iface (iface.id)}
                <div class="iface-row">
                  <div class="iface-main">
                    <strong>{iface.name}</strong>
                    <span class="iface-meta"
                      >{iface.kind} · {formatBytes(iface.bytes.up)} ↑ / {formatBytes(
                        iface.bytes.down,
                      )} ↓</span
                    >
                  </div>
                  <div class="iface-switches">
                    <Switch
                      label="Metered"
                      checked={iface.is_metered}
                      onchange={(metered: boolean) => {
                        void interfacesStore.setFlag(iface.id, 'is_metered', metered);
                      }}
                    />
                    <Switch
                      label="Excluded"
                      checked={iface.is_excluded}
                      onchange={(excluded: boolean) => {
                        void interfacesStore.setFlag(iface.id, 'is_excluded', excluded);
                      }}
                    />
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if active === 'Daemon'}
        <div class="card">
          <header class="card__head"><h2>Daemon</h2></header>
          <Switch
            label="Start on boot"
            description="Synced to the systemd service by the daemon; disabling affects the next boot."
            checked={settingsStore.value('start_on_boot') === true}
            onchange={(checked: boolean) => {
              void save('start_on_boot', checked);
            }}
          />
          <div class="daemon-actions">
            <Button variant="ghost" size="sm" disabled>Open diagnostics</Button>
            <span class="hint">Diagnostics lands with P9.1.</span>
          </div>
        </div>
      {:else if active === 'License'}
        <div class="card">
          <header class="card__head">
            <div>
              <h2>License</h2>
              <p>Advanced filters, export and 365-day retention are Pro features.</p>
            </div>
            <span class="tier" class:tier--pro={licenseStore.isPro}
              >{licenseStore.isPro ? 'Pro' : 'Free'}</span
            >
          </header>
          <Button
            variant={licenseStore.isPro ? 'secondary' : 'primary'}
            size="sm"
            onclick={() => {
              licenseStore.openUpgrade();
            }}
          >
            {licenseStore.isPro ? 'Manage license' : 'Upgrade to Pro'}
          </Button>
        </div>
      {:else}
        <div class="card">
          <header class="card__head"><h2>About Net Watch Pro</h2></header>
          <dl class="about">
            <dt>Version</dt>
            <dd class="mono">{version}</dd>
            <dt>License</dt>
            <dd>AGPL-3.0-or-later / commercial</dd>
            <dt>Privacy</dt>
            <dd>No telemetry, no analytics, no update pings — all data stays here.</dd>
          </dl>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .settings {
    display: grid;
    gap: var(--nwp-space-4);
    max-inline-size: 760px;
  }
  .tabs {
    display: flex;
    gap: var(--nwp-space-1);
    border-bottom: 1px solid var(--nwp-border);
  }
  .tab {
    min-height: 36px;
    padding: 0 var(--nwp-space-3);
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-label);
    cursor: pointer;
  }
  .tab:hover {
    color: var(--nwp-text);
  }
  .tab--active {
    border-bottom-color: var(--nwp-primary);
    color: var(--nwp-text);
    font-weight: 600;
  }
  .tab:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: -2px;
  }
  .panel {
    display: grid;
    gap: var(--nwp-space-5);
  }
  .card {
    padding: var(--nwp-space-6);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
  }
  .card__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-5);
  }
  h2 {
    margin: 0;
    font-size: var(--nwp-size-title-2);
    font-weight: 600;
  }
  .card__head p {
    margin: var(--nwp-space-1) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-body);
  }
  .saved {
    margin: 0;
    color: var(--nwp-success);
    font-size: var(--nwp-size-label);
  }
  .field-label {
    margin: 0 0 var(--nwp-space-2);
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
    font-weight: 500;
  }
  .segmented {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface-2);
  }
  .segmented button {
    min-height: 28px;
    padding: 0 var(--nwp-space-3);
    border: 0;
    border-radius: var(--nwp-radius-sm);
    background: transparent;
    color: var(--nwp-text-secondary);
    font: inherit;
    font-size: var(--nwp-size-label);
    cursor: pointer;
  }
  .segmented .segment--active {
    background: var(--nwp-surface);
    color: var(--nwp-primary);
    font-weight: 600;
  }
  .segmented button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: -2px;
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--nwp-space-4);
  }
  .hint,
  .muted,
  .bar-caption {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .hint {
    margin: var(--nwp-space-4) 0 0;
  }
  .danger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-top: var(--nwp-space-6);
    padding-top: var(--nwp-space-4);
    border-top: 1px solid var(--nwp-border);
  }
  .danger p {
    margin: var(--nwp-space-1) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .state-error {
    padding: var(--nwp-space-4);
    border: 1px solid var(--nwp-error);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-error-soft);
  }
  .state-error strong {
    color: var(--nwp-error);
  }
  .state-error p {
    margin: var(--nwp-space-1) 0 0;
  }
  .iface-list {
    display: grid;
    gap: var(--nwp-space-2);
  }
  .iface-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    padding: var(--nwp-space-3);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
  }
  .iface-main {
    display: flex;
    flex-direction: column;
    gap: var(--nwp-space-1);
  }
  .iface-meta {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .iface-switches {
    display: flex;
    gap: var(--nwp-space-5);
  }
  .daemon-actions {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
    margin-top: var(--nwp-space-5);
  }
  .daemon-actions .hint {
    margin: 0;
  }
  .tier {
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-surface-2);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .tier--pro {
    background: var(--nwp-secondary-soft);
    color: var(--nwp-secondary);
    font-weight: 600;
  }
  .about {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: var(--nwp-space-2) var(--nwp-space-4);
    margin: 0;
    font-size: var(--nwp-size-body);
  }
  .about dt {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .about dd {
    margin: 0;
  }
  .bar-block {
    margin-bottom: var(--nwp-space-5);
  }
  .bar-head {
    display: flex;
    justify-content: space-between;
    margin-bottom: var(--nwp-space-2);
    font-size: var(--nwp-size-label);
  }
  .mono {
    font-family: var(--nwp-font-mono);
    font-variant-numeric: tabular-nums;
  }
  .bar {
    block-size: 8px;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface-2);
    overflow: hidden;
  }
  .bar__fill {
    block-size: 100%;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-primary);
  }
  .bar__fill--warning {
    background: var(--nwp-warning);
  }
  .bar__fill--breach {
    background: var(--nwp-error);
  }
  .bar-caption {
    display: flex;
    justify-content: space-between;
    margin: var(--nwp-space-2) 0 0;
  }
  .bar-level {
    font-family: var(--nwp-font-mono);
  }
  .alerts {
    margin: var(--nwp-space-4) 0 0;
    padding-inline-start: var(--nwp-space-5);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
</style>
