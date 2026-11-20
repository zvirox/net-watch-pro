<script lang="ts">
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { diagnosticsStore } from '$lib/stores/diagnostics.svelte';

  const state = $derived(daemonStore.chipState);
  const labels = {
    monitoring: 'Monitoring',
    degraded: 'Degraded',
    stopped: 'Stopped',
    starting: 'Connecting…',
  } as const;
  // §7.5: shape differs per state — status is never conveyed by color alone.
  const statusIcons = {
    monitoring: ['M20 6 9 17l-5-5'], // check
    degraded: [
      'm10.29 3.86-8.18 14.2A2 2 0 0 0 3.82 21h16.36a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z',
      'M12 9v4',
      'M12 17h.01',
    ], // triangle-alert
    stopped: ['M18 6 6 18', 'M6 6l12 12'], // x
    starting: [], // spinner below
  } as const;
</script>

<button
  class="status-chip status-chip--{state}"
  type="button"
  aria-label="Daemon status: {labels[state]}"
  aria-live="polite"
  title={state === 'degraded' && daemonStore.collectorMissing
    ? daemonStore.ebpf === 'failed'
      ? 'eBPF collector failed — PC live speed works, per-app rates unavailable'
      : 'No collector — PC live speed works, per-app rates unavailable'
    : labels[state]}
  onclick={() => {
    if (daemonStore.connection === 'auth-required' || daemonStore.connection === 'auth-invalid') {
      void daemonStore.authorize();
    } else {
      diagnosticsStore.openModal();
    }
  }}
>
  <span class="status-chip__icon" aria-hidden="true">
    {#if state === 'starting'}
      <span class="status-chip__spinner"></span>
    {:else}
      <svg viewBox="0 0 24 24">
        {#each statusIcons[state] as path, index (index)}
          <path d={path} />
        {/each}
      </svg>
    {/if}
  </span>
  <span>{labels[state]}</span>
</button>

<style>
  .status-chip {
    display: inline-flex;
    min-height: 24px;
    align-items: center;
    gap: var(--nwp-space-2);
    border: 0;
    border-radius: var(--nwp-radius-full);
    padding: 0 var(--nwp-space-3);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-label);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color var(--nwp-dur-fast) var(--nwp-ease-standard),
      color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .status-chip--monitoring {
    background: var(--nwp-success-soft);
    color: var(--nwp-success);
  }
  .status-chip--degraded {
    background: var(--nwp-warning-soft);
    color: var(--nwp-warning);
  }
  .status-chip--stopped {
    background: var(--nwp-error-soft);
    color: var(--nwp-error);
  }
  .status-chip--starting {
    background: var(--nwp-surface-2);
    color: var(--nwp-text-secondary);
  }
  .status-chip__icon {
    display: grid;
    width: 12px;
    height: 12px;
    place-items: center;
  }
  .status-chip__icon svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .status-chip__spinner {
    width: 9px;
    height: 9px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: var(--nwp-radius-full);
    animation: status-spin 800ms linear infinite;
  }
  .status-chip:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  @keyframes status-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .status-chip__spinner {
      animation-duration: 1ms;
    }
  }
</style>
