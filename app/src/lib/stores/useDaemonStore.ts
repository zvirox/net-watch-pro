import { create } from 'zustand';
import { isTauri } from '@tauri-apps/api/core';
import { DaemonApiClient } from '$lib/api/client';
import { enableMonitoring } from '$lib/api/transport';
import {
  DEMO_INTERFACES,
  DEMO_ROWS,
  DEMO_SUMMARY,
  DEMO_TIMELINE,
  type HistoryRow,
  type InterfaceUsage,
  type Summary,
  type TimelinePoint,
} from '$lib/fixtures/demo';
import { onRatesFrame, ratesStore, tickRatesNow, type RateSnapshot } from './rates';
import { holdOrder, type LiveAppRow } from './live-apps';
import { licenseStore } from './license';

export type DaemonState = 'monitoring' | 'degraded' | 'stopped' | 'starting';
export type EbpfState = 'attached' | 'failed' | 'unsupported' | 'unattached' | 'no_collector';
export type ConnectionState =
  'disconnected' | 'connecting' | 'connected' | 'auth-required' | 'auth-invalid';

export type ScreenId =
  'dashboard' | 'live' | 'history' | 'apps' | 'app-detail' | 'diagnostics' | 'settings';

export type RangePreset = '1h' | '24h' | '7d' | '30d' | 'cycle';

export interface LiveConnectionRow {
  id: number;
  app: { id: number; name: string; icon: string | null };
  pid: number;
  process_name?: string | null;
  protocol: string;
  local_ip: string;
  local_port: number;
  remote_ip: string;
  remote_port: number;
  remote_host?: string | null;
  rate_up_bps: number;
  rate_down_bps: number;
  bytes_up: number;
  bytes_down: number;
}

export interface AppUsageRow {
  id: number;
  name: string;
  icon: string | null;
  attribution_confidence: string;
  first_seen_at: number;
  last_seen_at: number;
  usage: {
    today: { up: number; down: number };
    month: { up: number; down: number };
    range: { up: number; down: number };
  };
}

export interface AppDetailData extends AppUsageRow {
  exe_path: string;
  deleted_at: number | null;
}

export interface EndpointRow {
  remote_ip: string;
  remote_host: string | null;
  port: number;
  bytes_total: number;
  connections: number;
  last_seen_at: number;
}

export interface DiagnosticsPayload {
  checks: Array<{
    name: string;
    result: 'pass' | 'fail' | 'skip';
    detail?: string;
    guidance_url?: string;
  }>;
  journal_excerpt: Array<{
    occurred_at: number;
    level: 'info' | 'warn' | 'error';
    code: string;
    message: string;
  }>;
  data: {
    db_size_bytes: number;
    retention_days: number;
  };
  rejected_connections: {
    count: number;
    last_rejected_uid: number | null;
  };
}

export interface RateHistorySample {
  ts: number;
  down_bps: number;
  up_bps: number;
}

const DEMO_RATE_SERIES: RateHistorySample[] = Array.from({ length: 24 }, (_, idx) => {
  const wave = Math.sin((idx / 24) * Math.PI * 2) * 0.4 + 0.6;
  return {
    ts: Math.floor(Date.now() / 1000) - (23 - idx) * 60,
    down_bps: Math.round(18_500_000 * wave),
    up_bps: Math.round(3_200_000 * wave),
  };
});

const DEMO_APPS_LIST: AppUsageRow[] = [
  {
    id: 1,
    name: 'Firefox',
    icon: null,
    attribution_confidence: 'exact',
    first_seen_at: Math.floor(Date.now() / 1000) - 86400 * 14,
    last_seen_at: Math.floor(Date.now() / 1000) - 30,
    usage: {
      today: { up: 40_000_000, down: 560_000_000 },
      month: { up: 840_000_000, down: 12_400_000_000 },
      range: { up: 40_000_000, down: 560_000_000 },
    },
  },
  {
    id: 2,
    name: 'Chrome',
    icon: null,
    attribution_confidence: 'exact',
    first_seen_at: Math.floor(Date.now() / 1000) - 86400 * 10,
    last_seen_at: Math.floor(Date.now() / 1000) - 120,
    usage: {
      today: { up: 30_000_000, down: 270_000_000 },
      month: { up: 420_000_000, down: 4_100_000_000 },
      range: { up: 30_000_000, down: 270_000_000 },
    },
  },
  {
    id: 3,
    name: 'Signal',
    icon: null,
    attribution_confidence: 'exact',
    first_seen_at: Math.floor(Date.now() / 1000) - 86400 * 30,
    last_seen_at: Math.floor(Date.now() / 1000) - 300,
    usage: {
      today: { up: 20_000_000, down: 130_000_000 },
      month: { up: 120_000_000, down: 1_300_000_000 },
      range: { up: 20_000_000, down: 130_000_000 },
    },
  },
  {
    id: 4,
    name: 'systemd-resolved',
    icon: null,
    attribution_confidence: 'heuristic',
    first_seen_at: Math.floor(Date.now() / 1000) - 86400 * 30,
    last_seen_at: Math.floor(Date.now() / 1000) - 5,
    usage: {
      today: { up: 12_000_000, down: 18_000_000 },
      month: { up: 45_000_000, down: 68_000_000 },
      range: { up: 12_000_000, down: 18_000_000 },
    },
  },
];

