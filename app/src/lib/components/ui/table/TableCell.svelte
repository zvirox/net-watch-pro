<script lang="ts">
  // DESIGN.md §7.6 — body cell. Numeric columns: font-mono, tabular-nums, right-aligned.
  import type { HTMLTdAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLTdAttributes {
    class?: string;
    numeric?: boolean;
    mono?: boolean;
  }
  let { class: className, numeric = false, mono = false, children, ...rest }: Props = $props();
</script>

<td
  class={cn(
    'nwp-table-cell',
    numeric && 'nwp-table-cell--numeric',
    mono && 'nwp-table-cell--mono',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</td>

<style>
  .nwp-table-cell {
    padding: var(--nwp-space-2) var(--nwp-space-3);
    text-align: start;
    white-space: nowrap;
  }
  .nwp-table-cell--mono {
    font-family: var(--nwp-font-mono);
  }
  .nwp-table-cell--numeric {
    font-family: var(--nwp-font-mono);
    font-variant-numeric: tabular-nums;
    text-align: end;
  }
</style>
