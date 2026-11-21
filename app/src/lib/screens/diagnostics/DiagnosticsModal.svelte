<!-- Diagnostics modal — DESIGN.md §8.9 / §7.4 (glass recipe, focus trap). -->
<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { Button } from '$lib/components/ui';
  import { diagnosticsStore, type CheckResult } from '$lib/stores/diagnostics.svelte';
  import { formatBytes } from '$lib/utils/format';

  let modalEl: HTMLDivElement | undefined = $state();
  let trigger: HTMLElement | null = null;
  let copied = $state(false);

  const reduceMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const enterMs = reduceMotion ? 0 : 240;
  const exitMs = reduceMotion ? 0 : 160;

  const symbols: Record<CheckResult, string> = { pass: '✓', fail: '✗', skip: '—' };
  const words: Record<CheckResult, string> = { pass: 'pass', fail: 'fail', skip: 'not checked' };

  $effect(() => {
    if (diagnosticsStore.open && !trigger) {
      trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      modalEl?.querySelector<HTMLElement>('button')?.focus();
    }
    if (!diagnosticsStore.open && trigger) {
      if (trigger.isConnected) trigger.focus();
      trigger = null;
    }
  });

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      diagnosticsStore.closeModal();
      return;
    }
    if (event.key !== 'Tab' || !modalEl) return;
    const focusable = Array.from(
      modalEl.querySelectorAll<HTMLElement>('button:not(:disabled), a[href]'),
    );
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

  async function copyReport(): Promise<void> {
    const text = JSON.stringify(
      diagnosticsStore.payload ?? { error: 'no diagnostics loaded' },
      null,
      2,
    );
    try {
      await window.navigator.clipboard.writeText(text);
      copied = true;
    } catch {
      copied = false;
    }
  }

  function stamp(epoch: number): string {
    return new Date(epoch * 1000).toLocaleString();
  }
</script>

{#if diagnosticsStore.open}
  <div class="overlay">
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="diagnostics-title"
      tabindex="-1"
      bind:this={modalEl}
      onkeydown={onKeydown}
      in:fly={{ y: reduceMotion ? 0 : 8, duration: enterMs, easing: cubicOut }}
      out:fade={{ duration: exitMs }}
    >
      <header class="header">
        <div>
          <p class="eyebrow">LOCAL HEALTH</p>
          <h2 id="diagnostics-title">Diagnostics</h2>
        </div>
        <button
          type="button"
          class="close"
          aria-label="Close diagnostics"
          onclick={() => {
            diagnosticsStore.closeModal();
          }}>×</button
        >
      </header>

      <div class="body">
        {#if diagnosticsStore.error}
          <div class="inline-error" role="alert">{diagnosticsStore.error}</div>
        {/if}
        {#if diagnosticsStore.loading && diagnosticsStore.payload === null}
          <p class="muted">Reading local diagnostics…</p>
        {:else if diagnosticsStore.payload}
          <section aria-label="Health checks">
            <h3>Checks</h3>
            <ul class="checks">
              {#each diagnosticsStore.payload.checks as check (check.name)}
                <li>
                  <span class="mark mark--{check.result}" aria-hidden="true"
                    >{symbols[check.result]}</span
                  >
                  <span class="check-name">{check.name}</span>
                  <span class="check-result">{words[check.result]}</span>
                  {#if check.detail}<span class="detail">{check.detail}</span>{/if}
                  {#if check.guidance_url}<span class="detail">{check.guidance_url}</span>{/if}
                </li>
              {/each}
            </ul>
          </section>

          <section aria-label="Journal excerpt">
            <h3>Recent events</h3>
            {#if diagnosticsStore.payload.journal_excerpt.length === 0}
              <p class="muted">No events recorded.</p>
            {:else}
              <div class="journal" role="region" aria-label="Recent daemon events">
                {#each diagnosticsStore.payload.journal_excerpt as event, index (index)}
                  <p>
                    <span class="stamp">{stamp(event.occurred_at)}</span>
                    <span class="level level--{event.level}">{event.level}</span>
                    <span class="code">{event.code}</span>
                    <span>{event.message}</span>
                  </p>
                {/each}
              </div>
            {/if}
          </section>

          <p class="data">
            Local database: {formatBytes(diagnosticsStore.payload.data.db_size_bytes)} · retention
            {String(diagnosticsStore.payload.data.retention_days)} days
          </p>
        {/if}
      </div>

      <footer>
        <Button variant="ghost" size="sm" onclick={() => void copyReport()}
          >{copied ? 'Copied' : 'Copy report'}</Button
        >
        <Button
          variant="secondary"
          size="sm"
          loading={diagnosticsStore.acting}
          loadingLabel="Starting…"
          onclick={() => void diagnosticsStore.startDaemon()}>Restart daemon</Button
        >
        <Button
          variant="primary"
          size="sm"
          onclick={() => {
            diagnosticsStore.closeModal();
          }}>Close</Button
        >
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    z-index: 30;
    inset: 0;
    display: grid;
    place-items: center;
    background: var(--nwp-overlay);
  }
  .modal {
    display: flex;
    width: min(720px, 92vw);
    max-height: 80vh;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--nwp-glass-border);
    border-radius: var(--nwp-radius-xl);
    background: var(--nwp-glass-surface);
    box-shadow: var(--nwp-shadow-xl);
    backdrop-filter: blur(8px);
  }
  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    border-bottom: 1px solid var(--nwp-glass-border);
    padding: var(--nwp-space-6);
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
    font-size: var(--nwp-size-title-2);
  }
  h3 {
    margin: 0 0 var(--nwp-space-2);
    font-size: var(--nwp-size-title-3);
  }
  .close {
    border: 0;
    background: transparent;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-title-2);
    cursor: pointer;
  }
  .body {
    display: grid;
    gap: var(--nwp-space-5);
    overflow-y: auto;
    padding: var(--nwp-space-6);
  }
  .checks {
    display: grid;
    gap: var(--nwp-space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .checks li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--nwp-space-2);
    font-size: var(--nwp-size-body);
  }
  .mark {
    display: inline-grid;
    width: 20px;
    height: 20px;
    place-items: center;
    border-radius: var(--nwp-radius-full);
    font-family: var(--nwp-font-mono);
    font-weight: 600;
  }
  .mark--pass {
    background: var(--nwp-success-soft);
    color: var(--nwp-success);
  }
  .mark--fail {
    background: var(--nwp-error-soft);
    color: var(--nwp-error);
  }
  .mark--skip {
    background: var(--nwp-surface-2);
    color: var(--nwp-text-secondary);
  }
  .check-name {
    font-weight: 600;
  }
  .check-result,
  .detail,
  .code,
  .stamp,
  .level {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .journal {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    padding: var(--nwp-space-3);
  }
  .journal p {
    display: flex;
    flex-wrap: wrap;
    gap: var(--nwp-space-2);
    margin: 0 0 var(--nwp-space-2);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .level--error {
    color: var(--nwp-error);
  }
  .level--warn {
    color: var(--nwp-warning);
  }
  .data,
  .muted {
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .inline-error {
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-error-soft);
    padding: var(--nwp-space-3);
    color: var(--nwp-error);
    font-size: var(--nwp-size-label);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--nwp-space-2);
    border-top: 1px solid var(--nwp-glass-border);
    padding: var(--nwp-space-4) var(--nwp-space-6);
  }
  button:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .journal:focus-visible,
  .close:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
</style>
