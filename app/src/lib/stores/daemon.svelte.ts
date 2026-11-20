import { DaemonApiClient } from '$lib/api/client';

type DaemonState = 'monitoring' | 'degraded' | 'stopped' | 'starting';
type EbpfState = 'attached' | 'failed' | 'unsupported' | 'unattached' | 'no_collector';
type ConnectionState =
  'disconnected' | 'connecting' | 'connected' | 'auth-required' | 'auth-invalid';
type Listener = (state: ConnectionState) => void;
// Callback registries are not rendered state, so Svelte reactivity is unnecessary.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const connectionListeners = new Set<Listener>();
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const frameListeners = new Set<FrameListener>();
type FrameListener = (frame: unknown) => void;
type StatusFrame = {
  channel: 'status';
  kind: 'update';
  data: {
    daemon: DaemonState;
    ebpf: EbpfState;
    recording: boolean;
  };
};

const client = new DaemonApiClient();
let connection = $state<ConnectionState>('disconnected');
let daemon = $state<DaemonState>('stopped');
// Honest default: no collector is known until a status frame arrives.
let ebpf = $state<EbpfState>('no_collector');
let started = false;

client.onState((state) => {
  connection = state;
  connectionListeners.forEach((listener) => {
    listener(state);
  });
  if (state === 'connected') {
    return;
  }
  daemon = state === 'connecting' || state === 'disconnected' ? 'starting' : 'stopped';
  ebpf = 'no_collector';
});

client.onFrame((frame) => {
  // Forward every frame to store listeners (live.svelte, usage.svelte
  // filter by channel — §6.1); status handling stays here.
  frameListeners.forEach((listener) => {
    listener(frame);
  });
  if (!isStatusFrame(frame)) return;
  daemon = frame.data.daemon;
  ebpf = frame.data.ebpf;
});

export const daemonStore = {
  get connection(): ConnectionState {
    return connection;
  },
  get daemon(): DaemonState {
    return daemon;
  },
  get ebpf(): EbpfState {
    return ebpf;
  },
  /** Chip state: no collector means Degraded, never green (DESIGN.md §7.5). */
  get chipState(): DaemonState {
    if (daemon === 'monitoring' && (ebpf === 'no_collector' || ebpf === 'unattached')) {
      return 'degraded';
    }
    return daemon;
  },
  get recording(): boolean {
    return daemon === 'monitoring' || daemon === 'degraded';
  },
  onConnection(listener: Listener): () => void {
    connectionListeners.add(listener);
    listener(connection);
    return () => {
      connectionListeners.delete(listener);
    };
  },
  onFrame(listener: FrameListener): () => void {
    frameListeners.add(listener);
    return () => {
      frameListeners.delete(listener);
    };
  },
  subscribe(
    channel: 'status' | 'live.connections' | 'usage.totals' | 'net.rates' | 'live.apps',
  ): Promise<void> {
    return client.send({ op: 'subscribe', channel });
  },
  request(method: string, path: string, body?: unknown): Promise<unknown> {
    return client.request(method, path, body);
  },
  /** Raw response body for non-JSON content (export downloads). */
  requestText(method: string, path: string): Promise<string> {
    return client.requestText(method, path);
  },
  send(frame: unknown): Promise<void> {
    return client.send(frame);
  },
  async start(): Promise<void> {
    if (started) return;
    started = true;
    await client.start();
  },
  async authorize(): Promise<void> {
    await client.authorize();
  },
  async stop(): Promise<void> {
    started = false;
    await client.stop();
  },
};

function isStatusFrame(value: unknown): value is StatusFrame {
  if (typeof value !== 'object' || value === null || !('channel' in value) || !('data' in value)) {
    return false;
  }
  if (value.channel !== 'status' || typeof value.data !== 'object' || value.data === null)
    return false;
  return (
    'daemon' in value.data &&
    ['monitoring', 'degraded', 'stopped', 'starting'].includes(String(value.data.daemon))
  );
}
