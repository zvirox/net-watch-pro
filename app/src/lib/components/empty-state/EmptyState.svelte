<!-- Shared centered empty state — DESIGN.md §7.11. No retry action here. -->
<script lang="ts">
  type Props = {
    title: string;
    description: string;
    /** Compact glyph shown in the 48px token circle. */
    icon?: string;
    class?: string;
  };

  let { title, description, icon = '⌁', class: className = '' }: Props = $props();
</script>

<div class="empty-state {className}" role="status">
  <span class="empty-icon" aria-hidden="true">{icon}</span>
  <h3>{title}</h3>
  <p>{description}</p>
</div>

<style>
  .empty-state {
    display: flex;
    min-height: 200px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--nwp-space-3);
    text-align: center;
  }
  .empty-icon {
    display: grid;
    width: 48px;
    height: 48px;
    place-items: center;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface-2);
    color: var(--nwp-text-disabled);
    font-size: 24px;
  }
  h3 {
    margin: 0;
    color: var(--nwp-text);
    font-size: var(--nwp-size-title-3);
    font-weight: 600;
  }
  p {
    max-width: 32ch;
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-body);
    line-height: 1.5;
  }
</style>
