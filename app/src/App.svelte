<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui';
  import StatusChip from '$lib/components/status-chip/StatusChip.svelte';
  import Live from '$lib/screens/live/Live.svelte';
  import History from '$lib/screens/history/History.svelte';
  import { daemonStore } from '$lib/stores/daemon.svelte';

  let connected = $state(daemonStore.connection === 'connected');
  const routes = [
    { label: 'Dashboard', key: 'dashboard' },
    { label: 'Live', key: 'live' },
    { label: 'Apps', key: 'apps' },
    { label: 'History', key: 'history' },
    { label: 'Settings', key: 'settings' },
  ];
  const navigationIcons = ['◫', '⌁', '◉', '◷', '⚙'];
  let activeRoute = $state('dashboard');
  const routeLabel = $derived(
    routes.find((route) => route.key === activeRoute)?.label ?? 'Dashboard',
  );

  onMount(() => {
    const unlisten = daemonStore.onConnection((state) => {
      connected = state === 'connected';
    });
    const updateRoute = () => {
      const route = window.location.hash.slice(1);
      activeRoute = routes.some((item) => item.key === route) ? route : 'dashboard';
    };
    window.addEventListener('hashchange', updateRoute);
    updateRoute();
    void daemonStore.start();
    return () => {
      window.removeEventListener('hashchange', updateRoute);
      unlisten();
      void daemonStore.stop();
    };
  });
</script>

<svelte:head>
  <title>{routeLabel} · Net Watch Pro</title>
</svelte:head>

