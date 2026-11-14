<script lang="ts">
  // DESIGN.md §7.6 — vendored shadcn-svelte table family, token-styled.
  // Wrapper keeps the sticky-header scroll container contract.
  import type { HTMLTableAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils/cn';

  interface Props extends HTMLTableAttributes {
    class?: string;
  }
  let { class: className, children, ...rest }: Props = $props();
</script>

<div class="nwp-table__wrap">
  <table class={cn('nwp-table', className)} {...rest}>
    {@render children?.()}
  </table>
</div>

<style>
  .nwp-table__wrap {
    overflow: auto; /* sticky header within scroll container (§7.6) */
  }
  .nwp-table {
    inline-size: 100%;
    border-collapse: collapse;
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-body);
    color: var(--nwp-text);
  }
</style>
