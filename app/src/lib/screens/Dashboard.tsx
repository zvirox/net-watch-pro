import React from 'react';
import { Activity, ArrowDown, ArrowUp, Database, HardDrive, Info, Wifi } from 'lucide-react';
import {
    LinearCard,
    ProgressRow,
    RangeTabs,
    StatCard,
    LINEAR_SERIES_TOKENS,
} from '$lib/components/linear';
import { AppUsageChart, TrafficChart } from '$lib/components/charts';
import { budgetsStore } from '$lib/stores/budgets';
import { useDaemonStore, type RangePreset } from '$lib/stores/useDaemonStore';
import { formatBytes, formatRateParts, formatShortTime } from '$lib/utils/format';

const RANGE_OPTIONS: Array<{ id: RangePreset; label: string }> = [
    { id: '1h', label: '1h' },
    { id: '24h', label: '24h' },
    { id: '7d', label: '7d' },
    { id: '30d', label: '30d' },
    { id: 'cycle', label: 'Cycle' },
];

function splitBytesValue(bytes: number): { value: string; unit: string } {
    const formatted = formatBytes(bytes);
    if (formatted === '—') return { value: '—', unit: '' };
    const parts = formatted.split(' ');
    return { value: parts[0] ?? '0', unit: parts[1] ?? 'B' };
}

function budgetColorVar(level: string): string {
    if (level === 'breach' || level === 'critical') return 'var(--red)';
    if (level === 'warning') return 'var(--amber)';
    return 'var(--sky)';
}

