<script lang="ts">
  // DESIGN.md §7.2 — vendored shadcn-svelte-style Input, restyled to tokens.
  // Full state set: default/hover/focus/disabled/loading/error/read-only.
  import type { HTMLInputAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLInputAttributes {
    /** Field label rendered above (12px, 500). */
    label?: string;
    /** Helper text below; replaced by error when error is set. */
    helper?: string;
    /** Inline error message (border → error, message 12px 500). */
    error?: string;
    /** Async validation/search spinner at field end; stays editable. */
    loading?: boolean;
    /** Search variant: pill radius, leading magnifier. Clear button when non-empty. */
    search?: boolean;
    id?: string;
    value?: string | number | null;
    class?: string;
  }

  let {
    label,
    helper,
    error,
    loading = false,
    search = false,
    id,
    value = '',
    class: className,
    ...rest
  }: Props = $props();

  // Stable id per instance for label/description wiring (a11y §9.3).
  const uid = `nwp-input-${Math.random().toString(36).slice(2, 10)}`;
  const inputId = $derived(id ?? uid);
  const descId = $derived(`${inputId}-desc`);
</script>

<div class={cn('nwp-field', className)}>
  {#if label}
    <label class="nwp-field__label" for={inputId}>{label}</label>
  {/if}

  <div class="nwp-field__control" class:nwp-field__control--search={search} class:nwp-field__control--error={!!error}>
    {#if search}
      <!-- §7.2: leading search icon 16px (icon set lands per DESIGN.md §6.1) -->
      <svg class="nwp-field__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
      </svg>
    {/if}
    <input
      id={inputId}
      class="nwp-field__input"
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={helper || error ? descId : undefined}
      bind:value
      {...rest}
    />
    {#if loading}
      <span class="nwp-field__spinner" aria-hidden="true"></span>
    {/if}
  </div>

  {#if error}
    <p class="nwp-field__error" id={descId}>{error}</p>
  {:else if helper}
    <p class="nwp-field__helper" id={descId}>{helper}</p>
  {/if}
</div>

<style>
  .nwp-field {
    display: flex;
    flex-direction: column;
    gap: var(--nwp-space-2);
    font-family: var(--nwp-font-ui);
  }

  /* §7.2 structure: label 12px/500 above, 8px gap */
  .nwp-field__label {
    font-size: var(--nwp-size-label);
    font-weight: 500;
    color: var(--nwp-text);
  }

  .nwp-field__control {
    position: relative;
    display: flex;
    align-items: center;
  }

  /* §7.2: height 36px, padding 8px/12px, body text, border-strong 1px */
  .nwp-field__input {
    inline-size: 100%;
    block-size: 36px;
    padding: var(--nwp-space-2) 12px;
    background: var(--nwp-surface);
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-md);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-body);
    color: var(--nwp-text);
    transition: border-color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .nwp-field__input::placeholder {
    color: var(--nwp-text-disabled);
  }
  .nwp-field__input:disabled,
  .nwp-field__input[readonly] {
    background: var(--nwp-surface-2);
    color: var(--nwp-text-disabled);
    border-color: var(--nwp-border);
    cursor: not-allowed;
  }
  .nwp-field__input:not(:disabled):hover {
    border-color: var(--nwp-border);
  }
  /* Focus — border → primary + ring; read-only gets no ring (§7.2) */
  .nwp-field__input:not(:disabled):not([readonly]):focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 0;
    border-color: var(--nwp-primary);
  }

  /* Error state — border → error; ring uses error until corrected */
  .nwp-field__control--error .nwp-field__input {
    border-color: var(--nwp-error);
  }
  .nwp-field__control--error .nwp-field__input:focus-visible {
    outline-color: var(--nwp-error);
  }

  /* Search pill variant */
  .nwp-field__control--search .nwp-field__input {
    border-radius: var(--nwp-radius-full);
    padding-inline-start: 32px;
  }
  .nwp-field__icon {
    position: absolute;
    inset-inline-start: 10px;
    inline-size: 16px;
    block-size: 16px;
    color: var(--nwp-text-disabled);
    pointer-events: none;
  }

  /* Async loading spinner — 16px at field end; field stays editable */
  .nwp-field__spinner {
    position: absolute;
    inset-inline-end: 10px;
    inline-size: 16px;
    block-size: 16px;
    border: 2px solid var(--nwp-text-secondary);
    border-block-start-color: transparent;
    border-radius: var(--nwp-radius-full);
    animation: nwp-input-spin 0.8s linear infinite;
  }
  @keyframes nwp-input-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .nwp-field__spinner {
      animation-duration: 1ms;
    }
  }

  /* Helper / error below the field */
  .nwp-field__helper {
    font-size: var(--nwp-size-caption);
    color: var(--nwp-text-secondary);
  }
  .nwp-field__error {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-1);
    font-size: var(--nwp-size-label);
    font-weight: 500;
    color: var(--nwp-error);
  }
</style>