const DEMO_LIVE_CONNECTIONS: LiveConnectionRow[] = [
  {
    id: 1,
    app: { id: 1, name: 'Firefox', icon: null },
    pid: 2481,
    process_name: 'firefox',
    protocol: 'tcp',
    local_ip: '192.168.1.24',
    local_port: 52310,
    remote_ip: '142.250.190.46',
    remote_port: 443,
    remote_host: 'cdn.mozilla.net',
    rate_up_bps: 360_000,
    rate_down_bps: 9_600_000,
    bytes_up: 1_800_000,
    bytes_down: 42_000_000,
  },
  {
    id: 2,
    app: { id: 2, name: 'Chrome', icon: null },
    pid: 3120,
    process_name: 'chrome',
    protocol: 'udp',
    local_ip: '192.168.1.24',
    local_port: 51190,
    remote_ip: '162.159.130.233',
    remote_port: 443,
    remote_host: 'meet.google.com',
    rate_up_bps: 144_000,
    rate_down_bps: 1_920_000,
    bytes_up: 820_000,
    bytes_down: 12_400_000,
  },
  {
    id: 3,
    app: { id: 3, name: 'Signal', icon: null },
    pid: 4892,
    process_name: 'signal-desktop',
    protocol: 'tcp',
    local_ip: '192.168.1.24',
    local_port: 48210,
    remote_ip: '76.223.92.165',
    remote_port: 443,
    remote_host: 'chat.signal.org',
    rate_up_bps: 32_000,
    rate_down_bps: 192_000,
    bytes_up: 210_000,
    bytes_down: 3_100_000,
  },
];

const DEMO_DIAGNOSTICS: DiagnosticsPayload = {
  checks: [
    { name: 'service_unit', result: 'skip', detail: 'Sample preview without local systemd' },
    { name: 'kernel', result: 'skip', detail: 'CO-RE BTF check runs inside netwatchd' },
    { name: 'ebpf_attach', result: 'pass', detail: 'cgroup_skb attached (sample)' },
    { name: 'database', result: 'pass', detail: 'SQLite WAL writable, 14.2 MB' },
  ],
  journal_excerpt: [
    {
      occurred_at: Math.floor(Date.now() / 1000) - 120,
      level: 'info',
      code: 'DAEMON_READY',
      message: 'Sample telemetry stream initialized in local demo mode',
    },
    {
      occurred_at: Math.floor(Date.now() / 1000) - 3600,
      level: 'warn',
      code: 'EBPF_GSO_NOTICE',
      message: 'Egress accounting reflects socket buffer length prior to NIC segmentation',
    },
  ],
  data: { db_size_bytes: 14_889_779, retention_days: 90 },
  rejected_connections: { count: 0, last_rejected_uid: null },
};

export const daemonClient = new DaemonApiClient();

interface DaemonStoreState {
  connection: ConnectionState;
  daemon: DaemonState;
  ebpf: EbpfState;
  recording: boolean;
  demoMode: boolean;
  onboardingDismissed: boolean;
  enablingMonitoring: boolean;
  enableError: string | null;

  activeScreen: ScreenId;
  selectedAppId: number | null;
  sidebarCollapsed: boolean;
  upgradeOpen: boolean;

  rangePreset: RangePreset;
  speedUnit: 'mbps' | 'mbytes';
  isPro: boolean;

  summary: Summary | null;
  timeline: TimelinePoint[];
  interfaces: InterfaceUsage[];
  rateHistory: RateHistorySample[];
  latestRates: RateSnapshot | null;
  ratesStale: boolean;

  liveConnections: LiveConnectionRow[];
  liveApps: LiveAppRow[];
  appsList: AppUsageRow[];
  historyRows: HistoryRow[];
  historyHasMore: boolean;
  historyCursor: string | null;
  historyLoading: boolean;
  historyError: string | null;

