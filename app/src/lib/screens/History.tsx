import React, { useEffect, useState } from 'react';
import { Download, Lock, Search } from 'lucide-react';
import {
    DataTable,
    LinearButton,
    LinearCard,
    RangeTabs,
    type DataTableColumn,
} from '$lib/components/linear';
import { TrafficChart } from '$lib/components/charts';
import { daemonClient, useDaemonStore } from '$lib/stores/useDaemonStore';
import type { HistoryRow } from '$lib/fixtures/demo';
import { formatBytes, formatDateTime, formatShortTime } from '$lib/utils/format';
import { save } from '@tauri-apps/plugin-dialog';
import { saveExportFile } from '$lib/api/transport';

const PERIOD_OPTIONS = [
    { id: '24h', label: '24h' },
    { id: '7d', label: '7d' },
    { id: '30d', label: '30d' },
    { id: 'cycle', label: 'Cycle' },
];

export const History: React.FC = () => {
    const {
        historyRows,
        historyLoading,
        historyError,
        timeline,
        isPro,
        demoMode,
        connection,
        refreshHistory,
        openUpgrade,
        openAppDetail,
    } = useDaemonStore();

    const [period, setPeriod] = useState('7d');
    const [appSearch, setAppSearch] = useState('');
    const [remoteHost, setRemoteHost] = useState('');
    const [remotePort, setRemotePort] = useState('');
    const [protocol, setProtocol] = useState('');
    const [exportStatus, setExportStatus] = useState<string | null>(null);

    useEffect(() => {
        const params: Record<string, string | number> = {};
        if (appSearch.trim()) params.app_name = appSearch.trim();
        if (isPro) {
            if (remoteHost.trim()) params.remote_host = remoteHost.trim();
            if (remotePort.trim()) params.remote_port = remotePort.trim();
            if (protocol.trim()) params.protocol = protocol.trim();
        }
        void refreshHistory(params);
    }, [appSearch, remoteHost, remotePort, protocol, isPro, refreshHistory]);

    const slicePts = timeline.slice(-14);
    const firstPt = slicePts[0];
    const secondPt = slicePts[1];
    const subDayBuckets =
        firstPt !== undefined &&
        secondPt !== undefined &&
        secondPt.bucket_start - firstPt.bucket_start < 12 * 3600;
    const dailyBars = slicePts.map((pt) => ({
        label: subDayBuckets
            ? formatShortTime(pt.bucket_start)
            : new Date(pt.bucket_start * 1000).toLocaleDateString([], {
                  month: 'short',
                  day: 'numeric',
              }),
        down: pt.down,
        up: pt.up,
    }));

    const handleExport = async (format: 'csv' | 'json') => {
        if (!isPro) {
            openUpgrade();
            return;
        }
        if (demoMode || connection !== 'connected') {
            setExportStatus(`Demo export (${format.toUpperCase()}) requires an active daemon.`);
            return;
        }
        try {
            setExportStatus(`Preparing ${format.toUpperCase()} export...`);
            const job = (await daemonClient.request('POST', '/api/v1/exports', {
                scope: 'connections',
                format,
                filters: appSearch.trim() ? { app_name: appSearch.trim() } : { protocol: 'tcp' },
            })) as { id: string };
            const content = await daemonClient.requestText(
                'GET',
                `/api/v1/exports/${job.id}/content`,
            );
            const target = await save({
                defaultPath: `netwatch-pro-export.${format}`,
                filters: [{ name: format.toUpperCase(), extensions: [format] }],
            });
            if (target) {
                await saveExportFile(target, content);
                setExportStatus(`Saved ${format.toUpperCase()} export.`);
            } else {
                setExportStatus(null);
            }
        } catch (err) {
            setExportStatus(err instanceof Error ? err.message : 'Export failed');
        }
    };

    const columns: DataTableColumn<HistoryRow>[] = [
        {
            key: 'time',
            header: 'Timestamp',
            render: (row) => (
                <span className="text-[var(--text-muted)] tabular-nums">
                    {formatDateTime(row.last_seen_at ?? row.first_seen_at ?? 0)}
                </span>
            ),
        },
        {
            key: 'app',
            header: 'Application',
            render: (row) => {
                const name = row.app?.name || row.app_name || row.process_name || 'Unknown';
                return (
                    <div className="flex items-center gap-2">
                        <span className="font-medium text-[var(--text)]">{name}</span>
                        {row.connection_count && row.connection_count > 1 && (
                            <span className="px-1.5 py-0.5 rounded bg-[var(--bg-surface-2)] border border-[var(--border)] text-[11px] text-[var(--text-muted)]">
                                day rollup
                            </span>
                        )}
                    </div>
                );
            },
        },
        {
            key: 'remote',
            header: 'Remote Endpoint',
            render: (row) => (
                <div className="flex flex-col">
                    <span className="text-[var(--text)] tabular-nums">
                        {row.remote_ip}:{row.remote_port}
                    </span>
                    {row.remote_host && (
                        <span className="text-[11px] text-[var(--text-muted)]">
                            {row.remote_host}
                        </span>
                    )}
                </div>
            ),
        },
        {
            key: 'down',
            header: 'Download',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--primary-text)] font-medium tabular-nums">
                    {formatBytes(row.bytes_down)}
                </span>
            ),
        },
        {
            key: 'up',
            header: 'Upload',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--sky-text)] font-medium tabular-nums">
                    {formatBytes(row.bytes_up)}
                </span>
            ),
        },
    ];

    return (
        <div className="space-y-4">
            {/* Bar Chart by Day / Bucket */}
            <LinearCard level={1} className="p-5">
                <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
                    <div>
                        <h1 className="text-sm font-semibold text-[var(--text)]">Usage History</h1>
                        <p className="text-xs text-[var(--text-muted)]">
                            Local SQLite connection history and daily endpoint aggregates
                        </p>
                    </div>
                    <div className="flex items-center gap-2">
                        <RangeTabs
                            options={PERIOD_OPTIONS}
                            active={period}
                            onChange={setPeriod}
                            ariaLabel="History range"
                        />
                        <LinearButton
                            variant="secondary"
                            size="sm"
                            onClick={() => void handleExport('csv')}
                        >
                            <Download className="size-3.5" aria-hidden="true" />
                            <span>Export CSV</span>
                            {!isPro && (
                                <Lock
                                    className="size-3 text-[var(--text-muted)]"
                                    aria-hidden="true"
                                />
                            )}
                        </LinearButton>
                    </div>
                </div>

                <TrafficChart
                    data={dailyBars}
                    mode="bar"
                    unit="bytes"
                    className="h-52 w-full"
                    ariaLabel="Historical usage bar chart"
                />
            </LinearCard>

            {exportStatus && (
                <div className="text-xs text-[var(--text-muted)] px-1" role="status">
                    {exportStatus}
                </div>
            )}

            {/* Search + Pro Filters */}
            <div className="flex flex-wrap items-center gap-2">
                <div className="relative flex-1 min-w-[220px] max-w-xs">
                    <Search
                        className="absolute left-2.5 top-2 size-4 text-[var(--text-muted)]"
                        aria-hidden="true"
                    />
                    <input
                        type="text"
                        value={appSearch}
                        onChange={(e) => setAppSearch(e.target.value)}
                        placeholder="Search by app name..."
                        aria-label="Search history by application"
                        className="w-full h-8 pl-8 pr-3 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--primary-text)]"
                    />
                </div>

                {isPro ? (
                    <>
                        <input
                            type="text"
                            value={remoteHost}
                            onChange={(e) => setRemoteHost(e.target.value)}
                            placeholder="Remote host..."
                            aria-label="Filter by remote host"
                            className="h-8 px-2.5 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--primary-text)]"
                        />
                        <input
                            type="text"
                            value={remotePort}
                            onChange={(e) => setRemotePort(e.target.value)}
                            placeholder="Port"
                            aria-label="Filter by remote port"
                            className="w-20 h-8 px-2.5 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--primary-text)]"
                        />
                        <select
                            value={protocol}
                            onChange={(e) => setProtocol(e.target.value)}
                            aria-label="Filter by protocol"
                            className="h-8 px-2.5 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)]"
                        >
                            <option value="">Any proto</option>
                            <option value="tcp">TCP</option>
                            <option value="udp">UDP</option>
                            <option value="icmp">ICMP</option>
                        </select>
                    </>
                ) : (
                    <div className="flex flex-wrap items-center gap-1.5">
                        {['Remote host', 'Port', 'Protocol'].map((chip) => (
                            <button
                                key={chip}
                                type="button"
                                onClick={openUpgrade}
                                className="h-8 px-2.5 rounded bg-[var(--bg-surface-2)] border border-[var(--border)] text-xs text-[var(--text-muted)] hover:text-[var(--text)] inline-flex items-center gap-1.5 cursor-pointer"
                                title="Requires Pro license"
                            >
                                <Lock className="size-3" aria-hidden="true" />
                                <span>{chip}</span>
                            </button>
                        ))}
                    </div>
                )}
            </div>

            {/* History Table */}
            <DataTable
                columns={columns}
                rows={historyRows}
                rowKey={(row, idx) => String(row.id ?? `agg-${idx}`)}
                onRowClick={(row) => {
                    const id = row.app?.id ?? row.app_id;
                    if (id) openAppDetail(id);
                }}
                loading={historyLoading}
                error={historyError}
                onRetry={() => void refreshHistory()}
                emptyTitle="No recorded connections"
                emptyDescription="Connections recorded by netwatchd will appear here."
                caption="Historical network connections"
            />
        </div>
    );
};
