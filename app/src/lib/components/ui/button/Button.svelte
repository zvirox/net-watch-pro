<script lang="ts" module>
  // DESIGN.md §7.1 — vendored shadcn-svelte-style Button, restyled to tokens.
  import type { HTMLButtonAttributes } from 'svelte/elements';

  export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'destructive';
  export type ButtonSize = 'md' | 'sm';
</script>

<script lang="ts">
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLButtonAttributes {
    variant?: ButtonVariant;
    size?: ButtonSize;
    /** §7.1 Loading: label swapped for spinner + verb; disabled while loading. */
    loading?: boolean;
    /** Verb shown while loading (defaults to "Working…"). */
    loadingLabel?: string;
  }

  let {
    variant = 'primary',
    size = 'md',
    loading = false,
    loadingLabel = 'Working…',
    type = 'button',
    disabled = false,
    class: className,
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class={cn('nwp-btn', `nwp-btn--${variant}`, `nwp-btn--${size}`, className)}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  {...rest}
>
  {#if loading}
    <span class="nwp-btn__spinner" aria-hidden="true"></span>
    <span>{loadingLabel}</span>
  {:else}
    {@render children?.()}
  {/if}
</button>

<style>
  .nwp-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--nwp-space-2);
    border: 1px solid transparent;
    border-radius: var(--nwp-radius-md);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-body);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color var(--nwp-dur-fast) var(--nwp-ease-standard),
      border-color var(--nwp-dur-fast) var(--nwp-ease-standard),
      color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }

  /* §7.1 sizing: min-height 36px (sm 28px); padding 8px/16px (sm 6px/12px) */
  .nwp-btn--md {
    min-height: 36px;
    padding: var(--nwp-space-2) var(--nwp-space-4);
  }
  .nwp-btn--sm {
    min-height: 28px;
    padding: var(--nwp-space-2) 12px;
    font-size: var(--nwp-size-label);
  }

  /* Variants — rest fills per §7.1 table */
  .nwp-btn--primary {
    background: var(--nwp-primary);
    color: var(--nwp-on-primary);
  }
  .nwp-btn--secondary {
    background: var(--nwp-surface-2);
    color: var(--nwp-text);
    border-color: var(--nwp-border-strong);
  }
  .nwp-btn--ghost {
    background: transparent;
    color: var(--nwp-primary);
  }
  .nwp-btn--destructive {
    background: var(--nwp-error);
    color: var(--nwp-on-status);
  }

  /* Hover (100ms) */
  .nwp-btn--primary:not(:disabled):hover {
    background: var(--nwp-primary-hover);
  }
  .nwp-btn--secondary:not(:disabled):hover {
    border-color: var(--nwp-border-strong);
    background: var(--nwp-surface-2);
  }
  .nwp-btn--ghost:not(:disabled):hover {
    background: var(--nwp-surface-2);
  }
  .nwp-btn--destructive:not(:disabled):hover {
    background: var(--nwp-error-hover);
  }

  /* Active — fills darken one step; no shadow change */
  .nwp-btn--primary:not(:disabled):active {
    background: var(--nwp-primary-active);
  }

  /* Focus-visible — §9.2 uniform ring, never removed */
  .nwp-btn:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }

  /* Disabled — surface-2 fill + disabled text; opacity stays 100% (§7.1) */
  .nwp-btn:disabled {
    background: var(--nwp-surface-2);
    color: var(--nwp-text-disabled);
    border-color: var(--nwp-border);
    cursor: not-allowed;
  }

  /* Loading spinner — 16px, width preserved via min-height + centered content */
  .nwp-btn__spinner {
    inline-size: 16px;
    block-size: 16px;
    border: 2px solid currentColor;
    border-block-start-color: transparent;
    border-radius: var(--nwp-radius-full);
    animation: nwp-spin 0.8s linear infinite;
  }
  @keyframes nwp-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .nwp-btn__spinner {
      animation-duration: 1ms;
    }
  }
</style>
