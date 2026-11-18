<script lang="ts">
  // Settings — budgets/alerts section (DESIGN.md §8.7, PRD §6.1 #8, D6a).
  // Other sections land in P7.3; this slice owns the free-tier budget contract.
  import { onDestroy, onMount } from 'svelte';
  import { Input } from '$lib/components/ui';
  import { budgetsStore, type BudgetLevel, type BudgetState } from '$lib/stores/budgets.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { formatBytes } from '$lib/utils/format';

  type BudgetBar = { label: string; budget: BudgetState };
  type AppAlert = {
    app_id: number;
    name: string;
    level: BudgetLevel;
    scope: string;
    used_bytes: number;
    limit_bytes: number;
  };
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

  const fields = [
    { key: 'budget_total_daily_mb', label: 'Daily total budget (MB)' },
    { key: 'budget_total_monthly_mb', label: 'Monthly total budget (MB)' },
    { key: 'budget_per_app_daily_mb', label: 'Per-app daily budget (MB)' },
    { key: 'budget_per_app_monthly_mb', label: 'Per-app monthly budget (MB)' },
    { key: 'budget_warn_pct', label: 'Warn at (%)' },
  ];

  let savedKey = $state<string | null>(null);

  onMount(() => {
    void settingsStore.load();
    budgetsStore.start();
  });
  onDestroy(() => {
    budgetsStore.stop();
  });

  async function save(key: string, event: Event): Promise<void> {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    if (!Number.isFinite(value)) return;
    try {
      await settingsStore.setValue(key, value);
      savedKey = key;
      window.setTimeout(() => {
        if (savedKey === key) savedKey = null;
      }, 2500);
      await budgetsStore.refresh();
    } catch {
      // The store keeps the server's message for rendering.
    }
  }
</script>

<section class="settings" aria-label="Settings">
  <div class="card">
    <header class="card__head">
      <div>
        <h2>Budgets &amp; alerts</h2>
        <p>Get a desktop notification when usage approaches or exceeds a cap. Free tier.</p>
      </div>
      {#if savedKey !== null}<span class="saved" role="status">Saved</span>{/if}
    </header>

    {#if settingsStore.loading}
      <p class="muted" role="status">Reading settings…</p>
    {:else if settingsStore.error !== null}
      <div class="state-error" role="alert">
        <strong>Settings could not be loaded</strong>
        <p>{settingsStore.error}</p>
      </div>
    {:else}
      <div class="fields">
        {#each fields as field (field.key)}
          {@const entry = settingsStore.get(field.key)}
          {#if entry}
            <Input
              type="number"
              min={entry.min}
              max={entry.max}
              label={field.label}
              helper={entry.help}
              value={String(entry.value)}
              onchange={(event: Event) => void save(field.key, event)}
            />
          {/if}
        {/each}
      </div>
      <p class="hint">
        0 disables a budget. Counts use metered interfaces when any are marked metered.
      </p>
    {/if}
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
</section>

<style>
  .settings {
    display: grid;
    gap: var(--nwp-space-5);
    max-inline-size: 760px;
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
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-success-soft);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-success);
    font-size: var(--nwp-size-label);
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
  /* §7.14: warning/error colors always paired with a text label below. */
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
