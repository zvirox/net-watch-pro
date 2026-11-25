import React, { useEffect, useState } from 'react';
import { ArrowLeft } from 'lucide-react';
import {
    DataTable,
    LinearButton,
    LinearCard,
    StatCard,
    type DataTableColumn,
} from '$lib/components/linear';
import { TrafficChart } from '$lib/components/charts';
import {
    daemonClient,
    useDaemonStore,
    type AppDetailData,
    type EndpointRow,
} from '$lib/stores/useDaemonStore';
import type { HistoryRow, TimelinePoint } from '$lib/fixtures/demo';
import { formatBytes, formatDateTime, formatShortTime } from '$lib/utils/format';

type DetailTab = 'overview' | 'connections' | 'endpoints';

export const AppDetail: React.FC = () => {
    const {
        selectedAppId,
        appsList,
        historyRows,
        timeline,
        rangePreset,
        demoMode,
        connection,
        setActiveScreen,
    } = useDaemonStore();

    const [tab, setTab] = useState<DetailTab>('overview');
    const [detail, setDetail] = useState<AppDetailData | null>(null);
    const [appTimeline, setAppTimeline] = useState<TimelinePoint[]>([]);
    const [endpoints, setEndpoints] = useState<EndpointRow[]>([]);
    const [connections, setConnections] = useState<HistoryRow[]>([]);

    useEffect(() => {
        if (selectedAppId === null) return;

        if (demoMode || connection !== 'connected') {
            const found = appsList.find((a) => a.id === selectedAppId) ?? appsList[0];
            if (found) {
                setDetail({
                    ...found,
                    exe_path: `/usr/bin/${found.name.toLowerCase()}`,
                    deleted_at: null,
                });
            }
            setAppTimeline(timeline.slice(-12));
            setConnections(historyRows.filter((r) => (r.app?.id ?? r.app_id) === selectedAppId));
            setEndpoints([
                {
                    remote_ip: '142.250.190.46',
                    remote_host: 'cdn.example.net',
                    port: 443,
                    bytes_total: 42_000_000,
                    connections: 18,
                    last_seen_at: Math.floor(Date.now() / 1000) - 60,
                },
                {
                    remote_ip: '151.101.1.1',
                    remote_host: 'assets.example.org',
                    port: 443,
                    bytes_total: 9_400_000,
                    connections: 6,
                    last_seen_at: Math.floor(Date.now() / 1000) - 900,
                },
            ]);
            return;
        }

        void (async () => {
            try {
                const [d, u, ep, conn] = await Promise.all([
                    daemonClient.request('GET', `/api/v1/apps/${String(selectedAppId)}`),
                    daemonClient.request(
                        'GET',
                        `/api/v1/apps/${String(selectedAppId)}/usage?range=${rangePreset}`,
                    ),
                    daemonClient.request('GET', `/api/v1/apps/${String(selectedAppId)}/endpoints`),
                    daemonClient.request(
                        'GET',
                        `/api/v1/connections?app_id=${String(selectedAppId)}&limit=50`,
                    ),
                ]);
                setDetail(d as AppDetailData);
                const usageRes = u as { data?: TimelinePoint[] };
                setAppTimeline(Array.isArray(usageRes.data) ? usageRes.data : []);
                setEndpoints(Array.isArray(ep) ? (ep as EndpointRow[]) : []);
                const connRes = conn as { data?: HistoryRow[] };
                setConnections(Array.isArray(connRes.data) ? connRes.data : []);
            } catch {
                // Keep last state
            }
        })();
    }, [selectedAppId, demoMode, connection, rangePreset, appsList, historyRows, timeline]);

    if (!detail) {
        return (
            <LinearCard level={1} className="p-8 text-center">
                <p className="text-sm font-medium text-[var(--text)]">Select an application</p>
                <LinearButton
                    variant="secondary"
                    size="sm"
                    className="mt-3"
                    onClick={() => setActiveScreen('apps')}
                >
                    Back to Apps
                </LinearButton>
            </LinearCard>
        );
    }

    const connColumns: DataTableColumn<HistoryRow>[] = [
        {
            key: 'time',
            header: 'Last Active',
            render: (r) => (
                <span className="text-[var(--text-muted)] tabular-nums">
                    {formatDateTime(r.last_seen_at ?? 0)}
                </span>
            ),
        },
        {
            key: 'remote',
            header: 'Remote',
            render: (r) => (
                <span className="text-[var(--text)] tabular-nums">
                    {r.remote_ip}:{r.remote_port}
                </span>
            ),
        },
        {
            key: 'proto',
            header: 'Protocol',
            render: (r) => (
                <span className="uppercase text-xs text-[var(--text-muted)]">
                    {r.protocol ?? r.proto ?? 'tcp'}
                </span>
            ),
        },
        {
            key: 'down',
            header: 'Download',
            align: 'right',
            render: (r) => (
                <span className="text-[var(--tint-blue-fg)] tabular-nums">
                    {formatBytes(r.bytes_down)}
                </span>
            ),
        },
        {
            key: 'up',
            header: 'Upload',
            align: 'right',
            render: (r) => (
                <span className="text-[var(--tint-sky-fg)] tabular-nums">
                    {formatBytes(r.bytes_up)}
                </span>
            ),
        },
    ];

    const endpointColumns: DataTableColumn<EndpointRow>[] = [
        {
            key: 'host',
            header: 'Remote Host / IP',
            render: (e) => (
                <div className="flex flex-col">
                    <span className="font-medium text-[var(--text)]">
                        {e.remote_host ?? e.remote_ip}
                    </span>
                    {e.remote_host && (
                        <span className="text-[11px] text-[var(--text-muted)] tabular-nums">
                            {e.remote_ip}
                        </span>
                    )}
                </div>
            ),
        },
        {
            key: 'port',
            header: 'Port',
            render: (e) => <span className="text-[var(--text-muted)] tabular-nums">{e.port}</span>,
        },
        {
            key: 'count',
            header: 'Connections',
            align: 'right',
            render: (e) => <span className="tabular-nums">{e.connections}</span>,
        },
        {
            key: 'bytes',
            header: 'Total Bytes',
            align: 'right',
            render: (e) => (
                <span className="font-medium text-[var(--text)] tabular-nums">
                    {formatBytes(e.bytes_total)}
                </span>
            ),
        },
        {
            key: 'seen',
            header: 'Last Seen',
            align: 'right',
            render: (e) => (
                <span className="text-[var(--text-muted)] tabular-nums">
                    {formatDateTime(e.last_seen_at)}
                </span>
            ),
        },
    ];

    return (
        <div className="space-y-4">
            <LinearCard level={1} className="p-5 flex flex-wrap items-center justify-between gap-4">
                <div className="flex items-center gap-3 min-w-0">
                    <LinearButton
                        variant="ghost"
                        size="sm"
                        onClick={() => setActiveScreen('apps')}
                        aria-label="Back to apps"
                    >
                        <ArrowLeft className="size-4" aria-hidden="true" />
                        <span>Apps</span>
                    </LinearButton>

                    <span className="size-9 rounded bg-[var(--tint-blue)] text-[var(--tint-blue-fg)] font-semibold flex items-center justify-center shrink-0">
                        {detail.name.charAt(0).toUpperCase()}
                    </span>

                    <div className="min-w-0">
                        <div className="flex items-center gap-2">
                            <h1 className="text-base font-semibold text-[var(--text)] truncate">
                                {detail.name}
                            </h1>
                            <span className="px-1.5 py-0.5 rounded bg-[var(--bg-surface-2)] border border-[var(--border)] text-[11px] text-[var(--text-muted)]">
                                {detail.attribution_confidence}
                            </span>
                        </div>
                        <p className="text-xs font-mono text-[var(--text-muted)] truncate">
                            {detail.exe_path}
                        </p>
                    </div>
                </div>

                <div className="text-right">
                    <span className="text-xs uppercase tracking-wider text-[var(--text-muted)]">
                        Range ({rangePreset})
                    </span>
                    <div className="text-xl font-semibold text-[var(--text)] tabular-nums">
                        {formatBytes(detail.usage.range.down + detail.usage.range.up)}
                    </div>
                </div>
            </LinearCard>

            {/* Tabs */}
            <div className="flex items-center gap-4 border-b border-[var(--border)]" role="tablist">
                {(
                    [
                        { id: 'overview', label: 'Overview' },
                        { id: 'connections', label: 'Connections' },
                        { id: 'endpoints', label: 'Endpoints' },
                    ] as const
                ).map((item) => (
                    <button
                        key={item.id}
                        type="button"
                        role="tab"
                        aria-selected={tab === item.id}
                        onClick={() => setTab(item.id)}
                        className={`pb-2.5 text-[13px] font-medium border-b-2 transition-colors duration-150 cursor-pointer ${
                            tab === item.id
                                ? 'border-[var(--accent-blue)] text-[var(--text)]'
                                : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text)]'
                        }`}
                    >
                        {item.label}
                    </button>
                ))}
            </div>

            {tab === 'overview' && (
                <div className="space-y-4">
                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                        <StatCard
                            title="First seen"
                            value={formatDateTime(detail.first_seen_at)}
                            tone="teal"
                        />
                        <StatCard
                            title="Last active"
                            value={formatDateTime(detail.last_seen_at)}
                            tone="sky"
                        />
                        <StatCard
                            title="Month total"
                            value={formatBytes(detail.usage.month.down + detail.usage.month.up)}
                            tone="blue"
                        />
                    </div>

                    <LinearCard level={1} className="p-5">
                        <h2 className="text-sm font-semibold text-[var(--text)] mb-3">
                            Application Usage Timeline
                        </h2>
                        <TrafficChart
                            data={appTimeline.map((pt) => ({
                                label: formatShortTime(pt.bucket_start),
                                down: pt.down,
                                up: pt.up,
                            }))}
                            mode="line"
                            unit="bytes"
                            className="h-60 w-full"
                            ariaLabel={`Usage timeline for ${detail.name}`}
                        />
                    </LinearCard>
                </div>
            )}

            {tab === 'connections' && (
                <DataTable
                    columns={connColumns}
                    rows={connections}
                    rowKey={(r, i) => String(r.id ?? i)}
                    emptyTitle="No recorded connections"
                    emptyDescription="Recorded episodes for this application will appear here."
                    caption={`Connections for ${detail.name}`}
                />
            )}

            {tab === 'endpoints' && (
                <DataTable
                    columns={endpointColumns}
                    rows={endpoints}
                    rowKey={(e) => `${e.remote_ip}:${String(e.port)}`}
                    emptyTitle="No endpoints yet"
                    emptyDescription="Remote endpoints contacted by this app will be listed here."
                    caption={`Endpoints for ${detail.name}`}
                />
            )}
        </div>
    );
};
