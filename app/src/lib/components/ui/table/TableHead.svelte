<script lang="ts">
  // DESIGN.md §7.6 — header cell. Numeric columns: right-aligned + right-aligned header.
  import type { HTMLThAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLThAttributes {
    class?: string;
    /** Right-align (numeric columns: bytes, rate, port) — header matches. */
    numeric?: boolean;
  }
  let { class: className, numeric = false, children, ...rest }: Props = $props();
</script>

<th class={cn('nwp-table-head-cell', numeric && 'nwp-table-head-cell--numeric', className)} {...rest}>
  {@render children?.()}
</th>

<style>
  .nwp-table-head-cell {
    padding: var(--nwp-space-2) var(--nwp-space-3);
    text-align: start;
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-label);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--nwp-text-secondary);
    white-space: nowrap;
  }
  .nwp-table-head-cell--numeric {
    text-align: end;
  }
</style>