export const Dashboard: React.FC = () => {
    const {
        summary,
        timeline,
        interfaces,
        rateHistory,
        latestRates,
        ratesStale,
        liveConnections,
        rangePreset,
        speedUnit,
        ebpf,
        connection,
        demoMode,
        setRangePreset,
        openAppDetail,
    } = useDaemonStore();

    const downSpark = rateHistory.map((p) => p.down_bps);
    const upSpark = rateHistory.map((p) => p.up_bps);

    const liveDown = formatRateParts(latestRates?.down_bps ?? 0, speedUnit);
    const liveUp = formatRateParts(latestRates?.up_bps ?? 0, speedUnit);

    const rangeTotals = summary?.range.totals ?? { total: 0, attributed: 0, unattributed: 0 };
    const totalSplit = splitBytesValue(rangeTotals.total);
    const attribPct =
        rangeTotals.total > 0
            ? Math.min(100, Math.round((rangeTotals.attributed / rangeTotals.total) * 100))
            : 0;

    const chartPoints = timeline.slice(-24).map((pt) => ({
        label: formatShortTime(pt.bucket_start),
        down: pt.down,
        up: pt.up,
    }));

    const topApps = summary?.top_apps ?? [];
    const topAppsMax = topApps.reduce((m, a) => Math.max(m, a.bytes_total), 0);

    const donutSlices = [
        ...topApps.slice(0, 6).map((a) => ({ name: a.name, bytes: a.bytes_total })),
        ...(rangeTotals.unattributed > 0
            ? [{ name: 'Unattributed', bytes: rangeTotals.unattributed }]
            : []),
    ];

    const collectorMissing =
        !demoMode && connection === 'connected' && (ebpf === 'no_collector' || ebpf === 'failed');

    const cycleDaysLeft = summary?.cycle.days_left ?? 0;
    const dailyBudget = summary?.budgets.total_daily ?? {
        used_bytes: 0,
        limit_bytes: 0,
        level: 'ok' as const,
    };
    const monthlyBudget = summary?.budgets.total_monthly ?? {
        used_bytes: summary?.cycle.used_bytes ?? 0,
        limit_bytes: 0,
        level: 'ok' as const,
    };

    return (
        <div className="space-y-4">
            {/* Top toolbar row with Range selector */}
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <h1 className="text-base font-semibold text-[var(--text)]">Overview</h1>
                    <p className="text-xs text-[var(--text-muted)]">
                        {ratesStale
                            ? 'Live rate feed stale (waiting for daemon frame)'
                            : 'Live throughput and per-application accounting'}
                    </p>
                </div>
                <RangeTabs
                    options={RANGE_OPTIONS}
                    active={rangePreset}
                    onChange={(id) => setRangePreset(id as RangePreset)}
                    ariaLabel="Dashboard usage range"
                />
            </div>

            {collectorMissing && (
                <LinearCard
                    level={1}
                    className="px-4 py-3 border-[var(--amber)] bg-[var(--bg-surface-2)] flex items-center gap-2.5 text-xs text-[var(--text)]"
                    role="status"
                >
                    <Info className="size-4 text-[var(--amber)] shrink-0" aria-hidden="true" />
                    <span>
                        eBPF collector is not attached ({ebpf}). Whole-PC speed and interface totals
                        continue via sysfs; per-app attribution is paused.
                    </span>
                </LinearCard>
            )}

            {/* 4 Stat Cards */}
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
                <StatCard
                    title="Download rate"
                    value={liveDown.value}
                    unit={liveDown.unit}
                    tone="sky"
                    icon={<ArrowDown className="size-4" />}
                    sparkline={downSpark.length >= 2 ? downSpark : [0, 0]}
                    sparklineDirection="up"
                    delta={{
                        value:
                            splitBytesValue(summary?.today.total ?? 0).value +
                            ' ' +
                            splitBytesValue(summary?.today.total ?? 0).unit,
                        positive: true,
                        comparison: 'today total',
                    }}
                />

                <StatCard
                    title="Upload rate"
                    value={liveUp.value}
                    unit={liveUp.unit}
                    tone="orange"
                    icon={<ArrowUp className="size-4" />}
                    sparkline={upSpark.length >= 2 ? upSpark : [0, 0]}
                    sparklineDirection="up"
                    delta={{
                        value: `${attribPct}%`,
                        positive: attribPct >= 80,
                        comparison: 'attributed in range',
                    }}
                />

                <StatCard
                    title="Active connections"
                    value={liveConnections.length}
                    unit="sockets"
                    tone="blue"
                    icon={<Activity className="size-4" />}
                    delta={{
                        value: `${topApps.length} apps`,
                        positive: true,
                        comparison: 'tracked in window',
                    }}
                />

                <StatCard
                    title={`Used (${rangePreset})`}
                    value={totalSplit.value}
                    unit={totalSplit.unit}
                    tone={attribPct >= 90 ? 'sky' : 'amber'}
                    icon={<Database className="size-4" />}
                    delta={{
                        value: formatBytes(rangeTotals.unattributed),
                        positive: rangeTotals.unattributed <= rangeTotals.total * 0.15,
                        comparison: 'unattributed gap',
                    }}
                />
            </div>

            {/* Main charts: Throughput Timeline + App Usage Split Donut */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
                <LinearCard level={1} className="p-5 lg:col-span-2 flex flex-col justify-between">
                    <div className="flex flex-wrap items-center justify-between gap-2 mb-4">
                        <div>
                            <h2 className="text-sm font-semibold text-[var(--text)]">
                                Throughput &amp; Usage
                            </h2>
                            <p className="text-xs text-[var(--text-muted)]">
                                Download vs upload over selected window ({rangePreset})
                            </p>
                        </div>
                        <div className="flex items-center gap-4 text-xs text-[var(--text-muted)]">
                            <span className="inline-flex items-center gap-1.5">
                                <span className="size-2 rounded-full bg-[var(--primary)]" />
                                Download
                            </span>
                            <span className="inline-flex items-center gap-1.5">
                                <span className="size-2 rounded-full bg-[var(--orange)]" />
                                Upload
                            </span>
                        </div>
                    </div>

                    <TrafficChart
                        data={chartPoints}
                        mode="line"
                        unit="bytes"
                        className="h-64 w-full"
                        ariaLabel="Network throughput and usage timeline"
                    />
                </LinearCard>

                <LinearCard level={1} className="p-5 flex flex-col justify-between">
                    <div className="mb-3">
                        <h2 className="text-sm font-semibold text-[var(--text)]">
                            App Usage Split
                        </h2>
                        <p className="text-xs text-[var(--text-muted)]">
                            Share of attributed bytes
                        </p>
                    </div>

                    {donutSlices.length > 0 ? (
                        <AppUsageChart slices={donutSlices} totalBytes={rangeTotals.total} />
                    ) : (
                        <div className="py-12 text-center text-xs text-[var(--text-muted)]">
                            Waiting for attributed app usage samples.
                        </div>
                    )}
                </LinearCard>
            </div>

            {/* Bottom row: Top Applications + Interfaces & Cycle Budgets */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <LinearCard level={1} className="p-5">
                    <div className="flex items-center justify-between mb-4">
                        <div>
                            <h2 className="text-sm font-semibold text-[var(--text)]">
                                Top Applications
                            </h2>
                            <p className="text-xs text-[var(--text-muted)]">
                                Click any application to inspect connections and endpoints
                            </p>
                        </div>
                        <span className="text-xs text-[var(--text-muted)] tabular-nums">
                            {formatBytes(rangeTotals.attributed)} attributed
                        </span>
                    </div>

                    <div className="space-y-3">
                        {topApps.length === 0 ? (
                            <p className="text-xs text-[var(--text-muted)] py-4">
                                No attributed apps yet.
                            </p>
                        ) : (
                            topApps.map((app, idx) => {
                                const pct =
                                    topAppsMax > 0 ? (app.bytes_total / topAppsMax) * 100 : 0;
                                const colorToken =
                                    LINEAR_SERIES_TOKENS[idx % LINEAR_SERIES_TOKENS.length] ??
                                    '--primary';
                                return (
                                    <button
                                        key={app.id}
                                        type="button"
                                        onClick={() => openAppDetail(app.id)}
                                        className="w-full text-left rounded-md p-1.5 -mx-1.5 hover:bg-[var(--bg-hover)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--primary-text)] cursor-pointer"
                                    >
                                        <ProgressRow
                                            label={app.name}
                                            value={pct}
                                            displayValue={formatBytes(app.bytes_total)}
                                            colorVar={`var(${colorToken})`}
                                        />
                                    </button>
                                );
                            })
                        )}

                        <div className="pt-2 border-t border-[var(--border)] flex items-center justify-between text-xs text-[var(--text-muted)]">
                            <span title="Difference between kernel interface counters and attributed per-app eBPF counters (headers, ACKs, kernel traffic)">
                                Unattributed / system gap
                            </span>
                            <span className="tabular-nums font-medium text-[var(--text)]">
                                {formatBytes(rangeTotals.unattributed)}
                            </span>
                        </div>
                    </div>
                </LinearCard>

                <div className="space-y-4">
                    {/* Interfaces Card */}
                    <LinearCard level={1} className="p-5">
                        <div className="flex items-center justify-between mb-3">
                            <h2 className="text-sm font-semibold text-[var(--text)]">Interfaces</h2>
                            <span className="text-xs text-[var(--text-muted)]">
                                Loopback excluded
                            </span>
                        </div>

                        {interfaces.length === 0 ? (
                            <p className="text-xs text-[var(--text-muted)] py-2">
                                No active interfaces sampled.
                            </p>
                        ) : (
                            <div className="divide-y divide-[var(--border)]">
                                {interfaces.map((iface) => {
                                    const rx = iface.bytes?.down ?? iface.bytes_rx ?? 0;
                                    const tx = iface.bytes?.up ?? iface.bytes_tx ?? 0;
                                    return (
                                        <div
                                            key={iface.name}
                                            className="py-2.5 flex items-center justify-between gap-3 text-xs"
                                        >
                                            <div className="flex items-center gap-2 min-w-0">
                                                <span className="size-6 rounded bg-[var(--bg-surface-2)] border border-[var(--border)] flex items-center justify-center shrink-0">
                                                    {iface.kind === 'wifi' ? (
                                                        <Wifi className="size-3.5 text-[var(--sky)]" />
                                                    ) : (
                                                        <HardDrive className="size-3.5 text-[var(--text-muted)]" />
                                                    )}
                                                </span>
                                                <div className="min-w-0">
                                                    <div className="flex items-center gap-1.5">
                                                        <span className="font-medium text-[var(--text)]">
                                                            {iface.name}
                                                        </span>
                                                        {iface.is_metered && (
                                                            <span className="px-1.5 py-0.5 rounded bg-[var(--bg-surface-2)] text-[var(--amber)] border border-[var(--border)] text-[11px]">
                                                                Metered
                                                            </span>
                                                        )}
                                                    </div>
                                                    {iface.kind === 'wifi' && (
                                                        <div className="text-[11px] text-[var(--text-muted)] tabular-nums">
                                                            Signal:{' '}
                                                            {iface.signal_dbm !== null &&
                                                            iface.signal_dbm !== undefined
                                                                ? `${iface.signal_dbm} dBm`
                                                                : '—'}{' '}
                                                            · Link:{' '}
                                                            {iface.link_mbps !== null &&
                                                            iface.link_mbps !== undefined
                                                                ? `${iface.link_mbps} Mbps`
                                                                : '—'}
                                                        </div>
                                                    )}
                                                </div>
                                            </div>
                                            <div className="text-right tabular-nums shrink-0">
                                                <div className="text-[var(--text)] font-medium">
                                                    ↓ {formatBytes(rx)}
                                                </div>
                                                <div className="text-[11px] text-[var(--text-muted)]">
                                                    ↑ {formatBytes(tx)}
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        )}
                    </LinearCard>

                    {/* Budgets & Billing Cycle Card */}
                    <LinearCard level={1} className="p-5 space-y-3">
                        <div className="flex items-center justify-between">
                            <h2 className="text-sm font-semibold text-[var(--text)]">
                                Billing Cycle &amp; Budgets
                            </h2>
                            <span className="text-xs text-[var(--text-muted)] tabular-nums">
                                {cycleDaysLeft} days left
                            </span>
                        </div>

                        <ProgressRow
                            label="Daily budget"
                            value={
                                dailyBudget.limit_bytes > 0 ? budgetsStore.percent(dailyBudget) : 0
                            }
                            displayValue={
                                dailyBudget.limit_bytes > 0
                                    ? `${formatBytes(dailyBudget.used_bytes)} / ${formatBytes(dailyBudget.limit_bytes)}`
                                    : `${formatBytes(dailyBudget.used_bytes)} (no cap)`
                            }
                            colorVar={budgetColorVar(dailyBudget.level)}
                        />

                        <ProgressRow
                            label="Cycle budget"
                            value={
                                monthlyBudget.limit_bytes > 0
                                    ? budgetsStore.percent(monthlyBudget)
                                    : 0
                            }
                            displayValue={
                                monthlyBudget.limit_bytes > 0
                                    ? `${formatBytes(monthlyBudget.used_bytes)} / ${formatBytes(monthlyBudget.limit_bytes)}`
                                    : `${formatBytes(monthlyBudget.used_bytes)} used`
                            }
                            colorVar={budgetColorVar(monthlyBudget.level)}
                        />
                    </LinearCard>
                </div>
            </div>
        </div>
    );
};
