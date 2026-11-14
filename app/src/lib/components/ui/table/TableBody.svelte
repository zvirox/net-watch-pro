<script lang="ts">
  // DESIGN.md §7.6 — table body wrapper.
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLAttributes<HTMLTableSectionElement> {
    class?: string;
  }
  let { class: className, children, ...rest }: Props = $props();
</script>

<tbody class={cn('nwp-table-body', className)} {...rest}>
  {@render children?.()}
</tbody>
