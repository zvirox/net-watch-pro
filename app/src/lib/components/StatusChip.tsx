import React from 'react';
import { StatusPill } from './linear/StatusPill';
import { useDaemonStore } from '$lib/stores/useDaemonStore';

export const StatusChip: React.FC = () => {
  const { daemon, ebpf, connection } = useDaemonStore();

  const getStatus = (): 'online' | 'active' | 'warning' | 'alert' | 'idle' => {
    if (connection === 'connected' && daemon === 'monitoring') return 'online';
    if (connection === 'connecting' || daemon === 'starting') return 'active';
    if (daemon === 'degraded' || ebpf === 'failed') return 'warning';
    if (connection === 'auth-required' || connection === 'auth-invalid') return 'alert';
    return 'idle';
  };

  const getLabel = () => {
    if (connection === 'connected') {
      return daemon === 'monitoring' ? 'daemon active' : daemon;
    }
    return connection;
  };

  return (
    <StatusPill
      label={getLabel()}
      status={getStatus()}
      monoValue={ebpf === 'attached' ? 'eBPF CO-RE' : undefined}
    />
  );
};
