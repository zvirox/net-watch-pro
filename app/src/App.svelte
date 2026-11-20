<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui';
  import StatusChip from '$lib/components/status-chip/StatusChip.svelte';
  import Live from '$lib/screens/live/Live.svelte';
  import Apps from '$lib/screens/apps/Apps.svelte';
  import History from '$lib/screens/history/History.svelte';
  import Dashboard from '$lib/screens/dashboard/Dashboard.svelte';
  import AppDetail from '$lib/screens/app-detail/AppDetail.svelte';
  import UpgradeModal from '$lib/screens/upgrade/UpgradeModal.svelte';
  import DiagnosticsModal from '$lib/screens/diagnostics/DiagnosticsModal.svelte';
  import ExportMenu from '$lib/screens/history/ExportMenu.svelte';
  import Settings from '$lib/screens/settings/Settings.svelte';
  import Onboarding from '$lib/screens/onboarding/Onboarding.svelte';
  import { daemonStore } from '$lib/stores/daemon.svelte';
  import { appDetailStore } from '$lib/stores/app-detail.svelte';
  import { licenseStore } from '$lib/stores/license.svelte';
  import { onboardingStore } from '$lib/stores/onboarding.svelte';
  import { demoStore } from '$lib/stores/demo.svelte';
  import { diagnosticsStore } from '$lib/stores/diagnostics.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import '$lib/stores/settings-ui.svelte';

  let connected = $state(daemonStore.connection === 'connected');
  const routes = [
    { label: 'Dashboard', key: 'dashboard' },
    { label: 'Live', key: 'live' },
    { label: 'Apps', key: 'apps' },
    { label: 'History', key: 'history' },
    { label: 'Settings', key: 'settings' },
  ];
  // DESIGN.md §6.2: Lucide stroke icons. Inline SVG keeps the zero-dependency
  // promise (no icon package); each entry is a Lucide `path` `d` string.
  const navigationIcons = [
    ['m12 14 4-4', 'M3.34 19a10 10 0 1 1 17.32 0'], // gauge — distinct from the Apps grid
    ['M22 12h-4l-3 9L9 3l-3 9H2'], // activity
    ['M3 3h7v7H3z', 'M14 3h7v7h-7z', 'M14 14h7v7h-7z', 'M3 14h7v7H3z'], // layout-grid
    ['M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8', 'M3 3v5h5', 'M12 7v5l4 2'], // history
    [
      'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z',
      'M12 9a3 3 0 1 0 0 6 3 3 0 1 0 0-6',
    ], // settings
  ];
  const upgradeIcon = ['M2 18h20', 'M2 9l4 3 6-9 6 9 4-3-2 7H4']; // crown
  let activeRoute = $state('dashboard');
  let selectedAppId = $state<number | null>(null);
  const navRoute = $derived(activeRoute === 'app-detail' ? 'apps' : activeRoute);
  const routeLabel = $derived(
    activeRoute === 'app-detail'
      ? (appDetailStore.detail?.name ?? 'App detail')
      : (routes.find((route) => route.key === activeRoute)?.label ?? 'Dashboard'),
  );

  /** §9.3: roving arrow-key navigation across the sidebar links. */
  function onNavKeydown(event: KeyboardEvent, index: number): void {
    const keys = ['ArrowDown', 'ArrowUp', 'ArrowRight', 'ArrowLeft', 'Home', 'End'];
    if (!keys.includes(event.key)) return;
    event.preventDefault();
    const links = document.querySelectorAll<HTMLAnchorElement>('.sidebar nav a');
    let next = index;
    if (event.key === 'ArrowDown' || event.key === 'ArrowRight') next = (index + 1) % links.length;
    if (event.key === 'ArrowUp' || event.key === 'ArrowLeft')
      next = (index - 1 + links.length) % links.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = links.length - 1;
    links[next]?.focus();
  }

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
    onboardingStore.begin();
    // Load persisted settings at app start so the saved theme applies on any
    // screen, not only when Settings is opened.
    void settingsStore.load();
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

