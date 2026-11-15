import { invoke, isTauri } from '@tauri-apps/api/core';
import {
  connectDaemonWebSocket,
  disconnectDaemonWebSocket,
  listenDaemonEvents,
  requestDaemon,
  sendDaemonFrame,
  type DaemonHttpResponse,
} from './transport';

type LinkState = 'disconnected' | 'connecting' | 'connected' | 'auth-required' | 'auth-invalid';
type LinkListener = (state: LinkState) => void;
type FrameListener = (frame: unknown) => void;

const MAX_RETRY_MS = 16_000;

export class DaemonApiClient {
  #token: string | null = null;
  #state: LinkState = 'disconnected';
  #stateListeners = new Set<LinkListener>();
  #frameListeners = new Set<FrameListener>();
  #eventUnlisteners: Array<() => void> = [];
  #retryTimer: ReturnType<typeof setTimeout> | null = null;
  #retryDelayMs = 1_000;
  #generation = 0;
  #invalidTokenRetries = 0;
  #closed = false;

  get state(): LinkState {
    return this.#state;
  }

  onState(listener: LinkListener): () => void {
    this.#stateListeners.add(listener);
    listener(this.#state);
    return () => {
      this.#stateListeners.delete(listener);
    };
  }

  onFrame(listener: FrameListener): () => void {
    this.#frameListeners.add(listener);
    return () => {
      this.#frameListeners.delete(listener);
    };
  }

  async start(): Promise<void> {
    this.#closed = false;
    if (!isTauri()) {
      this.#setState('auth-required');
      return;
    }
    if (!(await this.#installEventListeners())) return;
    await this.#connect();
  }

  async authorize(): Promise<void> {
    this.#closed = false;
    this.#invalidTokenRetries = 0;
    this.#clearRetry();
    if (!isTauri()) {
      this.#setState('auth-required');
      return;
    }
    if (!(await this.#installEventListeners())) return;
    this.#token = null;
    await this.#connect();
  }

  async stop(): Promise<void> {
    this.#closed = true;
    this.#generation += 1;
    this.#clearRetry();
    this.#token = null;
    this.#invalidTokenRetries = 0;
    this.#eventUnlisteners.forEach((unlisten) => {
      unlisten();
    });
    this.#eventUnlisteners = [];
    await disconnectDaemonWebSocket().catch(() => undefined);
    this.#setState('disconnected');
  }

  async request(method: string, path: string, body?: unknown): Promise<unknown> {
    const token = this.#token;
    if (!token) throw new Error('Daemon authorization is required');

    let response: DaemonHttpResponse;
    try {
      response = await requestDaemon(token, method, path, body);
    } catch {
      this.#scheduleReconnect();
      throw new Error('Daemon is unreachable');
    }

    if (response.status === 401) {
      await this.#invalidateToken();
      throw new Error('Daemon authorization expired');
    }
    if (response.status < 200 || response.status >= 300) {
      const envelope = parseJson(response.body);
      const message = isApiErrorEnvelope(envelope)
        ? envelope.error.message
        : `Daemon request failed (HTTP ${String(response.status)})`;
      throw new Error(message);
    }
    const parsed = parseJson(response.body);
    if (parsed === undefined) throw new Error('Daemon returned invalid JSON');
    return parsed;
  }

  async send(frame: unknown): Promise<void> {
    if (this.#state !== 'connected') throw new Error('Daemon WebSocket is disconnected');
    try {
      await sendDaemonFrame(frame);
    } catch {
      this.#scheduleReconnect();
      throw new Error('Daemon WebSocket is disconnected');
    }
  }

  async #installEventListeners(): Promise<boolean> {
    if (this.#eventUnlisteners.length) return true;
    try {
      this.#eventUnlisteners = await listenDaemonEvents({
        state: (state) => {
          this.#handleNativeState(state);
        },
        frame: (frame) => {
          this.#handleFrame(frame);
        },
      });
      return true;
    } catch {
      // Browser preview and isolated UI tests have no Tauri event bridge.
      this.#setState('auth-required');
      return false;
    }
  }

  async #connect(): Promise<void> {
    if (this.#closed || !isTauri()) {
      this.#setState('auth-required');
      return;
    }
    const generation = ++this.#generation;
    this.#setState('connecting');

    let token = this.#token;
    if (!token) {
      try {
        token = await invoke<string>('retrieve_daemon_token');
      } catch {
        if (this.#isCurrent(generation)) {
          this.#setState('auth-required');
          this.#clearRetry();
        }
        return;
      }
      if (!this.#isCurrent(generation)) return;
      if (!isToken(token)) {
        this.#setState('auth-required');
        return;
      }
      this.#token = token;
    }

    try {
      await connectDaemonWebSocket(token);
      if (!this.#isCurrent(generation)) return;
      await sendDaemonFrame({ op: 'subscribe', channel: 'status' });
      this.#retryDelayMs = 1_000;
      this.#clearRetry();
    } catch {
      if (!this.#isCurrent(generation)) return;
      this.#scheduleReconnect();
    }
  }

  #isCurrent(generation: number): boolean {
    return !this.#closed && generation === this.#generation;
  }

  #handleNativeState(state: string): void {
    if (state === 'connected') {
      this.#retryDelayMs = 1_000;
      this.#clearRetry();
      this.#setState('connected');
    } else if (state === 'auth-invalid') {
      void this.#invalidateToken();
    } else if (state === 'auth-required') {
      this.#token = null;
      this.#clearRetry();
      this.#setState('auth-required');
    } else if (state === 'disconnected') {
      if (this.#state === 'auth-invalid' || this.#state === 'auth-required') return;
      void this.#disconnectAfterFailure();
      this.#setState('connecting');
      this.#scheduleReconnect();
    } else if (state === 'connecting' || state === 'error') {
      this.#setState('connecting');
    }
  }

  #handleFrame(raw: string): void {
    const frame = parseJson(raw);
    if (frame !== undefined) {
      // Any server frame proves the current token was accepted; allow a later
      // daemon restart/rotation to use the same single-refresh recovery path.
      this.#invalidTokenRetries = 0;
      this.#frameListeners.forEach((listener) => {
        listener(frame);
      });
    }
  }

