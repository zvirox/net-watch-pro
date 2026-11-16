<script lang="ts">
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { Button } from '$lib/components/ui';

  let connected = $state(daemonStore.connection === 'connected');

  // Subscribe to live.connections channel
  let rows: Array<{
    id: number;
    app: string;
    pid: number;
    protocol: string;
    local_ip: string;
    local_port: number;
    remote_ip: string;
    remote_port: number;
    rate_up_bps: number;
    rate_down_bps: number;
    bytes_up: number;
    bytes_down: number;
  }> = [];

  // Subscribe to live.connections channel
  void daemonStore.subscribe('live.connections');

  // Listen for frame updates from the daemon store
  // Note: In a full implementation, daemonStore.onFrame would be used here
  // to receive live.connection delta frames. For Phase 2 skeleton, we show
  // the connection state and empty state until real data flows.

  const columns = [
    { key: 'app', label: 'App' },
    { key: 'pid', label: 'PID' },
    { key: 'protocol', label: 'Proto' },
    { key: 'remote', label: 'Remote' },
    { key: 'rate', label: 'Rate' },
    { key: 'bytes', label: 'Bytes' }
  ];

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatRate(bps: number): string {
    return formatBytes(bps) + '/s';
  }
</script>

<svelte:head>
  <title>Live Connections · Net Watch Pro</title>
</svelte:head>

<div class="app-shell">
  <aside class="sidebar" aria-label="Main navigation">
    <a class="brand" href="#dashboard" aria-label="Net Watch Pro home"
      ><span class="brand__mark">NW</span><span>Net Watch Pro</span></a
    >
    <nav>
      <p class="nav-heading">MONITOR</p>
      <a
        class:active={activeRoute === 'dashboard'}
        href="#dashboard"
        aria-current={activeRoute === 'dashboard' ? 'page' : undefined}
        onclick={() => (activeRoute = 'dashboard')}
      >
        <span class="nav-icon" aria-hidden="true">{navigationIcons[0]}</span>
        <span>Dashboard</span>
      </a>
      <a
        class:active={activeRoute === 'live'}
        href="#live"
        aria-current={activeRoute === 'live' ? 'page' : undefined}
        onclick={() => (activeRoute = 'live')}
      >
        <span class="nav-icon" aria-hidden="true">{navigationIcons[1]}</span>
        <span>Live</span>
      </a>
      <a
        class:active={activeRoute === 'apps'}
        href="#apps"
        aria-current={activeRoute === 'apps' ? 'page' : undefined}
        onclick={() => (activeRoute = 'apps')}
      >
        <span class="nav-icon" aria-hidden="true">{navigationIcons[2]}</span>
        <span>Apps</span>
      </a>
      <a
        class:active={activeRoute === 'history'}
        href="#history"
        aria-current={activeRoute === 'history' ? 'page' : undefined}
        onclick={() => (activeRoute = 'history')}
      >
        <span class="nav-icon" aria-hidden="true">{navigationIcons[3]}</span>
        <span>History</span>
      </a>
      <a
        class:active={activeRoute === 'settings'}
        href="#settings"
        aria-current={activeRoute === 'settings' ? 'page' : undefined}
        onclick={() => (activeRoute = 'settings')}
      >
        <span class="nav-icon" aria-hidden="true">{navigationIcons[4]}</span>
        <span>Settings</span>
      </a>
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
        <p class="eyebrow">MONITOR</p>
        <h1>Live Connections</h1>
      </div>
      <div class="topbar-actions">
        <StatusChip /><span class="avatar" aria-label="Local user">L</span>
      </div>
    </header>
    <main class="content">
      {#if daemonStore.connection === 'auth-required' || daemonStore.connection === 'auth-invalid'}
        <section class="notice" role="status">
          <div>
            <strong>Authorization needed</strong>
            <p>Authenticate locally to connect to the Net Watch Pro daemon.</p>
          </div>
          <Button variant="primary" onclick={() => void daemonStore.authorize()}>Authorize</Button>
        </section>
      {/if}
      {#if connected}
        <section class="panel">
          <div class="panel-heading">
            <div>
              <p class="eyebrow">LIVE CONNECTIONS</p>
              <h2>Active Connections</h2>
            </div>
            <span class="period">LIVE⌄</span>
          </div>
          <div class="empty-chart" style="min-height: 400px;">
            <div class="chart-glyph" aria-hidden="true">⌁</div>
            <h3>No active connections</h3>
            <p>No connections are currently being monitored. Start browsing to see live data.</p>
          </div>
        </section>
      {:else if daemonStore.connection === 'connecting'}
        <section class="notice" role="status">
          <div>
            <strong>Connecting…</strong>
            <p>Connecting to the Net Watch Pro daemon. Please wait.</p>
          </div>
        </section>
      {:else if daemonStore.connection === 'auth-required' || daemonStore.connection === 'auth-invalid'}
        <section class="notice" role="status">
          <div>
            <strong>Authorization required</strong>
            <p>Authenticate to connect to the daemon.</p>
          </div>
          <Button variant="primary" onclick={() => void daemonStore.authorize()}>Authorize</Button>
        </section>
      {:else}
        <section class="notice" role="status">
          <div>
            <strong>Daemon stopped</strong>
            <p>The daemon is not running. Start the daemon to view live connections.</p>
          </div>
        </section>
      {/if}
    </main>
    <footer class="page-footer">
      <span>NET WATCH PRO <i>·</i> PRIVATE BY DESIGN</span><span
        >ALL DATA STORED ON THIS DEVICE</span
      >
    </footer>
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
  .brand__mark {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-primary);
    color: var(--nwp-on-primary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-label);
    font-weight: 700;
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
    height: 20px;
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
    border: 1px solid var(--nwp-border);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-surface);
    padding: var(--nwp-space-4);
  }
  .notice strong {
    font-weight: 600;
  }
  .notice p {
    margin: 4px 0 0;
    color: var(--nwp-text-secondary);
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
</style>