{#if onboardingStore.visible}
  <Onboarding />
{:else}
  <div class="app-shell">
    <a class="skip-link" href="#main-content">Skip to content</a>
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
            aria-label={route.label}
            aria-current={navRoute === route.key ? 'page' : undefined}
            onclick={() => (activeRoute = route.key)}
            onkeydown={(event: KeyboardEvent) => {
              onNavKeydown(event, index);
            }}
          >
            <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
              {#each navigationIcons[index] as path, pathIndex (pathIndex)}
                <path d={path} />
              {/each}
            </svg>
            <span>{route.label}</span>
          </a>
        {/each}
        <p class="nav-heading nav-heading--lower">ACCOUNT</p>
        <a
          href="#upgrade"
          aria-label="Upgrade to Pro"
          onclick={(event) => {
            event.preventDefault();
            licenseStore.openUpgrade();
          }}
          onkeydown={(event: KeyboardEvent) => {
            onNavKeydown(event, routes.length);
          }}
        >
          <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
            {#each upgradeIcon as path, index (index)}
              <path d={path} />
            {/each}
          </svg>
          <span class="nav-label">Upgrade</span>
          {#if licenseStore.isPro}<span class="pro-badge">PRO</span>{/if}
        </a>
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
          {#if demoStore.active}
            <span class="demo-badge">
              Demo mode
              <button
                type="button"
                onclick={() => {
                  demoStore.stop();
                }}
              >
                Exit
              </button>
            </span>
          {/if}
          {#if activeRoute === 'history'}
            <ExportMenu />
          {/if}
          <StatusChip /><span class="avatar" aria-label="Local user">L</span>
        </div>
      </header>
      <main class="content" id="main-content" tabindex="-1">
        {#if activeRoute !== 'dashboard' && activeRoute !== 'live' && activeRoute !== 'apps' && activeRoute !== 'history' && activeRoute !== 'app-detail' && activeRoute !== 'settings'}
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
            <Button variant="primary" onclick={() => void daemonStore.authorize()}>Authorize</Button
            >
          </section>
        {/if}
        {#if daemonStore.connection === 'disconnected' || daemonStore.connection === 'connecting'}
          <section class="stopped-banner" role="status" aria-label="Daemon stopped">
            <div>
              <strong>Monitoring stopped — daemon not running.</strong>
              <p>Local-only monitoring resumes as soon as the daemon is linked again.</p>
            </div>
            <div class="banner-actions">
              <Button
                variant="primary"
                size="sm"
                onclick={() => {
                  void diagnosticsStore.startDaemon();
                }}>Start daemon</Button
              >
              <Button
                variant="ghost"
                size="sm"
                onclick={() => {
                  diagnosticsStore.openModal();
                }}>Open diagnostics</Button
              >
            </div>
          </section>
        {/if}
        {#if activeRoute === 'live'}
          <Live />
        {/if}
        {#if activeRoute === 'apps'}
          <Apps />
        {/if}
        {#if activeRoute === 'history'}
          <History />
        {/if}
        {#if activeRoute === 'dashboard'}
          <Dashboard />
        {/if}
        {#if activeRoute === 'settings'}
          <Settings />
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
{/if}

<UpgradeModal />
<DiagnosticsModal />

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
  /* §9.3: skip link is the first focusable element, visible only on focus. */
  .skip-link {
    position: absolute;
    z-index: 10;
    top: var(--nwp-space-2);
    left: var(--nwp-space-2);
    transform: translateY(-200%);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-primary);
    padding: var(--nwp-space-2) var(--nwp-space-4);
    color: var(--nwp-on-primary);
    text-decoration: none;
  }
  .skip-link:focus-visible {
    transform: translateY(0);
    outline: 2px solid var(--nwp-text);
    outline-offset: 2px;
  }
  .app-shell {
    display: flex;
    min-height: 100vh;
  }
  .sidebar {
    display: flex;
    width: var(--nwp-sidebar-width);
    flex: 0 0 var(--nwp-sidebar-width);
    flex-direction: column;
    padding: var(--nwp-space-6) var(--nwp-space-4);
    border-right: 1px solid var(--nwp-border);
    background: var(--nwp-surface);
    transition:
      width var(--nwp-dur-slow) var(--nwp-ease-standard),
      flex-basis var(--nwp-dur-slow) var(--nwp-ease-standard);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-3);
    margin: 0 0 var(--nwp-space-10);
    color: var(--nwp-text);
    text-decoration: none;
    font-weight: 600;
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
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
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
    transition:
      background-color var(--nwp-dur-fast) var(--nwp-ease-standard),
      color var(--nwp-dur-fast) var(--nwp-ease-standard);
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
  /* §7.7: secondary-soft pill marks the Pro entitlement */
  .pro-badge {
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-secondary-soft);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    font-weight: 600;
  }
  nav a:focus-visible,
  .brand:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
  }
  /* §6.2: 20px Lucide stroke icons — 2px stroke, round caps/joins, currentColor. */
  .nav-icon {
    width: 20px;
    height: 20px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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
    height: var(--nwp-topbar-height);
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
  .demo-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--nwp-space-2);
    border-radius: var(--nwp-radius-sm);
    background: var(--nwp-secondary-soft);
    padding: 2px var(--nwp-space-2);
    color: var(--nwp-secondary);
    font-size: var(--nwp-size-label);
    font-weight: 600;
  }
  .demo-badge button {
    border: 0;
    background: transparent;
    color: var(--nwp-secondary);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .demo-badge button:focus-visible {
    outline: 2px solid var(--nwp-secondary);
    outline-offset: 2px;
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
    max-width: var(--nwp-content-max);
    margin: auto;
    padding: var(--nwp-space-8) var(--nwp-space-8) var(--nwp-space-6);
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
  /* §7.9: non-dismissible daemon-stopped banner. */
  .stopped-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--nwp-space-4);
    margin-bottom: var(--nwp-space-6);
    border: 1px solid var(--nwp-warning);
    border-radius: var(--nwp-radius-md);
    background: var(--nwp-warning-soft);
    padding: var(--nwp-space-3) var(--nwp-space-4);
  }
  .stopped-banner strong {
    font-weight: 600;
  }
  .stopped-banner p {
    margin: 4px 0 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-label);
  }
  .banner-actions {
    display: flex;
    flex: none;
    gap: var(--nwp-space-2);
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
    color: var(--nwp-text-secondary);
    font-family: var(--nwp-font-mono);
    font-size: var(--nwp-size-caption);
    letter-spacing: 0.04em;
  }
  .page-footer i {
    padding: 0 var(--nwp-space-1);
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
      width: var(--nwp-sidebar-width-collapsed);
      flex-basis: var(--nwp-sidebar-width-collapsed);
      align-items: center;
      padding-right: var(--nwp-space-2);
      padding-left: var(--nwp-space-2);
    }
    .brand {
      margin-bottom: var(--nwp-space-10);
    }
    .brand > span:last-child,
    nav a > span:last-child,
    .nav-label,
    .pro-badge,
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
