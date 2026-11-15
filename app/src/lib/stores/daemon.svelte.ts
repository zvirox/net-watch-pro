import { DaemonApiClient } from '$lib/api/client';

type DaemonState = 'monitoring' | 'degraded' | 'stopped' | 'starting';
type ConnectionState =
  'disconnected' | 'connecting' | 'connected' | 'auth-required' | 'auth-invalid';
type Listener = (state: ConnectionState) => void;
// Callback registry is not rendered state, so Svelte reactivity is unnecessary.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const connectionListeners = new Set<Listener>();
type StatusFrame = {
  channel: 'status';
  kind: 'update';
  data: {
    daemon: DaemonState;
    ebpf: string;
    recording: boolean;
  };
};

const client = new DaemonApiClient();
let connection = $state<ConnectionState>('disconnected');
let daemon = $state<DaemonState>('stopped');
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
});

client.onFrame((frame) => {
  if (!isStatusFrame(frame)) return;
  daemon = frame.data.daemon;
});

export const daemonStore = {
  get connection(): ConnectionState {
    return connection;
  },
  get daemon(): DaemonState {
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
  subscribe(channel: 'status' | 'live.connections' | 'usage.totals'): Promise<void> {
    return client.send({ op: 'subscribe', channel });
  },
  request(method: string, path: string, body?: unknown): Promise<unknown> {
    return client.request(method, path, body);
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
