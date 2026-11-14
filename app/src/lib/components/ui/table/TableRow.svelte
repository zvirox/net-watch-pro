<script lang="ts">
  // DESIGN.md §7.6 — row: 40px (dense 32px), hairline border, surface-2 hover.
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLAttributes<HTMLTableRowElement> {
    class?: string;
    dense?: boolean;
    /** Selected row: primary-soft fill + 2px inset bar. */
    selected?: boolean;
  }
  let { class: className, dense = false, selected = false, children, ...rest }: Props = $props();
</script>

<tr
  class={cn('nwp-table-row', dense && 'nwp-table-row--dense', selected && 'nwp-table-row--selected', className)}
  {...rest}
>
  {@render children?.()}
</tr>

<style>
  .nwp-table-row {
    border-block-end: 1px solid var(--nwp-border);
    transition: background-color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .nwp-table-row:not(.nwp-table-row--selected):hover {
    background: var(--nwp-surface-2);
  }
  .nwp-table-row--dense {
    block-size: 32px;
  }
  /* Selected — §7.6: primary-soft + 2px left inset bar */
  .nwp-table-row--selected {
    background: var(--nwp-primary-soft);
    box-shadow: inset 2px 0 0 var(--nwp-primary);
  }
  :global(.nwp-table-row:focus-visible) {
    outline: 2px solid var(--nwp-primary);
    outline-offset: -2px;
  }
</style>
