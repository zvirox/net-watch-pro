import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { DaemonApiClient } from './client';

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
}));

vi.mock('./transport', () => ({
  connectDaemonWebSocket: vi.fn().mockRejectedValue(new Error('daemon connection failed')),
  disconnectDaemonWebSocket: vi.fn().mockResolvedValue(undefined),
  listenDaemonEvents: vi.fn().mockResolvedValue([]),
  requestDaemon: vi.fn(),
  sendDaemonFrame: vi.fn(),
}));

describe('DaemonApiClient', () => {
  let client: DaemonApiClient;

  beforeEach(() => {
    vi.useFakeTimers();
    client = new DaemonApiClient();
  });

  afterEach(async () => {
    await client.stop();
    vi.useRealTimers();
  });

  it('starts in disconnected state with 1000ms initial retry delay', () => {
    expect(client.state).toBe('disconnected');
    expect(client.retryDelayMs).toBe(1000);
  });

  it('exponentially backs off reconnect attempts up to 16,000 ms', async () => {
    client.handleNativeState('disconnected');
    expect(client.state).toBe('connecting');
    expect(client.retryDelayMs).toBe(2000);

    // Timer fires -> triggers #connect() -> throws -> schedules reconnect doubling to 4000
    await vi.advanceTimersByTimeAsync(1000);
    expect(client.retryDelayMs).toBe(4000);

    await vi.advanceTimersByTimeAsync(2000);
    expect(client.retryDelayMs).toBe(8000);

    await vi.advanceTimersByTimeAsync(4000);
    expect(client.retryDelayMs).toBe(16000);

    await vi.advanceTimersByTimeAsync(8000);
    expect(client.retryDelayMs).toBe(16000);

    await vi.advanceTimersByTimeAsync(16000);
    expect(client.retryDelayMs).toBe(16000);
  });

  it('resets retry delay to 1000ms upon successful connection', () => {
    client.handleNativeState('disconnected');
    expect(client.retryDelayMs).toBe(2000);

    client.handleNativeState('connected');
    expect(client.state).toBe('connected');
    expect(client.retryDelayMs).toBe(1000);
  });

  it('transitions to auth-invalid and ceases reconnect attempts', () => {
    client.handleNativeState('auth-invalid');
    expect(client.state).toBe('auth-invalid');

    // Subsequent disconnect events must not enter reconnect loop when auth is invalid
    client.handleNativeState('disconnected');
    expect(client.state).toBe('auth-invalid');
  });
});
