import React, { useMemo, useState } from 'react';
import { Search } from 'lucide-react';
import {
    DataTable,
    LinearCard,
    ProgressRow,
    RangeTabs,
    LINEAR_COLORS,
    LINEAR_SERIES_ORDER,
    type DataTableColumn,
} from '$lib/components/linear';
import { useDaemonStore, type AppUsageRow, type RangePreset } from '$lib/stores/useDaemonStore';
import { formatBytes, formatRateText } from '$lib/utils/format';

const RANGE_OPTIONS: Array<{ id: RangePreset; label: string }> = [
    { id: '1h', label: '1h' },
    { id: '24h', label: '24h' },
    { id: '7d', label: '7d' },
    { id: '30d', label: '30d' },
    { id: 'cycle', label: 'Cycle' },
];

const SORT_OPTIONS = [
    { id: 'total', label: 'Total' },
    { id: 'speed', label: 'Speed' },
    { id: 'name', label: 'Name' },
];

function MiniRowSpark({ seed }: { seed: number }) {
    const heights = [30, 55, 40, 80, 65, 90, 50].map((v, i) => ((v + seed * (i + 3)) % 85) + 15);
    const pts = heights
        .map((y, idx) => `${idx * 12},${Math.round(20 - (y / 100) * 18)}`)
        .join(' L ');
    return (
        <svg viewBox="0 0 72 20" className="w-16 h-5 inline-block" aria-hidden="true">
            <path
                d={`M ${pts}`}
                fill="none"
                stroke="var(--accent-green)"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
            />
        </svg>
    );
}

