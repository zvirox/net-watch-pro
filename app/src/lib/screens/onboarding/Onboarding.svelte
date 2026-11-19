<script lang="ts">
  // First-run onboarding (DESIGN.md §8.2): 3-card summary, one pkexec prompt,
  // health-checked success. Demo mode is OQ4-gated (decided in P8.6).
  import { Button } from '$lib/components/ui';
  import { demoStore } from '$lib/stores/demo.svelte';
  import { onboardingStore } from '$lib/stores/onboarding.svelte';

  const cards = [
    {
      title: 'What we collect',
      body: 'Connection timestamps, apps, remote endpoints and byte counters. Never packet contents.',
    },
    {
      title: "Where it's stored",
      body: 'A SQLite database owned by root on this device. No cloud, no telemetry, no outbound traffic.',
    },
    {
      title: 'How to remove it',
      body: 'Uninstall the package. Your recorded data stays until you choose to delete it.',
    },
  ];
</script>

<section class="onboarding" aria-labelledby="onboarding-title">
  <img class="logo" src="/logo.png" alt="" width="48" height="48" />
  <h1 id="onboarding-title">Welcome to Net Watch Pro</h1>
  <p class="pitch">
    See where your data goes, which apps use it, and stay inside your cap — privately.
  </p>

  <div class="cards">
    {#each cards as card (card.title)}
      <article class="card">
        <h2>{card.title}</h2>
        <p>{card.body}</p>
      </article>
    {/each}
  </div>

  {#if onboardingStore.error !== null}
    <div class="failure" role="alert">
      <div>
        <strong>Monitoring could not be enabled</strong>
        {#if onboardingStore.guidance !== null}<p class="failure__guidance">
            {onboardingStore.guidance}
          </p>{/if}
        <p class="failure__raw">{onboardingStore.error}</p>
      </div>
      <div class="failure__actions">
        <a
          href="https://github.com/zvirox/net-watch-pro/blob/main/SETUP.md"
          target="_blank"
          rel="noopener noreferrer"
        >
          View docs
        </a>
        <Button size="sm" onclick={() => void onboardingStore.enable()}>Retry</Button>
      </div>
    </div>
  {/if}

  <div class="actions">
    <Button
      loading={onboardingStore.enabling}
      loadingLabel="Enabling…"
      disabled={onboardingStore.enabling}
      onclick={() => void onboardingStore.enable()}
    >
      Enable monitoring
    </Button>
    <Button
      variant="ghost"
      onclick={() => {
        demoStore.start();
      }}
    >
      See a demo instead
    </Button>
  </div>
  <p class="note">One admin prompt, once. Later launches connect without asking.</p>
</section>

<style>
  .onboarding {
    display: flex;
    max-inline-size: 760px;
    flex-direction: column;
    align-items: center;
    margin: var(--nwp-space-16) auto;
    text-align: center;
  }
  .logo {
    margin-bottom: var(--nwp-space-4);
  }
  h1 {
    margin: 0;
    font-size: var(--nwp-size-display);
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .pitch {
    max-inline-size: 68ch;
    margin: var(--nwp-space-3) 0 var(--nwp-space-8);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-body);
  }
  .cards {
    display: grid;
    width: 100%;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-8);
  }
  .card {
    padding: var(--nwp-space-4);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
    text-align: left;
  }
  .card h2 {
    margin: 0 0 var(--nwp-space-2);
    font-size: var(--nwp-size-title-3);
    font-weight: 600;
  }
  .card p {
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
    line-height: 1.5;
  }
  .failure {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-5);
    padding: var(--nwp-space-4);
    border: 1px solid var(--nwp-error);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-error-soft);
    text-align: left;
  }
  .failure strong {
    color: var(--nwp-error);
  }
  .failure p {
    margin: var(--nwp-space-1) 0 0;
    font-size: var(--nwp-size-label);
  }
  .failure__guidance {
    color: var(--nwp-text);
    font-weight: 500;
  }
  .failure__raw {
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .failure__actions {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: var(--nwp-space-3);
  }
  .failure__actions a {
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
  }
  .note {
    margin: var(--nwp-space-4) 0 0;
    color: var(--nwp-text-disabled);
    font-size: var(--nwp-size-caption);
  }
</style>
