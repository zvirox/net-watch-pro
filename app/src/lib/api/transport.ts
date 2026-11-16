import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type DaemonHttpResponse = {
  status: number;
  body: string;
};

type DaemonEvent = {
  state: (state: string) => void;
  frame: (frame: string) => void;
};

export async function requestDaemon(
  method: string,
  path: string,
  body?: unknown,
): Promise<DaemonHttpResponse> {
  return invoke<DaemonHttpResponse>('daemon_http_request', {
    method,
    path,
    body: body === undefined ? null : JSON.stringify(body),
  });
}

export async function connectDaemonWebSocket(): Promise<void> {
  await invoke('daemon_connect');
}

export async function sendDaemonFrame(frame: unknown): Promise<void> {
  await invoke('daemon_ws_send', { frame: JSON.stringify(frame) });
}

export async function disconnectDaemonWebSocket(): Promise<void> {
  await invoke('daemon_ws_disconnect');
}

export async function listenDaemonEvents(handlers: DaemonEvent): Promise<UnlistenFn[]> {
  const unlistenState = await listen<string>('daemon://state', (event) => {
    handlers.state(event.payload);
  });
  const unlistenFrame = await listen<string>('daemon://frame', (event) => {
    handlers.frame(event.payload);
  });
  return [unlistenState, unlistenFrame];
}