export const Apps: React.FC = () => {
    const { appsList, liveApps, summary, rangePreset, speedUnit, setRangePreset, openAppDetail } =
        useDaemonStore();

    const [search, setSearch] = useState('');
    const [sortBy, setSortBy] = useState<'total' | 'speed' | 'name'>('total');

    const liveRateFor = (name: string) => {
        const match = liveApps.find((r) => r.name.toLowerCase() === name.toLowerCase());
        return match ? match.down_bps + match.up_bps : 0;
    };

    const filtered = useMemo(() => {
        const q = search.trim().toLowerCase();
        const list = appsList.filter((row) => row.name.toLowerCase().includes(q));
        if (sortBy === 'name') {
            return [...list].sort((a, b) => a.name.localeCompare(b.name));
        }
        if (sortBy === 'speed') {
            return [...list].sort((a, b) => liveRateFor(b.name) - liveRateFor(a.name));
        }
        return [...list].sort(
            (a, b) =>
                b.usage.range.down + b.usage.range.up - (a.usage.range.down + a.usage.range.up),
        );
    }, [appsList, search, sortBy, liveApps]);

    const unattributedBytes = summary?.range.totals.unattributed ?? 0;
    const appsRangeSum = filtered.reduce(
        (sum, r) => sum + r.usage.range.down + r.usage.range.up,
        0,
    );
    const totalRangeBytes = Math.max(
        summary?.range.totals.total ?? 0,
        appsRangeSum + unattributedBytes,
    );

    const columns: DataTableColumn<AppUsageRow>[] = [
        {
            key: 'app',
            header: 'Application',
            render: (row, idx) => {
                const initial = (row.name || '?').charAt(0).toUpperCase();
                return (
                    <div className="flex items-center gap-2.5">
                        <span
                            className="size-7 rounded bg-[var(--tint-blue)] text-[var(--tint-blue-fg)] text-xs font-semibold flex items-center justify-center shrink-0"
                            aria-hidden="true"
                        >
                            {initial}
                        </span>
                        <div className="flex flex-col">
                            <span className="font-medium text-[var(--text)]">
                                {row.name === 'unknown' ? 'Unknown process' : row.name}
                            </span>
                            {row.attribution_confidence !== 'exact' && (
                                <span className="text-[11px] text-[var(--text-muted)]">
                                    {row.attribution_confidence} attribution
                                </span>
                            )}
                        </div>
                        <span className="sr-only">{idx}</span>
                    </div>
                );
            },
        },
        {
            key: 'down',
            header: 'Received',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--tint-blue-fg)] font-medium tabular-nums">
                    {formatBytes(row.usage.range.down)}
                </span>
            ),
        },
        {
            key: 'up',
            header: 'Sent',
            align: 'right',
            render: (row) => (
                <span className="text-[var(--tint-sky-fg)] font-medium tabular-nums">
                    {formatBytes(row.usage.range.up)}
                </span>
            ),
        },
        {
            key: 'share',
            header: 'Traffic Share',
            className: 'min-w-[160px]',
            render: (row, idx) => {
                const bytes = row.usage.range.down + row.usage.range.up;
                const pct =
                    totalRangeBytes > 0
                        ? Math.min(100, Math.max(0, (bytes / totalRangeBytes) * 100))
                        : 0;
                const color =
                    LINEAR_SERIES_ORDER[idx % LINEAR_SERIES_ORDER.length] ?? LINEAR_COLORS.blue;
                return (
                    <ProgressRow
                        label=""
                        value={pct}
                        displayValue={`${Math.round(pct)}%`}
                        colorVar={color}
                    />
                );
            },
        },
        {
            key: 'live',
            header: 'Live Speed',
            align: 'right',
            render: (row) => {
                const bps = liveRateFor(row.name);
                return (
                    <span className="text-[var(--text-muted)] tabular-nums">
                        {bps > 0 ? formatRateText(bps, speedUnit) : '—'}
                    </span>
                );
            },
        },
        {
            key: 'trend',
            header: 'Trend',
            align: 'right',
            render: (row) => <MiniRowSpark seed={row.id} />,
        },
    ];

    return (
        <div className="space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <h1 className="text-base font-semibold text-[var(--text)]">Applications</h1>
                    <p className="text-xs text-[var(--text-muted)]">
                        Per-application bandwidth usage and share across selected period
                    </p>
                </div>

                <RangeTabs
                    options={RANGE_OPTIONS}
                    active={rangePreset}
                    onChange={(id) => setRangePreset(id as RangePreset)}
                    ariaLabel="Applications usage range"
                />
            </div>

            <LinearCard level={1} className="p-3 flex flex-wrap items-center justify-between gap-3">
                <div className="relative flex-1 min-w-[220px] max-w-xs">
                    <Search
                        className="absolute left-2.5 top-2 size-4 text-[var(--text-muted)]"
                        aria-hidden="true"
                    />
                    <input
                        type="text"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        placeholder="Search apps..."
                        aria-label="Search applications"
                        className="w-full h-8 pl-8 pr-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--accent-blue)]"
                    />
                </div>

                <RangeTabs
                    options={SORT_OPTIONS}
                    active={sortBy}
                    onChange={(id) => setSortBy(id as typeof sortBy)}
                    ariaLabel="Sort applications"
                />
            </LinearCard>

            <DataTable
                columns={columns}
                rows={filtered}
                rowKey={(row) => String(row.id)}
                onRowClick={(row) => openAppDetail(row.id)}
                emptyTitle="No attributed applications yet"
                emptyDescription="Application traffic appears here after monitoring begins."
                caption="Application usage ranking"
                footerRow={
                    <tr className="bg-[var(--bg-surface-2)] text-[var(--text-muted)]">
                        <td className="py-2.5 px-4 font-medium">Unattributed / system</td>
                        <td className="py-2.5 px-4 text-right tabular-nums">—</td>
                        <td className="py-2.5 px-4 text-right tabular-nums">—</td>
                        <td className="py-2.5 px-4 tabular-nums">
                            {totalRangeBytes > 0
                                ? `${Math.round((unattributedBytes / totalRangeBytes) * 100)}%`
                                : '0%'}
                        </td>
                        <td className="py-2.5 px-4 text-right tabular-nums">
                            {formatBytes(unattributedBytes)}
                        </td>
                        <td className="py-2.5 px-4" />
                    </tr>
                }
            />
        </div>
    );
};
