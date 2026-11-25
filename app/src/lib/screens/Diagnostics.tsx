import React, { useEffect, useState } from 'react';
import { Copy, RefreshCw, ShieldAlert } from 'lucide-react';
import { LinearButton, LinearCard, StatusPill } from '$lib/components/linear';
import { useDaemonStore } from '$lib/stores/useDaemonStore';
import { formatBytes, formatDateTime } from '$lib/utils/format';

export const Diagnostics: React.FC = () => {
    const {
        diagnostics,
        diagnosticsLoading,
        diagnosticsError,
        ebpf,
        daemon,
        connection,
        refreshDiagnostics,
        triggerEnableMonitoring,
        enablingMonitoring,
    } = useDaemonStore();

    const [copied, setCopied] = useState(false);

    useEffect(() => {
        void refreshDiagnostics();
    }, [refreshDiagnostics]);

    const mapCheckStatus = (result: 'pass' | 'fail' | 'skip') => {
        if (result === 'pass') return 'online' as const;
        if (result === 'fail') return 'alert' as const;
        return 'idle' as const;
    };

    const handleCopyReport = async () => {
        const payload = {
            generated_at: new Date().toISOString(),
            connection,
            daemon,
            ebpf,
            diagnostics,
        };
        try {
            await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
            setCopied(true);
            window.setTimeout(() => setCopied(false), 2000);
        } catch {
            // Clipboard blocked in restricted context
        }
    };

    return (
        <div className="space-y-4 max-w-4xl">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <h1 className="text-base font-semibold text-[var(--text)]">
                        System Diagnostics
                    </h1>
                    <p className="text-xs text-[var(--text-muted)]">
                        Health checks, Unix socket peer-credential stats, and daemon event log
                    </p>
                </div>

                <div className="flex items-center gap-2">
                    <LinearButton
                        variant="secondary"
                        size="sm"
                        onClick={() => void handleCopyReport()}
                    >
                        <Copy className="size-3.5" aria-hidden="true" />
                        <span>{copied ? 'Copied' : 'Copy report'}</span>
                    </LinearButton>
                    <LinearButton
                        variant="secondary"
                        size="sm"
                        onClick={() => void refreshDiagnostics()}
                        disabled={diagnosticsLoading}
                    >
                        <RefreshCw className="size-3.5" aria-hidden="true" />
                        <span>Refresh</span>
                    </LinearButton>
                    <LinearButton
                        variant="primary"
                        size="sm"
                        onClick={() => void triggerEnableMonitoring()}
                        disabled={enablingMonitoring}
                    >
                        <span>{enablingMonitoring ? 'Authorizing...' : 'Restart daemon'}</span>
                    </LinearButton>
                </div>
            </div>

            {diagnosticsError && (
                <LinearCard
                    level={1}
                    className="p-4 border-[var(--accent-red)] bg-[var(--tint-red)] flex items-center gap-2 text-xs text-[var(--tint-red-fg)]"
                    role="alert"
                >
                    <ShieldAlert className="size-4 shrink-0" aria-hidden="true" />
                    <span>{diagnosticsError}</span>
                </LinearCard>
            )}

            {/* Health Checks */}
            <LinearCard level={1} className="p-5">
                <h2 className="text-sm font-semibold text-[var(--text)] mb-3">Health Checks</h2>
                <div className="divide-y divide-[var(--border)]">
                    {(diagnostics?.checks ?? []).map((chk) => (
                        <div
                            key={chk.name}
                            className="py-3 flex flex-wrap items-center justify-between gap-3 text-xs"
                        >
                            <div className="space-y-0.5">
                                <div className="font-medium text-[var(--text)]">{chk.name}</div>
                                {chk.detail && (
                                    <div className="text-[var(--text-muted)]">{chk.detail}</div>
                                )}
                            </div>
                            <StatusPill
                                label={chk.result === 'skip' ? '— skip' : chk.result}
                                status={mapCheckStatus(chk.result)}
                            />
                        </div>
                    ))}
                </div>
            </LinearCard>

            {/* Storage & Auth Counters */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <LinearCard level={1} className="p-5">
                    <div className="text-xs uppercase tracking-wider text-[var(--text-muted)]">
                        Database size
                    </div>
                    <div className="mt-2 text-xl font-semibold text-[var(--text)] tabular-nums">
                        {formatBytes(diagnostics?.data.db_size_bytes ?? 0)}
                    </div>
                </LinearCard>

                <LinearCard level={1} className="p-5">
                    <div className="text-xs uppercase tracking-wider text-[var(--text-muted)]">
                        Retention window
                    </div>
                    <div className="mt-2 text-xl font-semibold text-[var(--text)] tabular-nums">
                        {diagnostics?.data.retention_days ?? 90} days
                    </div>
                </LinearCard>

                <LinearCard level={1} className="p-5">
                    <div className="text-xs uppercase tracking-wider text-[var(--text-muted)]">
                        Rejected UID connects
                    </div>
                    <div className="mt-2 text-xl font-semibold text-[var(--text)] tabular-nums">
                        {diagnostics?.rejected_connections.count ?? 0}
                    </div>
                    {diagnostics?.rejected_connections.last_rejected_uid !== null &&
                        diagnostics?.rejected_connections.last_rejected_uid !== undefined && (
                            <div className="mt-1 text-xs text-[var(--text-muted)]">
                                Last UID: {diagnostics.rejected_connections.last_rejected_uid}
                            </div>
                        )}
                </LinearCard>
            </div>

            {/* Event Journal (Monospace reserved strictly for logs per Plan §1) */}
            <LinearCard level={1} className="p-5">
                <h2 className="text-sm font-semibold text-[var(--text)] mb-3">Daemon Event Log</h2>
                <div className="rounded-md border border-[var(--border)] bg-[var(--bg-app)] p-3 font-mono text-xs space-y-2 max-h-72 overflow-y-auto">
                    {(diagnostics?.journal_excerpt ?? []).length === 0 ? (
                        <div className="text-[var(--text-muted)]">No daemon events recorded.</div>
                    ) : (
                        (diagnostics?.journal_excerpt ?? []).map((ev, idx) => (
                            <div
                                key={`${ev.code}-${String(idx)}`}
                                className="flex flex-wrap items-baseline gap-2.5 border-b border-[var(--border)] last:border-0 pb-1.5 last:pb-0"
                            >
                                <span className="text-[var(--text-faint)] tabular-nums shrink-0">
                                    {formatDateTime(ev.occurred_at)}
                                </span>
                                <span
                                    className={`px-1.5 py-0.5 rounded text-[11px] uppercase shrink-0 ${
                                        ev.level === 'error'
                                            ? 'bg-[var(--tint-red)] text-[var(--tint-red-fg)]'
                                            : ev.level === 'warn'
                                              ? 'bg-[var(--tint-amber)] text-[var(--tint-amber-fg)]'
                                              : 'bg-[var(--tint-green)] text-[var(--tint-green-fg)]'
                                    }`}
                                >
                                    {ev.level}
                                </span>
                                <span className="text-[var(--tint-blue-fg)] shrink-0">
                                    {ev.code}
                                </span>
                                <span className="text-[var(--text)]">{ev.message}</span>
                            </div>
                        ))
                    )}
                </div>
            </LinearCard>
        </div>
    );
};
