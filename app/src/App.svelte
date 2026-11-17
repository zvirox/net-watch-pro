<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui';
  import StatusChip from '$lib/components/status-chip/StatusChip.svelte';
  import Live from '$lib/screens/live/Live.svelte';
  import History from '$lib/screens/history/History.svelte';
  import Dashboard from '$lib/screens/dashboard/Dashboard.svelte';
  import AppDetail from '$lib/screens/app-detail/AppDetail.svelte';
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { appDetailStore } from '$lib/stores/app-detail.svelte';

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
  let selectedAppId = $state<number | null>(null);
  const navRoute = $derived(activeRoute === 'app-detail' ? 'apps' : activeRoute);
  const routeLabel = $derived(
    activeRoute === 'app-detail'
      ? (appDetailStore.detail?.name ?? 'App detail')
      : (routes.find((route) => route.key === activeRoute)?.label ?? 'Dashboard'),
  );

  onMount(() => {
    const unlisten = daemonStore.onConnection((state) => {
      connected = state === 'connected';
    });
    const updateRoute = () => {
      const raw = window.location.hash.slice(1);
      const [route = '', argument] = raw.split('/');
      if (route === 'apps' && argument !== undefined) {
        const id = Number(argument);
        if (Number.isInteger(id) && id > 0) {
          activeRoute = 'app-detail';
          selectedAppId = id;
          return;
        }
      }
      activeRoute = routes.some((item) => item.key === route) ? route : 'dashboard';
      selectedAppId = null;
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
          class:active={navRoute === route.key}
          href="#{route.key}"
          aria-current={navRoute === route.key ? 'page' : undefined}
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
        <p class="eyebrow">
          {activeRoute === 'dashboard'
            ? 'OVERVIEW'
            : activeRoute === 'app-detail'
              ? 'APPS / DETAIL'
              : 'MONITOR'}
        </p>
        <h1>{routeLabel}</h1>
      </div>
      <div class="topbar-actions">
        <StatusChip /><span class="avatar" aria-label="Local user">L</span>
      </div>
    </header>
    <main class="content">
      {#if activeRoute !== 'dashboard' && activeRoute !== 'live' && activeRoute !== 'history' && activeRoute !== 'app-detail'}
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
        <Dashboard />
      {/if}
      {#if activeRoute === 'app-detail' && selectedAppId !== null}
        <AppDetail appId={selectedAppId} />
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
  .connection-dot {
    width: 7px;
    height: 7px;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-text-disabled);
  }
  .connection-dot.online {
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
  @media (max-width: 600px) {
    .topbar {
      padding: 0 var(--nwp-space-4);
    }
    .content {
      padding: var(--nwp-space-5) var(--nwp-space-4);
    }
    .page-footer {
      gap: var(--nwp-space-2);
      flex-direction: column;
    }
  }
</style>