<div class="app-shell">
  <aside class="sidebar" aria-label="Main navigation">
    <a class="brand" href="#dashboard" aria-label="Net Watch Pro home"
      ><img class="brand__logo" src="/logo.png" alt="" width="32" height="32" /><span
        >Net Watch Pro</span
      ></a
    >
    <nav>
      <p class="nav-heading">MONITOR</p>
      {#each routes as route, index (route.key)}
        <a
          class:active={activeRoute === route.key}
          href="#{route.key}"
          aria-current={activeRoute === route.key ? 'page' : undefined}
          onclick={() => (activeRoute = route.key)}
        >
          <span class="nav-icon" aria-hidden="true">{navigationIcons[index]}</span>
          <span>{route.label}</span>
        </a>
      {/each}
      <p class="nav-heading nav-heading--lower">ACCOUNT</p>
      <a href="#upgrade"><span class="nav-icon" aria-hidden="true">◇</span><span>Upgrade</span></a>
    </nav>
    <div class="sidebar-foot">
      <span class="connection-dot" class:online={connected}></span>{connected
        ? 'Daemon linked'
        : 'Local only'}
    </div>
  </aside>

  <div class="main-column">
    <header class="topbar">
      <div>
        <p class="eyebrow">{activeRoute === 'dashboard' ? 'OVERVIEW' : 'MONITOR'}</p>
        <h1>{routeLabel}</h1>
      </div>
      <div class="topbar-actions">
        <StatusChip /><span class="avatar" aria-label="Local user">L</span>
      </div>
    </header>
    <main class="content">
      {#if activeRoute !== 'dashboard' && activeRoute !== 'live' && activeRoute !== 'history'}
        <section class="notice" role="status">
          <div>
            <strong>{routes.find((route) => route.key === activeRoute)?.label}</strong>
            <p>This screen is scheduled for a later implementation phase.</p>
          </div>
        </section>
      {/if}
      {#if daemonStore.connection === 'auth-required' || daemonStore.connection === 'auth-invalid'}
        <section class="notice" role="status">
          <div>
            <strong>Authorization needed</strong>
            <p>Authenticate locally to connect to the Net Watch Pro daemon.</p>
          </div>
          <Button variant="primary" onclick={() => void daemonStore.authorize()}>Authorize</Button>
        </section>
      {/if}
      {#if activeRoute === 'live'}
        <Live />
      {/if}
      {#if activeRoute === 'history'}
        <History />
      {/if}
      {#if activeRoute === 'dashboard'}
        <section class="welcome-row">
          <div>
            <h2>Your network, clearly in view.</h2>
            <p>Private monitoring that stays on this device.</p>
          </div>
          <span class="local-badge"><span></span> LOCAL ONLY</span>
        </section>
        <section class="metrics" aria-label="Usage summary">
          <article class="metric-card">
            <div class="metric-label">DATA TODAY <span class="metric-icon">↓</span></div>
            <strong>—</strong>
            <p>Usage history starts in Phase 3</p>
          </article>
          <article class="metric-card">
            <div class="metric-label">ACTIVE APPS <span class="metric-icon">◉</span></div>
            <strong>—</strong>
            <p>App attribution starts in Phase 2</p>
          </article>
          <article class="metric-card metric-card--status">
            <div class="metric-label">MONITORING <span class="metric-icon">⌁</span></div>
            <strong
              >{daemonStore.daemon === 'monitoring'
                ? 'On'
                : daemonStore.daemon === 'degraded'
                  ? 'Limited'
                  : 'Off'}</strong
            >
            <p>
              {daemonStore.recording
                ? 'Traffic recording is active'
                : 'Waiting for daemon connection'}
            </p>
          </article>
        </section>
        <section class="lower-grid">
          <article class="panel timeline-panel">
            <div class="panel-heading">
              <div>
                <p class="eyebrow">NETWORK ACTIVITY</p>
                <h2>Usage timeline</h2>
              </div>
              <span class="period">TODAY⌄</span>
            </div>
            <div class="empty-chart">
              <div class="chart-glyph" aria-hidden="true">⌁</div>
              <h3>Waiting for your first data</h3>
              <p>Once monitoring is enabled, your activity will appear here.</p>
              <div class="chart-baseline"></div>
            </div>
          </article>
          <article class="panel apps-panel">
            <div class="panel-heading">
              <div>
                <p class="eyebrow">BY APPLICATION</p>
                <h2>Top apps</h2>
              </div>
              <a href="#apps">View all</a>
            </div>
            <div class="empty-apps">
              <div class="app-orbit" aria-hidden="true">◎</div>
              <p>Application usage will show up here after monitoring begins.</p>
            </div>
            <div class="privacy-note">
              <span>◇</span><span
                ><strong>Your data stays yours</strong><br />Stored locally. Never sent to a server.</span
              >
            </div>
          </article>
        </section>
      {/if}
      <footer class="page-footer">
        <span>NET WATCH PRO <i>·</i> PRIVATE BY DESIGN</span><span
          >ALL DATA STORED ON THIS DEVICE</span
        >
      </footer>
    </main>
  </div>
</div>

<style>
  :global(*) {
    box-sizing: border-box;
  }
  :global(body) {
    margin: 0;
    background: var(--nwp-background);
    color: var(--nwp-text);
    font-family: var(--nwp-font-ui);
    font-size: var(--nwp-size-body);
  }
  :global(button),
  :global(a) {
    font: inherit;
  }
  .app-shell {
    display: flex;
    min-height: 100vh;
  }
  .sidebar {
    display: flex;
    width: 240px;
    flex: 0 0 240px;
    flex-direction: column;
    padding: var(--nwp-space-6) var(--nwp-space-4);
    border-right: 1px solid var(--nwp-border);
    background: var(--nwp-surface);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
    margin: 0 0 42px;
    color: var(--nwp-text);
    text-decoration: none;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .brand__logo {
    display: block;
    width: 32px;
    height: 32px;
    object-fit: contain;
    flex-shrink: 0;
  }
  nav {
    display: grid;
    gap: var(--nwp-space-2);
  }
  .nav-heading,
  .eyebrow {
    margin: 0 0 var(--nwp-space-2);
    color: var(--nwp-text-disabled);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.08em;
  }
  .nav-heading--lower {
    margin-top: var(--nwp-space-8);
  }
  nav a {
    display: flex;
    min-height: 36px;
    align-items: center;
    gap: var(--nwp-space-3);
    border-radius: var(--nwp-radius-md);
    padding: 0 var(--nwp-space-3);
    color: var(--nwp-text-secondary);
    text-decoration: none;
  }
  nav a:hover {
    background: var(--nwp-surface-2);
    color: var(--nwp-text);
  }
  nav a.active {
    background: var(--nwp-primary-soft);
    color: var(--nwp-primary);
    font-weight: 600;
  }
  nav a:focus-visible,
  .brand:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  .nav-icon {
    display: inline-grid;
    width: 20px;
    place-items: center;
    font-size: 18px;
  }
  .sidebar-foot {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
    margin-top: auto;
    padding: var(--nwp-space-4) var(--nwp-space-2) 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .connection-dot,
  .local-badge span {
    width: 7px;
    height: 7px;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-text-disabled);
  }
  .connection-dot.online,
  .local-badge span {
    background: var(--nwp-success);
  }
  .main-column {
    min-width: 0;
    flex: 1;
  }
  .topbar {
    display: flex;
    height: 72px;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--nwp-border);
    background: var(--nwp-surface);
    padding: 0 var(--nwp-space-8);
  }
  .topbar .eyebrow {
    margin-bottom: 2px;
  }
  h1 {
    margin: 0;
    font-size: var(--nwp-size-title-2);
    font-weight: 600;
  }
  .topbar-actions {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-4);
  }
  .avatar {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface-2);
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-label);
  }
  .content {
    max-width: 1440px;
    margin: auto;
    padding: 36px var(--nwp-space-8) var(--nwp-space-6);
  }
  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-6);
    border: 1px solid var(--nwp-warning);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-warning-soft);
    padding: var(--nwp-space-4);
  }
  .notice strong {
    font-weight: 600;
  }
  .notice p {
    margin: 4px 0 0;
    color: var(--nwp-text-secondary);
  }
  .welcome-row {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-6);
  }
  h2 {
    margin: 0;
    font-size: var(--nwp-size-title-2);
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .welcome-row p {
    margin: var(--nwp-space-2) 0 0;
    color: var(--nwp-text-secondary);
  }
  .local-badge {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface);
    padding: 6px 10px;
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-4);
  }
  .metric-card,
  .panel {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-lg);
    background: var(--nwp-surface);
  }
  .metric-card {
    min-height: 144px;
    padding: var(--nwp-space-5);
  }
  .metric-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.045em;
  }
  .metric-icon {
    display: grid;
    width: 27px;
    height: 27px;
    place-items: center;
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-primary-soft);
    color: var(--nwp-primary);
    font-size: var(--nwp-size-body);
  }
  .metric-card > strong {
    display: block;
    margin-top: var(--nwp-space-3);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-data-lg);
    font-weight: 600;
  }
  .metric-card > p {
    margin: var(--nwp-space-1) 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .lower-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.7fr) minmax(290px, 1fr);
    gap: var(--nwp-space-4);
  }
  .panel {
    min-height: 326px;
    padding: var(--nwp-space-5);
  }
  .panel-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .panel-heading h2 {
    font-size: var(--nwp-size-title-3);
  }
  .panel-heading a {
    color: var(--nwp-primary);
    text-decoration: none;
    font-size: var(--nwp-size-label);
  }
  .panel-heading a:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 3px;
  }
  .period {
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    padding: 7px 9px;
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
  }
  .empty-chart {
    position: relative;
    display: flex;
    min-height: 245px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    text-align: center;
  }
  .chart-glyph,
  .app-orbit {
    display: grid;
    width: 44px;
    height: 44px;
    place-items: center;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface-2);
    color: var(--nwp-primary);
    font-size: 25px;
  }
  .empty-chart h3 {
    margin: var(--nwp-space-3) 0 var(--nwp-space-1);
    font-size: var(--nwp-size-body);
    font-weight: 600;
  }
  .empty-chart p,
  .empty-apps p {
    max-width: 32ch;
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
    line-height: 1.6;
  }
  .chart-baseline {
    position: absolute;
    right: 0;
    bottom: 12px;
    left: 0;
    height: 48px;
    border-bottom: 1px solid var(--nwp-border);
    background: linear-gradient(
      transparent 24%,
      var(--nwp-border) 25%,
      transparent 26%,
      transparent 49%,
      var(--nwp-border) 50%,
      transparent 51%,
      transparent 74%,
      var(--nwp-border) 75%,
      transparent 76%
    );
    opacity: 0.55;
  }
  .empty-apps {
    display: flex;
    min-height: 188px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--nwp-space-3);
    text-align: center;
  }
  .app-orbit {
    color: var(--nwp-secondary);
    background: var(--nwp-secondary-soft);
  }
  .privacy-note {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
    border-top: 1px solid var(--nwp-border);
    padding-top: var(--nwp-space-4);
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
    line-height: 1.6;
  }
  .privacy-note > span:first-child {
    color: var(--nwp-success);
    font-size: 20px;
  }
  .privacy-note strong {
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
    font-weight: 600;
  }
  .page-footer {
    display: flex;
    justify-content: space-between;
    margin-top: var(--nwp-space-8);
    color: var(--nwp-text-disabled);
    font-family: var(--nwp-font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
  }
  .page-footer i {
    padding: 0 5px;
    font-style: normal;
  }
  @media (min-width: 1600px) {
    .content {
      padding-right: var(--nwp-space-10);
      padding-left: var(--nwp-space-10);
    }
  }
  @media (max-width: 1279px) {
    .sidebar {
      width: 68px;
      flex-basis: 68px;
      align-items: center;
      padding-right: var(--nwp-space-2);
      padding-left: var(--nwp-space-2);
    }
    .brand {
      margin-bottom: 42px;
    }
    .brand > span:last-child,
    nav a > span:last-child,
    .nav-heading,
    .sidebar-foot {
      display: none;
    }
    nav a {
      justify-content: center;
      width: 44px;
      padding: 0;
    }
    .content {
      padding-right: var(--nwp-space-6);
      padding-left: var(--nwp-space-6);
    }
  }
  @media (max-width: 900px) {
    .lower-grid {
      grid-template-columns: 1fr;
    }
    .metrics {
      grid-template-columns: 1fr 1fr;
    }
    .metric-card:last-child {
      grid-column: span 2;
    }
  }
  @media (max-width: 600px) {
    .topbar {
      padding: 0 var(--nwp-space-4);
    }
    .content {
      padding: var(--nwp-space-5) var(--nwp-space-4);
    }
    .metrics {
      grid-template-columns: 1fr;
    }
    .metric-card:last-child {
      grid-column: auto;
    }
    .welcome-row {
      align-items: start;
      flex-direction: column;
    }
    .page-footer {
      gap: var(--nwp-space-2);
      flex-direction: column;
    }
  }
</style>
