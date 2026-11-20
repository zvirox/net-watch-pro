<script lang="ts">
  // Upgrade / license modal — DESIGN.md §8.8, glass recipe per §7.4.
  // Entry state per §7.2: invalid keys surface an inline input error.
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { Button, Input } from '$lib/components/ui';
  import { LICENSE_PURCHASE_URL, licenseStore } from '$lib/stores/license.svelte';

  let key = $state('');
  let modalEl: HTMLDivElement | undefined = $state();

  const reduceMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const enterMs = reduceMotion ? 0 : 240;
  const exitMs = reduceMotion ? 0 : 160;

  // §9.3: remember the trigger, focus the first field, and restore focus on
  // close (including Escape) so keyboard users continue where they left off.
  let trigger: HTMLElement | null = null;
  $effect(() => {
    if (licenseStore.upgradeOpen) {
      if (trigger === null) {
        trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      }
      modalEl?.querySelector<HTMLInputElement>('input')?.focus();
      key = '';
    } else if (trigger !== null) {
      if (trigger.isConnected) trigger.focus();
      trigger = null;
    }
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await licenseStore.activate(key);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      licenseStore.closeUpgrade();
      return;
    }
    if (event.key !== 'Tab' || !modalEl) return;
    // §9.3: focus stays inside the dialog while it is open.
    const focusable = Array.from(
      modalEl.querySelectorAll<HTMLElement>('button:not(:disabled), a[href], input:not(:disabled)'),
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

{#if licenseStore.upgradeOpen}
  <div class="overlay">
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="upgrade-title"
      tabindex="-1"
      bind:this={modalEl}
      onkeydown={onKeydown}
      in:fly={{ y: reduceMotion ? 0 : 8, duration: enterMs, easing: cubicOut }}
      out:fade={{ duration: exitMs }}
    >
      <header class="header">
        <p class="brand">✦ NET WATCH PRO</p>
        <h2 id="upgrade-title">Unlock Pro</h2>
        <button
          class="close"
          type="button"
          aria-label="Close"
          onclick={() => {
            licenseStore.closeUpgrade();
          }}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            aria-hidden="true"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </header>

      <div class="body">
        {#if licenseStore.isPro}
          <div class="active" role="status">
            <p class="active__title">✦ Pro is active</p>
            <p class="active__text">
              Advanced history filters, CSV/JSON export and 365-day retention are unlocked on this
              device.
            </p>
            <Button
              variant="ghost"
              size="sm"
              loading={licenseStore.activating}
              onclick={() => void licenseStore.deactivate()}
            >
              Deactivate license
            </Button>
          </div>
          {#if licenseStore.error}
            <p class="sr-only" role="alert">{licenseStore.error}</p>
          {/if}
        {:else}
          <ul class="benefits">
            <li>
              <span class="check" aria-hidden="true">✓</span><span>Multi-filter history search</span
              >
            </li>
            <li><span class="check" aria-hidden="true">✓</span><span>CSV/JSON export</span></li>
            <li>
              <span class="check" aria-hidden="true">✓</span><span>Retention up to 1 year</span>
            </li>
          </ul>

          <form onsubmit={(event) => void submit(event)}>
            <Input
              label="License key"
              placeholder="NWP1-…"
              helper="Paste the key from your license email. Verification is offline."
              error={licenseStore.error ?? undefined}
              autocomplete="off"
              spellcheck="false"
              oninput={(event: Event) => {
                key = (event.currentTarget as HTMLInputElement).value;
                licenseStore.clearError();
              }}
            />
            <div class="actions">
              <a class="buy" href={LICENSE_PURCHASE_URL} target="_blank" rel="noopener noreferrer">
                Buy a license
              </a>
              <Button
                type="submit"
                loading={licenseStore.activating}
                loadingLabel="Activating…"
                disabled={key.trim().length === 0}
              >
                Activate
              </Button>
            </div>
          </form>
          <p class="sr-only" role="status" aria-live="polite">
            {licenseStore.loading ? 'Reading license state' : ''}
          </p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: var(--nwp-space-4);
    background: var(--nwp-overlay);
  }
  .modal {
    display: flex;
    width: min(560px, 100%);
    max-height: 80vh;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--nwp-glass-border);
    border-radius: var(--nwp-radius-xl);
    background: var(--nwp-glass-surface);
    backdrop-filter: blur(8px);
    box-shadow: var(--nwp-shadow-xl);
    font-family: var(--nwp-font-ui);
    color: var(--nwp-text);
  }
  .header {
    position: relative;
    padding: var(--nwp-space-6) var(--nwp-space-6) var(--nwp-space-4);
    border-bottom: 1px solid var(--nwp-glass-border);
  }
  .brand {
    margin: 0;
    color: var(--nwp-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    font-weight: 600;
    letter-spacing: 0.08em;
  }
  h2 {
    margin: var(--nwp-space-2) 0 0;
    font-size: var(--nwp-size-title-1);
    font-weight: 600;
  }
  .close {
    position: absolute;
    top: var(--nwp-space-4);
    right: var(--nwp-space-4);
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: var(--nwp-radius-md);
    background: transparent;
    color: var(--nwp-text-secondary);
    cursor: pointer;
  }
  .close:hover {
    background: var(--nwp-surface-2);
    color: var(--nwp-text);
  }
  .close svg {
    width: 16px;
    height: 16px;
  }
  .close:focus-visible,
  .buy:focus-visible,
  .modal :global(button:focus-visible) {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .body {
    padding: var(--nwp-space-6);
    overflow-y: auto;
  }
  .benefits {
    display: grid;
    gap: var(--nwp-space-3);
    margin: 0 0 var(--nwp-space-6);
    padding: 0;
    list-style: none;
  }
  .benefits li {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
    font-size: var(--nwp-size-body);
  }
  .check {
    display: grid;
    width: 20px;
    height: 20px;
    place-items: center;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-secondary-soft);
    color: var(--nwp-secondary);
    font-size: var(--nwp-size-label);
    font-weight: 700;
  }
  form {
    display: grid;
    gap: var(--nwp-space-5);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--nwp-space-2);
    border-top: 1px solid var(--nwp-glass-border);
    padding-top: var(--nwp-space-4);
  }
  .buy {
    display: inline-flex;
    min-height: 36px;
    align-items: center;
    padding: var(--nwp-space-2) var(--nwp-space-4);
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface-2);
    color: var(--nwp-text);
    font-size: var(--nwp-size-body);
    font-weight: 500;
    text-decoration: none;
  }
  .buy:hover {
    background: var(--nwp-surface);
  }
  .active {
    display: grid;
    gap: var(--nwp-space-3);
    justify-items: start;
    padding: var(--nwp-space-4);
    border: 1px solid var(--nwp-secondary);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-secondary-soft);
  }
  .active__title {
    margin: 0;
    font-size: var(--nwp-size-title-3);
    font-weight: 600;
    color: var(--nwp-secondary);
  }
  .active__text {
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-body);
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
</style>
