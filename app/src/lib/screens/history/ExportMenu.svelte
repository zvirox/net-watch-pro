<script lang="ts">
  // History topbar Export menu (DESIGN.md §8.6): free tier opens Upgrade,
  // Pro picks CSV/JSON and the native save dialog (P6.8).
  import { exportStore, type ExportFormat } from '$lib/stores/export.svelte';
  import { licenseStore } from '$lib/stores/license.svelte';

  let open = $state(false);
  let container: HTMLDivElement | undefined = $state();

  function toggle(): void {
    exportStore.clear();
    if (!licenseStore.isPro) {
      licenseStore.openUpgrade();
      return;
    }
    open = !open;
  }

  async function choose(format: ExportFormat): Promise<void> {
    open = false;
    await exportStore.download(format);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') open = false;
  }

  function onWindowClick(event: MouseEvent): void {
    if (open && container !== undefined && !container.contains(event.target as Node)) {
      open = false;
    }
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKeydown} />

<div class="export" bind:this={container}>
  <button
    type="button"
    class="export__button"
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={exportStore.busy}
    onclick={toggle}
  >
    <span>{exportStore.busy ? 'Exporting…' : 'Export'}</span>
    {#if !licenseStore.isPro}<span class="export__pro" aria-label="Pro feature">★</span>{/if}
    <span class="export__caret" aria-hidden="true">▾</span>
  </button>

  {#if open}
    <div class="export__menu" role="menu" aria-label="Export format">
      <button type="button" role="menuitem" onclick={() => void choose('csv')}>CSV</button>
      <button type="button" role="menuitem" onclick={() => void choose('json')}>JSON</button>
    </div>
  {/if}

  {#if exportStore.error}
    <p class="export__note export__note--error" role="alert">{exportStore.error}</p>
  {:else if exportStore.status}
    <p class="export__note" role="status">{exportStore.status}</p>
  {/if}
</div>

<style>
  .export {
    position: relative;
    display: flex;
    align-items: center;
  }
  .export__button {
    display: inline-flex;
    min-height: 36px;
    align-items: center;
    gap: var(--nwp-space-2);
    padding: var(--nwp-space-2) var(--nwp-space-3);
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface-2);
    color: var(--nwp-text);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-body);
    font-weight: 500;
    cursor: pointer;
  }
  .export__button:hover:not(:disabled) {
    border-color: var(--nwp-primary);
    color: var(--nwp-primary);
  }
  .export__button:disabled {
    color: var(--nwp-text-disabled);
    cursor: wait;
  }
  .export__button:focus-visible,
  .export__menu button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .export__pro {
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-secondary-soft);
    padding: 0 var(--nwp-space-1);
    color: var(--nwp-secondary);
    font-size: var(--nwp-size-caption);
  }
  .export__caret {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .export__menu {
    position: absolute;
    top: calc(100% + var(--nwp-space-2));
    right: 0;
    z-index: 20;
    display: grid;
    min-width: 120px;
    padding: var(--nwp-space-1);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    box-shadow: var(--nwp-shadow-md);
  }
  .export__menu button {
    min-height: 28px;
    padding: var(--nwp-space-1) var(--nwp-space-2);
    border: 0;
    border-radius: var(--nwp-radius-sm);
    background: transparent;
    color: var(--nwp-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .export__menu button:hover {
    background: var(--nwp-primary-soft);
    color: var(--nwp-primary);
  }
  .export__note {
    position: absolute;
    top: calc(100% + var(--nwp-space-2));
    right: 0;
    z-index: 20;
    max-inline-size: 280px;
    margin: 0;
    padding: var(--nwp-space-2);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    box-shadow: var(--nwp-shadow-md);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .export__note--error {
    border-color: var(--nwp-error);
    color: var(--nwp-error);
  }
</style>