  diagnostics: DiagnosticsPayload | null;
  diagnosticsLoading: boolean;
  diagnosticsError: string | null;
  settingsMap: Record<string, unknown>;
  settingsError: string | null;

  setActiveScreen: (screen: ScreenId) => void;
  openAppDetail: (appId: number) => void;
  toggleSidebar: () => void;
  openUpgrade: () => void;
  closeUpgrade: () => void;
  setRangePreset: (preset: RangePreset) => void;
  startDemo: () => void;
  exitDemo: () => void;
  triggerEnableMonitoring: () => Promise<void>;
  refreshDashboard: () => Promise<void>;
  refreshApps: () => Promise<void>;
  refreshHistory: (params?: Record<string, string | number>) => Promise<void>;
  refreshDiagnostics: () => Promise<void>;
  refreshSettings: () => Promise<void>;
  updateSetting: (key: string, value: unknown) => Promise<void>;
  toggleInterfaceFlag: (
    id: number,
    field: 'is_metered' | 'is_excluded',
    value: boolean,
  ) => Promise<void>;
  activateLicenseKey: (key: string) => Promise<string | null>;
  deactivateLicenseKey: () => Promise<string | null>;
  start: () => void;
  stop: () => void;
}

export const useDaemonStore = create<DaemonStoreState>((set, get) => {
  let started = false;
  let listenersBound = false;
  let staleTimer: ReturnType<typeof setInterval> | null = null;

  const bindListeners = () => {
    if (listenersBound) return;
    listenersBound = true;

    daemonClient.onState((state) => {
      set((prev) => ({
        connection: state,
        daemon:
          state === 'connected' ? prev.daemon : state === 'connecting' ? 'starting' : 'stopped',
        ebpf: state === 'connected' ? prev.ebpf : 'no_collector',
        onboardingDismissed: state === 'connected' ? true : prev.onboardingDismissed,
      }));

      if (state === 'connected') {
        void daemonClient.send({ op: 'subscribe', channel: 'net.rates' }).catch(() => {});
        void daemonClient.send({ op: 'subscribe', channel: 'live.apps' }).catch(() => {});
        void daemonClient.send({ op: 'subscribe', channel: 'live.connections' }).catch(() => {});
        void daemonClient.send({ op: 'subscribe', channel: 'usage.totals' }).catch(() => {});
        void get().refreshDashboard();
        void get().refreshSettings();
      } else if (
        !isTauri() &&
        state === 'auth-required' &&
        !get().demoMode &&
        !get().onboardingDismissed
      ) {
        // Browser preview defaults smoothly to Demo mode (Exit demo still reveals Onboarding)
        get().startDemo();
      }
    });

    daemonClient.onFrame((frame: unknown) => {
      if (!frame || typeof frame !== 'object') return;
      const f = frame as { channel?: string; kind?: string; data?: unknown };
      if (f.channel === 'status' && f.data && typeof f.data === 'object') {
        const d = f.data as { daemon?: DaemonState; ebpf?: EbpfState; recording?: boolean };
        set((prev) => ({
          daemon: d.daemon ?? prev.daemon,
          ebpf: d.ebpf ?? prev.ebpf,
          recording: d.recording ?? prev.recording,
        }));
      } else if (f.channel === 'net.rates' && f.data) {
        onRatesFrame(f);
        const snap = f.data as RateSnapshot;
        set((prev) => ({
          latestRates: snap,
          ratesStale: false,
          rateHistory: [
            ...prev.rateHistory.slice(-59),
            { ts: snap.ts, down_bps: snap.down_bps, up_bps: snap.up_bps },
          ],
        }));
      } else if (f.channel === 'live.apps' && f.data && typeof f.data === 'object') {
        const d = f.data as { apps?: LiveAppRow[] };
        if (Array.isArray(d.apps)) {
          set({ liveApps: holdOrder(d.apps) });
        }
      } else if (f.channel === 'live.connections' && f.data && typeof f.data === 'object') {
        const d = f.data as { rows?: LiveConnectionRow[]; data?: LiveConnectionRow[] };
        const list = d.rows ?? d.data;
        if (Array.isArray(list)) {
          set({ liveConnections: list });
        }
      }
    });
  };

  return {
    connection: 'disconnected',
    daemon: 'stopped',
    ebpf: 'no_collector',
    recording: false,
    demoMode: false,
    onboardingDismissed: false,
    enablingMonitoring: false,
    enableError: null,

    activeScreen: 'dashboard',
    selectedAppId: null,
    sidebarCollapsed: false,
    upgradeOpen: false,

    rangePreset: '24h',
    speedUnit: 'mbps',
    isPro: licenseStore.isPro,

    summary: null,
    timeline: [],
    interfaces: [],
    rateHistory: [],
    latestRates: null,
    ratesStale: false,

    liveConnections: [],
    liveApps: [],
    appsList: [],
    historyRows: [],
    historyHasMore: false,
    historyCursor: null,
    historyLoading: false,
    historyError: null,

    diagnostics: null,
    diagnosticsLoading: false,
    diagnosticsError: null,
    settingsMap: { speed_unit: 'mbps', retention_days: 90, range_default: '24h' },
    settingsError: null,

    setActiveScreen: (screen) => set({ activeScreen: screen }),
    openAppDetail: (appId) => set({ selectedAppId: appId, activeScreen: 'app-detail' }),
    toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
    openUpgrade: () => {
      licenseStore.openUpgrade();
      set({ upgradeOpen: true });
    },
    closeUpgrade: () => {
      licenseStore.closeUpgrade();
      set({ upgradeOpen: false });
    },
    setRangePreset: (preset) => {
      set({ rangePreset: preset });
      void get().refreshDashboard();
      void get().refreshApps();
    },

    startDemo: () => {
      set({
        demoMode: true,
        onboardingDismissed: true,
        summary: DEMO_SUMMARY,
        timeline: DEMO_TIMELINE,
        interfaces: DEMO_INTERFACES,
        rateHistory: DEMO_RATE_SERIES,
        latestRates: {
          ts: Math.floor(Date.now() / 1000),
          down_bps: 14_400_000,
          up_bps: 2_600_000,
        },
        ratesStale: false,
        liveConnections: DEMO_LIVE_CONNECTIONS,
        appsList: DEMO_APPS_LIST,
        historyRows: DEMO_ROWS,
        historyHasMore: false,
        diagnostics: DEMO_DIAGNOSTICS,
      });
    },

    exitDemo: () => {
      set({
        demoMode: false,
        onboardingDismissed: false,
        summary: null,
        timeline: [],
        interfaces: [],
        liveConnections: [],
        appsList: [],
        historyRows: [],
      });
    },

    triggerEnableMonitoring: async () => {
      set({ enablingMonitoring: true, enableError: null });
      try {
        await enableMonitoring();
        await daemonClient.authorize();
        set({ enablingMonitoring: false, onboardingDismissed: true, demoMode: false });
      } catch (err) {
        const msg = err instanceof Error ? err.message : 'Monitoring could not be enabled.';
        set({ enablingMonitoring: false, enableError: msg });
      }
    },

    refreshDashboard: async () => {
      if (get().demoMode || get().connection !== 'connected') return;
      const preset = get().rangePreset;
      try {
        const [summary, timelineRes, interfacesRes, ratesRes] = await Promise.all([
          daemonClient.request('GET', `/api/v1/usage/summary?range=${preset}&top_limit=8`),
          daemonClient.request('GET', `/api/v1/usage/timeline?range=${preset}`),
          daemonClient.request('GET', `/api/v1/usage/interfaces?range=${preset}`),
          daemonClient.request('GET', '/api/v1/usage/rate-history'),
        ]);
        const tl = timelineRes as { data?: TimelinePoint[] };
        const rh = ratesRes as { samples?: RateHistorySample[] };
        set({
          summary: summary as Summary,
          timeline: Array.isArray(tl.data) ? tl.data : [],
          interfaces: Array.isArray(interfacesRes) ? (interfacesRes as InterfaceUsage[]) : [],
          rateHistory: Array.isArray(rh.samples) ? rh.samples : [],
        });
      } catch {
        // Keep last known data cleanly
      }
    },

    refreshApps: async () => {
      if (get().demoMode || get().connection !== 'connected') return;
      const preset = get().rangePreset;
      try {
        const rows = await daemonClient.request(
          'GET',
          `/api/v1/apps?sort=usage_range&range=${preset}`,
        );
        if (Array.isArray(rows)) set({ appsList: rows as AppUsageRow[] });
      } catch {
        // Silent retain
      }
    },

    refreshHistory: async (params = {}) => {
      if (get().demoMode || get().connection !== 'connected') {
        set({ historyRows: DEMO_ROWS, historyHasMore: false, historyError: null });
        return;
      }
      set({ historyLoading: true, historyError: null });
      try {
        const q = new URLSearchParams();
        Object.entries(params).forEach(([k, v]) => {
          if (v !== '' && v !== undefined) q.set(k, String(v));
        });
        const suffix = q.toString() ? `?${q.toString()}` : '';
        const res = (await daemonClient.request('GET', `/api/v1/connections${suffix}`)) as {
          data?: HistoryRow[];
          has_more?: boolean;
          next_cursor?: string | null;
        };
        set({
          historyRows: Array.isArray(res.data) ? res.data : [],
          historyHasMore: Boolean(res.has_more),
          historyCursor: res.next_cursor ?? null,
          historyLoading: false,
        });
      } catch (err) {
        set({
          historyLoading: false,
          historyError: err instanceof Error ? err.message : 'History could not be loaded',
        });
      }
    },

    refreshDiagnostics: async () => {
      if (get().demoMode || get().connection !== 'connected') {
        set({ diagnostics: DEMO_DIAGNOSTICS, diagnosticsError: null });
        return;
      }
      set({ diagnosticsLoading: true, diagnosticsError: null });
      try {
        const res = await daemonClient.request('GET', '/api/v1/diagnostics');
        set({ diagnostics: res as DiagnosticsPayload, diagnosticsLoading: false });
      } catch (err) {
        set({
          diagnosticsLoading: false,
          diagnosticsError: err instanceof Error ? err.message : 'Diagnostics unavailable',
        });
      }
    },

    refreshSettings: async () => {
      if (get().demoMode || get().connection !== 'connected') return;
      try {
        const res = await daemonClient.request('GET', '/api/v1/settings');
        const map: Record<string, unknown> = {};
        if (Array.isArray(res)) {
          for (const row of res as Array<{ key?: string; value?: unknown }>) {
            if (row.key) map[row.key] = row.value;
          }
        }
        const unit = map.speed_unit === 'mbytes' ? 'mbytes' : 'mbps';
        const rng = (map.range_default as RangePreset) || '24h';
        set({ settingsMap: map, speedUnit: unit, rangePreset: rng, settingsError: null });
      } catch (err) {
        set({ settingsError: err instanceof Error ? err.message : 'Could not read settings' });
      }
    },

    updateSetting: async (key, value) => {
      if (get().demoMode || get().connection !== 'connected') {
        set((s) => ({
          settingsMap: { ...s.settingsMap, [key]: value },
          speedUnit:
            key === 'speed_unit' && value === 'mbytes'
              ? 'mbytes'
              : key === 'speed_unit'
                ? 'mbps'
                : s.speedUnit,
        }));
        return;
      }
      try {
        await daemonClient.request('PUT', `/api/v1/settings/${key}`, { value });
        await get().refreshSettings();
      } catch (err) {
        set({ settingsError: err instanceof Error ? err.message : 'Failed to save setting' });
      }
    },

    toggleInterfaceFlag: async (id, field, value) => {
      if (get().demoMode || get().connection !== 'connected') {
        set((s) => ({
          interfaces: s.interfaces.map((iface) =>
            iface.id === id ? { ...iface, [field]: value } : iface,
          ),
        }));
        return;
      }
      await daemonClient.request('PATCH', `/api/v1/interfaces/${String(id)}`, { [field]: value });
      await get().refreshDashboard();
    },

    activateLicenseKey: async (key) => {
      if (get().demoMode || get().connection !== 'connected') {
        if (!key.trim().startsWith('NWP1-')) {
          return 'Invalid key format (expected NWP1-...)';
        }
        licenseStore.isPro = true;
        set({ isPro: true, upgradeOpen: false });
        return null;
      }
      try {
        await daemonClient.request('POST', '/api/v1/license/activate', { key: key.trim() });
        licenseStore.isPro = true;
        set({ isPro: true, upgradeOpen: false });
        return null;
      } catch (err) {
        return err instanceof Error ? err.message : 'License activation failed';
      }
    },

    deactivateLicenseKey: async () => {
      if (get().demoMode || get().connection !== 'connected') {
        licenseStore.isPro = false;
        set({ isPro: false });
        return null;
      }
      try {
        await daemonClient.request('DELETE', '/api/v1/license');
        licenseStore.isPro = false;
        set({ isPro: false });
        return null;
      } catch (err) {
        return err instanceof Error ? err.message : 'Could not deactivate license';
      }
    },

    start: () => {
      if (started) return;
      started = true;
      bindListeners();
      void daemonClient.start();
      if (!staleTimer) {
        staleTimer = setInterval(() => {
          tickRatesNow();
          set({ ratesStale: ratesStore.stale });
        }, 1000);
      }
    },

    stop: () => {
      started = false;
      if (staleTimer) {
        clearInterval(staleTimer);
        staleTimer = null;
      }
      void daemonClient.stop();
    },
  };
});
