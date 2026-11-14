<script lang="ts">
  // DESIGN.md §7.6 — header row: label-mono, uppercase, text-secondary, surface-2.
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLAttributes<HTMLTableSectionElement> {
    class?: string;
  }
  let { class: className, children, ...rest }: Props = $props();
</script>

<thead class={cn('nwp-table-head', className)} {...rest}>
  {@render children?.()}
</thead>

<style>
  .nwp-table-head {
    position: sticky;
    inset-block-start: 0;
    background: var(--nwp-surface-2);
  }
  .nwp-table-head :global(th) {
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-label);
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--nwp-text-secondary);
  }
</style>
