<!-- Shared centered empty state — DESIGN.md §7.11. No retry action here. -->
<script lang="ts">
  // DESIGN.md §6.2: Lucide stroke icons. Inline SVG avoids an icon dependency;
  // each entry is a Lucide `path` `d` string, 24px, 2px stroke, currentColor.
  const iconPaths: Record<string, string[]> = {
    activity: ['M22 12h-4l-3 9L9 3l-3 9H2'],
    clock: ['M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8', 'M3 3v5h5', 'M12 7v5l4 2'],
    grid: ['M3 3h7v7H3z', 'M14 3h7v7h-7z', 'M14 14h7v7h-7z', 'M3 14h7v7H3z'],
    chart: ['M3 3v18h18', 'M7 16l4-5 3 3 5-7'],
    wifi: [
      'M5 12.55a11 11 0 0 1 14.08 0',
      'M1.42 9a16 16 0 0 1 21.16 0',
      'M8.53 16.11a6 6 0 0 1 6.95 0',
      'M12 20h.01',
    ],
  };

  type Props = {
    title: string;
    description: string;
    /** Named Lucide icon shown in the 48px token circle. */
    icon?: string;
    class?: string;
  };

  let { title, description, icon = 'activity', class: className = '' }: Props = $props();
  const paths = $derived(iconPaths[icon] ?? iconPaths.activity);
</script>

<div class="empty-state {className}" role="status">
  <span class="empty-icon" aria-hidden="true">
    <svg viewBox="0 0 24 24">
      {#each paths as path, index (index)}
        <path d={path} />
      {/each}
    </svg>
  </span>
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
    /* §9.1: text-disabled is exempt for disabled controls, not for meaningful
       empty-state icons — text-secondary clears 5:1 in both themes. */
    color: var(--nwp-text-secondary);
  }
  .empty-icon svg {
    width: 24px;
    height: 24px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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
