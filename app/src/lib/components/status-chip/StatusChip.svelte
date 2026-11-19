<script lang="ts">
  import { daemonStore } from '$lib/stores/daemon.svelte';

  const state = $derived(daemonStore.chipState);
  const labels = {
    monitoring: 'Monitoring',
    degraded: 'Degraded',
    stopped: 'Stopped',
    starting: 'Connecting…',
  } as const;
</script>

<button
  class="status-chip status-chip--{state}"
  type="button"
  aria-label="Daemon status: {labels[state]}"
  aria-live="polite"
  title={state === 'degraded' && daemonStore.ebpf === 'no_collector'
    ? 'No collector — PC live speed works, per-app rates unavailable'
    : labels[state]}
  onclick={() => {
    if (daemonStore.connection === 'auth-required' || daemonStore.connection === 'auth-invalid') {
      void daemonStore.authorize();
    }
  }}
>
  <span class="status-chip__icon" aria-hidden="true">
    {#if state === 'monitoring'}✓{:else if state === 'degraded'}!{:else if state === 'stopped'}×{:else}<span
        class="status-chip__spinner"
      ></span>{/if}
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
    font-weight: 700;
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
