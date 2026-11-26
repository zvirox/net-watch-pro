import React from 'react';
import { StatusPill } from './linear/StatusPill';
import { useDaemonStore } from '$lib/stores/useDaemonStore';

export interface StatusChipProps {
    className?: string;
    onClick?: () => void;
}

export const StatusChip: React.FC<StatusChipProps> = ({ className = '', onClick }) => {
    const { daemon, ebpf, connection, demoMode } = useDaemonStore();

    const getStatus = (): 'online' | 'active' | 'warning' | 'alert' | 'idle' => {
        if (demoMode) return 'active';
        if (connection === 'connected' && daemon === 'monitoring' && ebpf === 'attached')
            return 'online';
        if (
            connection === 'connected' &&
            (ebpf === 'no_collector' || daemon === 'degraded' || ebpf === 'failed')
        ) {
            return 'warning';
        }
        if (connection === 'connecting' || daemon === 'starting') return 'active';
        if (connection === 'auth-required' || connection === 'auth-invalid') return 'alert';
        return 'idle';
    };

    const getLabel = (): string => {
        if (demoMode) return 'Demo mode';
        if (connection === 'connected') {
            if (ebpf === 'no_collector' || ebpf === 'failed') return 'Degraded';
            return daemon === 'monitoring' ? 'Monitoring' : daemon;
        }
        if (connection === 'connecting') return 'Connecting';
        if (connection === 'auth-required') return 'Setup required';
        if (connection === 'auth-invalid') return 'Auth invalid';
        return 'Stopped';
    };

    const getBadge = (): string | undefined => {
        if (demoMode) return 'Demo';
        if (connection === 'connected' && ebpf === 'attached') return 'eBPF';
        if (connection === 'connected' && ebpf === 'no_collector') return 'Sysfs only';
        return undefined;
    };

    const tooltip =
        connection === 'connected' && ebpf === 'no_collector'
            ? 'No collector — PC live speed works, per-app rates unavailable'
            : `Daemon: ${getLabel()}`;

    if (onClick) {
        return (
            <button
                type="button"
                onClick={onClick}
                title={tooltip}
                aria-label={tooltip}
                className={`w-full text-left rounded cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--primary-text)] ${className}`}
            >
                <StatusPill
                    label={getLabel()}
                    status={getStatus()}
                    monoValue={getBadge()}
                    className="w-full justify-between"
                />
            </button>
        );
    }

    return (
        <StatusPill
            label={getLabel()}
            status={getStatus()}
            monoValue={getBadge()}
            className={className}
        />
    );
};
