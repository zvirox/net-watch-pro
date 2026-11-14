<script lang="ts">
  // Root component — P0.3/P0.5 smoke: vendored Button renders (PLAN P0.5 verify).
  // Themed app shell (sidebar/topbar/screens) lands with P1.7 (DESIGN.md §8.1).
  import { Button } from '$lib/components/ui';

  const version: string = '0.0.1-alpha.1';
</script>

<main>
  <h1>Net Watch Pro</h1>
  <p>Skeleton UI — Phase 0. Design-system shell lands in P1.7.</p>
  <p>v{version}</p>

  <div class="smoke-row">
    <Button variant="primary">Enable monitoring</Button>
    <Button variant="secondary">Secondary</Button>
    <Button variant="ghost">Ghost</Button>
    <Button variant="destructive">Stop daemon</Button>
  </div>
</main>

<style>
  main {
    font-family: var(--nwp-font-ui);
    margin: var(--nwp-space-12) auto;
    max-width: 64ch;
    color: var(--nwp-text);
    background: var(--nwp-background);
    padding: var(--nwp-space-6);
  }
  h1 {
    font-size: var(--nwp-size-display);
    margin: 0 0 var(--nwp-space-3);
  }
  .smoke-row {
    display: flex;
    gap: var(--nwp-space-3);
    margin-top: var(--nwp-space-6);
  }
</style>
