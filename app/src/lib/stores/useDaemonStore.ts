import { create } from 'zustand';
import { DaemonApiClient } from '$lib/api/client';

export type DaemonState = 'monitoring' | 'degraded' | 'stopped' | 'starting';
export type EbpfState = 'attached' | 'failed' | 'unsupported' | 'unattached' | 'no_collector';
export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'auth-required'
  | 'auth-invalid';

interface DaemonStoreState {
  connection: ConnectionState;
  daemon: DaemonState;
  ebpf: EbpfState;
  recording: boolean;
  activeScreen: string;
  setActiveScreen: (screen: string) => void;
  start: () => void;
  stop: () => void;
}

const client = new DaemonApiClient();

export const useDaemonStore = create<DaemonStoreState>((set) => {
  let started = false;

  client.onState((state) => {
    set((prev) => ({
      connection: state,
      daemon:
        state === 'connected'
          ? prev.daemon
          : state === 'connecting' || state === 'disconnected'
            ? 'starting'
            : 'stopped',
      ebpf: state === 'connected' ? prev.ebpf : 'no_collector',
    }));
  });

  client.onFrame((frame: any) => {
    if (frame && frame.channel === 'status' && frame.kind === 'update' && frame.data) {
      set({
        daemon: frame.data.daemon,
        ebpf: frame.data.ebpf,
        recording: frame.data.recording,
      });
    }
  });

  return {
    connection: 'disconnected',
    daemon: 'stopped',
    ebpf: 'no_collector',
    recording: false,
    activeScreen: 'dashboard',
    setActiveScreen: (screen: string) => set({ activeScreen: screen }),
    start: () => {
      if (started) return;
      started = true;
      client.start();
    },
    stop: () => {
      started = false;
      client.stop();
    },
  };
});