  async #disconnectAfterFailure(): Promise<void> {
    this.#token = null;
    await disconnectDaemonWebSocket().catch(() => undefined);
  }

  async #invalidateToken(): Promise<void> {
    this.#token = null;
    this.#generation += 1;
    this.#setState('auth-invalid');
    await disconnectDaemonWebSocket().catch(() => undefined);
    if (this.#invalidTokenRetries === 0) {
      this.#invalidTokenRetries = 1;
      this.#scheduleReconnect();
    } else {
      this.#clearRetry();
      this.#setState('auth-required');
    }
  }

  #scheduleReconnect(): void {
    if (this.#closed || this.#retryTimer !== null) return;
    this.#setState('connecting');
    const delay = this.#retryDelayMs;
    this.#retryDelayMs = Math.min(this.#retryDelayMs * 2, MAX_RETRY_MS);
    this.#retryTimer = setTimeout(() => {
      this.#retryTimer = null;
      void this.#connect();
    }, delay);
  }

  #clearRetry(): void {
    if (this.#retryTimer !== null) clearTimeout(this.#retryTimer);
    this.#retryTimer = null;
  }

  #setState(state: LinkState): void {
    if (this.#state === state) return;
    this.#state = state;
    this.#stateListeners.forEach((listener) => {
      listener(state);
    });
  }
}

function isApiErrorEnvelope(value: unknown): value is { error: { message: string } } {
  if (typeof value !== 'object' || value === null || !('error' in value)) return false;
  const error = value.error;
  return (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  );
}

function parseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return undefined;
  }
}

function isToken(value: string): boolean {
  return /^[0-9a-fA-F]{64}$/.test(value);
}
