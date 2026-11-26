import React, { useMemo, useState } from 'react';
import { Pause, Play, Search, Info } from 'lucide-react';
import {
    DataTable,
    LinearButton,
    LinearCard,
    RangeTabs,
    StatusPill,
    type DataTableColumn,
} from '$lib/components/linear';
import { TrafficChart } from '$lib/components/charts';
import { useDaemonStore, type LiveConnectionRow } from '$lib/stores/useDaemonStore';
import { formatBytes, formatRateText, formatShortTime } from '$lib/utils/format';

const LIVE_RANGE_OPTIONS = ['1W', '1M', '3W', 'YTD', 'Total'];

export const Live: React.FC = () => {
    const { liveConnections, rateHistory, speedUnit, ebpf, connection, demoMode, openAppDetail } =
        useDaemonStore();

    const [search, setSearch] = useState('');
    const [protocol, setProtocol] = useState<'all' | 'tcp' | 'udp' | 'icmp' | 'other'>('all');
    const [liveWindow, setLiveWindow] = useState('1W');
    const [paused, setPaused] = useState(false);
    const [frozenRows, setFrozenRows] = useState<LiveConnectionRow[]>([]);

    const activeRows = paused ? frozenRows : liveConnections;

    const filtered = useMemo(() => {
        const q = search.trim().toLowerCase();
        return activeRows.filter((row) => {
            if (protocol !== 'all' && row.protocol.toLowerCase() !== protocol) return false;
            if (!q) return true;
            return (
                row.app.name.toLowerCase().includes(q) ||
                (row.process_name?.toLowerCase().includes(q) ?? false) ||
                row.remote_ip.includes(q) ||
                (row.remote_host?.toLowerCase().includes(q) ?? false)
            );
        });
    }, [activeRows, search, protocol]);

    const togglePause = () => {
        if (paused) {
            setPaused(false);
        } else {
            setFrozenRows(liveConnections);
            setPaused(true);
        }
    };

    const chartData = rateHistory.slice(-24).map((sample) => ({
        label: formatShortTime(sample.ts),
        down: sample.down_bps,
        up: sample.up_bps,
    }));

    const collectorMissing =
        !demoMode && connection === 'connected' && (ebpf === 'no_collector' || ebpf === 'failed');

    const columns: DataTableColumn<LiveConnectionRow>[] = [
        {
            key: 'pid',
            header: 'PID',
            render: (row) => (
                <span className="text-[var(--text-muted)] tabular-nums">
                    {row.pid === 0 ? '—' : String(row.pid)}
                </span>
            ),
        },
        {
            key: 'app',
            header: 'Application',
            render: (row) => (
                <div className="flex flex-col">
                    <span className="font-medium text-[var(--text)]">
                        {row.app.name || 'Unknown'}
                    </span>
                    {row.process_name && row.process_name !== row.app.name && (
                        <span className="text-[11px] text-[var(--text-muted)]">
                            {row.process_name}
                        </span>
                    )}
                </div>
            ),
        },
        {
            key: 'proto',
            header: 'Protocol',
            render: (row) => (
                <span className="px-1.5 py-0.5 rounded bg-[var(--bg-surface-2)] border border-[var(--border)] text-[11px] font-medium uppercase text-[var(--text-muted)]">
                    {row.protocol}
                </span>
            ),
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
            header: 'Down Rate',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--primary-text)] font-medium tabular-nums">
                    {formatRateText(row.rate_down_bps, speedUnit)}
                </span>
            ),
        },
        {
            key: 'up',
            header: 'Up Rate',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--sky-text)] font-medium tabular-nums">
                    {formatRateText(row.rate_up_bps, speedUnit)}
                </span>
            ),
        },
        {
            key: 'total',
            header: 'Transferred',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--text-muted)] tabular-nums">
                    {formatBytes(row.bytes_down + row.bytes_up)}
                </span>
            ),
        },
        {
            key: 'status',
            header: 'Status',
            align: 'right',
            render: () => <StatusPill label="Active" status="online" />,
        },
    ];

    return (
        <div className="space-y-4">
            {/* Live Bandwidth Chart */}
            <LinearCard level={1} className="p-5">
                <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
                    <div>
                        <h1 className="text-sm font-semibold text-[var(--text)]">Live Bandwidth</h1>
                        <p className="text-xs text-[var(--text-muted)]">
                            Real-time throughput stream (blue download, orange upload)
                        </p>
                    </div>
                    <RangeTabs
                        options={LIVE_RANGE_OPTIONS}
                        active={liveWindow}
                        onChange={setLiveWindow}
                        ariaLabel="Live chart window"
                    />
                </div>
                <TrafficChart
                    data={chartData}
                    mode="line"
                    unit="rate"
                    className="h-52 w-full"
                    ariaLabel="Real-time bandwidth chart"
                />
            </LinearCard>

            {collectorMissing && (
                <LinearCard
                    level={1}
                    className="px-4 py-3 border-[var(--amber)] bg-[var(--bg-surface-2)] flex items-center gap-2 text-xs text-[var(--text)]"
                    role="status"
                >
                    <Info className="size-4 text-[var(--amber)] shrink-0" aria-hidden="true" />
                    <span>
                        No eBPF collector attached ({ebpf}). Live per-connection rows are
                        unavailable until monitoring is active.
                    </span>
                </LinearCard>
            )}

            {/* Filter & Pause Controls */}
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-2 flex-1 min-w-[240px] max-w-md">
                    <div className="relative flex-1">
                        <Search
                            className="absolute left-2.5 top-2 size-4 text-[var(--text-muted)]"
                            aria-hidden="true"
                        />
                        <input
                            type="text"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Search app, IP, or host..."
                            aria-label="Filter active connections"
                            className="w-full h-8 pl-8 pr-3 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--primary-text)]"
                        />
                    </div>

                    <select
                        aria-label="Filter by protocol"
                        value={protocol}
                        onChange={(e) => setProtocol(e.target.value as typeof protocol)}
                        className="h-8 px-2.5 rounded-md bg-[var(--bg-surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] focus:outline-none focus:border-[var(--primary-text)]"
                    >
                        <option value="all">All protocols</option>
                        <option value="tcp">TCP</option>
                        <option value="udp">UDP</option>
                        <option value="icmp">ICMP</option>
                        <option value="other">Other</option>
                    </select>
                </div>

                <LinearButton
                    variant={paused ? 'primary' : 'secondary'}
                    size="sm"
                    onClick={togglePause}
                    aria-pressed={paused}
                >
                    {paused ? (
                        <>
                            <Play className="size-3.5" aria-hidden="true" />
                            <span>Resume</span>
                        </>
                    ) : (
                        <>
                            <Pause className="size-3.5" aria-hidden="true" />
                            <span>Pause</span>
                        </>
                    )}
                </LinearButton>
            </div>

            {/* Connections Table */}
            <DataTable
                columns={columns}
                rows={filtered}
                rowKey={(row) => String(row.id)}
                onRowClick={(row) => openAppDetail(row.app.id)}
                emptyTitle="No active connections"
                emptyDescription={
                    search || protocol !== 'all'
                        ? 'No active sockets match the current filter.'
                        : 'Open network flows will stream here automatically.'
                }
                caption="Active network connections"
            />
        </div>
    );
};
